use std::any::TypeId;
use std::mem;
use std::sync::*;

use ::desync::*;
use futures::future;
use futures::prelude::*;
use futures::stream::BoxStream;
use futures::task::Poll;

use crate::entity_channel::*;
use crate::entity_id::*;
use crate::error::*;
use crate::scene::scene_core::*;
use crate::simple_entity_channel::*;
use crate::standard_components::*;

thread_local! {
    static CURRENT_CONTEXT: RefCell<Option<Arc<SceneContext>>> = RefCell::new(None);
}

///
/// Used to restore the context after a `with_context` call returns
///
struct DropContext {
    previous_context: Option<Arc<SceneContext>>,
}

///
/// The scene context allows for communication with a scene, either while it's being set up or while it's running.
///
/// Retrieve the context for the currently running thread (set automatically by the running scene):
///
/// ```
/// # use flo_scene::*;
/// let context = SceneContext::current();
/// ```
///
/// Send messages to an entity within a context:
///
/// ```
/// # use flo_scene::*;
/// # let scene = Scene::default();
/// # let context = scene.context();
/// let mut channel = context.send_to::<ExampleRequest>(EXAMPLE).unwrap();
/// // channel.send_immediate(ExampleRequest::Example); (TODO: synchronous messages)
/// ```
///
/// Create a new entity to run in the scene:
///
/// ```
/// # use flo_scene::*;
/// # use futures::prelude::*;
/// # let scene = Scene::empty();
/// # let context = scene.context();
/// context.create_entity(EXAMPLE, move |_context, mut requests| {
///     async move {
///         while let Some(request) = requests.next().await {
///             match request {
///                 ExampleRequest::Example => { println!("Example!"); }
///             }
///         }
///     }
/// }).unwrap();
/// ```
///
/// Add automatic conversions between entity message types:
///
/// ```
/// # use flo_scene::*;
/// # let scene = Scene::default();
/// # let context = scene.context();
/// enum MyRequest { SomeRequest };
///
/// assert!(context.send_to::<MyRequest>(EXAMPLE).is_err());
///
/// impl Into<ExampleRequest> for MyRequest {
///     fn into(self) -> ExampleRequest { ExampleRequest::Example }
/// }
///
/// context.convert_message::<MyRequest, ExampleRequest>().unwrap();
/// let channel = context.send_to::<MyRequest>(EXAMPLE).unwrap();
/// ``` 
///
/// Run a background process in an entity:
///
/// ```
/// # use flo_scene::*;
/// # use futures::prelude::*;
/// # use futures::future;
/// # let scene = Scene::empty();
/// # let scene_context = scene.context();
/// scene_context.create_entity(EXAMPLE, move |entity_context, mut requests| {
///     async move {
///         entity_context.run_in_background(async {
///             future::pending::<()>().await;
///         }).unwrap();
///
///         while let Some(request) = requests.next().await {
///             match request {
///                 ExampleRequest::Example => { println!("Example!"); }
///             }
///         }
///     }
/// }).unwrap();
/// ```
///
/// Shut down a running entity:
///
/// ```
/// # use flo_scene::*;
/// # let scene = Scene::default();
/// # let context = scene.context();
/// context.close_entity(EXAMPLE).unwrap();
/// ```
///
/// Replace the context for the current thread with another context:
///
/// ```
/// # use flo_scene::*;
/// # let scene = Scene::default();
/// # let new_context = scene.context();
/// SceneContext::with_context(&new_context, || {
///     // SceneContext::current() == new_context   
/// }).unwrap();
/// ```
///
/// Replace the context for a future with another context:
///
/// ```
/// # use flo_scene::*;
/// # use futures::executor;
/// # let scene = Scene::default();
/// # let new_context = scene.context();
/// # executor::block_on(async move {
/// SceneContext::with_context_async(&new_context, async {
///     SceneContext::current().send_to(EXAMPLE).unwrap()
///         .send(ExampleRequest::Example)
///         .await
///         .unwrap();
/// }).await.unwrap();
/// # });
/// ```
///
pub struct SceneContext {
    /// The entity that's executing code on the current thread, or none for things like default actions
    entity: Option<EntityId>,

    /// The core of the scene that the entity is a part of
    scene_core: Result<Weak<Desync<SceneCore>>, SceneContextError>,
}

impl SceneContext {
    ///
    /// Retrieves the active scene context (or a context error if one is available)
    ///
    pub fn current() -> Arc<SceneContext> {
        let context = CURRENT_CONTEXT
            .try_with(|ctxt| ctxt.borrow().clone());

        match context {
            Ok(Some(context)) => context,
            Ok(None) => Arc::new(SceneContext::none()),
            Err(err) => Arc::new(SceneContext::error(err.into())),
        }
    }

    ///
    /// Creates a scene context that means 'no context'
    ///
    fn none() -> SceneContext {
        Self::error(SceneContextError::NoCurrentScene)
    }

    ///
    /// Creates an error scene context
    ///
    fn error(error: SceneContextError) -> SceneContext {
        SceneContext {
            entity: None,
            scene_core: Err(error),
        }
    }

    ///
    /// Returns a context with no active entity 
    ///
    pub(crate) fn with_no_entity(core: &Arc<Desync<SceneCore>>) -> SceneContext {
        SceneContext {
            entity: None,
            scene_core: Ok(Arc::downgrade(core)),
        }
    }

    ///
    /// Fetches a reference to the scene core
    ///
    #[inline]
    fn scene_core(&self) -> Result<Arc<Desync<SceneCore>>, SceneContextError> {
        match &self.scene_core {
            Ok(core) => {
                if let Some(core) = core.upgrade() {
                    Ok(core)
                } else {
                    Err(SceneContextError::SceneFinished)
                }
            }

            Err(err) => {
                Err(*err)
            }
        }
    }

    ///
    /// Evaluates a function within a particular scene context
    ///
    /// This is typically done automatically when running the runtimes for entities, but this can be used if if's ever necessary to
    /// artificially change contexts (eg: if an entity spawns its own thread, or in an independent runtime)
    ///
    #[inline]
    pub fn with_context<TFn, TResult>(new_context: &Arc<SceneContext>, in_context: TFn) -> Result<TResult, SceneContextError>
        where
            TFn: FnOnce() -> TResult
    {
        let new_context = Arc::clone(new_context);

        // When the function returns, reset the context
        let last_context = DropContext {
            previous_context: CURRENT_CONTEXT.try_with(|ctxt| ctxt.borrow().clone())?,
        };

        // Set the updated context
        CURRENT_CONTEXT.try_with(move |ctxt| *(ctxt.borrow_mut()) = Some(new_context))?;

        // Call the function with the new context
        let result = in_context();

        // Restore the context
        mem::drop(last_context);

        Ok(result)
    }

    ///
    /// Evaluates a future within a particular scene context
    ///
    /// This is typically done automatically when running the runtimes for entities, but this can be used if if's ever necessary to
    /// artificially change contexts (eg: if an entity spawns its own thread, or in an independent runtime)
    ///
    #[inline]
    pub fn with_context_async<'a, TFuture>(new_context: &Arc<SceneContext>, future: TFuture) -> impl 'a + Send + Future<Output=Result<TFuture::Output, SceneContextError>>
        where
            TFuture: 'a + Future + Send
    {
        let new_context = Arc::clone(new_context);
        let mut future = future.boxed();

        async move {
            let result = future::poll_fn(move |context| {
                // When the function returns, reset the context
                let previous_context = match CURRENT_CONTEXT.try_with(|ctxt| ctxt.borrow().clone()) {
                    Ok(context) => context,
                    Err(err) => { return Poll::Ready(Err(err.into())); }
                };
                let last_context = DropContext { previous_context };

                // Set the updated context
                match CURRENT_CONTEXT.try_with(|ctxt| { *(ctxt.borrow_mut()) = Some(Arc::clone(&new_context)); }) {
                    Ok(()) => {}
                    Err(err) => { return Poll::Ready(Err(err.into())); }
                }

                // Call the function with the new context
                let poll_result = future.poll_unpin(context);

                // Restore the context
                mem::drop(last_context);

                match poll_result {
                    Poll::Ready(val) => Poll::Ready(Ok(val)),
                    Poll::Pending => Poll::Pending
                }
            }).await;

            result
        }
    }

    ///
    /// Returns the entuty that this context is for
    ///
    pub fn entity(&self) -> Option<EntityId> {
        self.entity
    }

    ///
    /// Specify that entities that can process messages of type `TNewMessage` can also process messages of type `TOriginalMessage`
    ///
    /// That is, if an entity can be addressed using `EntityChannel<Message=TNewMessage>` it will automatically convert from `TOriginalMessage`
    /// so that `EntityChannel<Message=TOriginalMessage>` also works.
    ///
    pub fn convert_message<TOriginalMessage, TNewMessage>(&self) -> Result<(), SceneContextError>
        where
            TOriginalMessage: 'static + Send + Into<TNewMessage>,
            TNewMessage: 'static + Send,
    {
        self.scene_core()?.sync(move |core| {
            // Register that one type can be converted to another
            core.convert_message::<TOriginalMessage, TNewMessage>();

            // Send to the entity registry
            if let Ok(channel) = core.send_to::<InternalRegistryRequest>(ENTITY_REGISTRY) {
                core.send_background_message(channel, InternalRegistryRequest::ConvertMessage(TypeId::of::<TOriginalMessage>(), TypeId::of::<TNewMessage>()));
            }
        });

        Ok(())
    }

    ///
    /// Creates a channel to send messages in this context
    ///
    pub fn send_to<TMessage>(&self, entity_id: EntityId) -> Result<BoxedEntityChannel<'static, TMessage>, EntityChannelError>
        where
            TMessage: 'static + Send,
    {
        self.scene_core()?.sync(|core| {
            core.send_to(entity_id)
        })
    }

    ///
    /// Sends a message without waiting for the channel to finish processing it
    ///
    pub fn send<TMessage>(&self, entity_id: EntityId, message: TMessage) -> impl 'static + Future<Output=Result<(), EntityChannelError>>
        where
            TMessage: 'static + Send,
    {
        let channel = self.send_to::<TMessage>(entity_id);
        async move {
            let mut channel = channel?;
            channel.send(message).await
        }
    }

    ///
    /// Sends a stream of data to an entity
    ///
    /// This will use the `<TMessage, ()>` interface of the entity to send the data
    ///
    pub fn send_stream<TMessage>(&self, entity_id: EntityId, stream: impl 'static + Send + Stream<Item=TMessage>) -> Result<impl Send + Future<Output=()>, EntityChannelError>
        where
            TMessage: 'static + Send,
    {
        // Connect to the entity
        let mut channel = self.send_to::<TMessage>(entity_id)?;
        let mut stream = stream.boxed();

        Ok(async move {
            // Future reads from the stream until it's done
            while let Some(message) = stream.next().await {
                // Send to the channel
                let response = channel.send(message).await;

                // Break if the channel responds with an error
                if response.is_err() {
                    break;
                }
            }
        })
    }

    ///
    /// Creates an entity that processes a particular kind of message
    ///
    pub fn create_entity<TMessage, TFn, TFnFuture>(&self, entity_id: EntityId, runtime: TFn) -> Result<SimpleEntityChannel<TMessage>, CreateEntityError>
        where
            TMessage: 'static + Send,
            TFn: 'static + Send + FnOnce(Arc<SceneContext>, BoxStream<'static, TMessage>) -> TFnFuture,
            TFnFuture: 'static + Send + Future<Output=()>,
    {
        // Create a SceneContext for the new entity
        let new_context = Arc::new(SceneContext {
            entity: Some(entity_id),
            scene_core: Ok(Arc::downgrade(&self.scene_core()?)),
        });

        // Request that the core create the entity
        self.scene_core()?.sync(move |core| {
            core.create_entity(new_context, runtime)
        })
    }

    ///
    /// Returns a future that indicates when the entity has started up (is awaiting its first message)
    ///
    /// Normally this isn't necessary but sometimes (for example when retrieving a property), it can be useful to wait for
    /// an entity to finish initializing before requesting information about it.
    ///
    pub fn wait_for_entity_to_start(&self, entity_id: EntityId) -> impl Send + Future<Output=()> {
        let scene_core = self.scene_core();

        async move {
            if let Ok(scene_core) = scene_core {
                scene_core.sync(|core| core.wait_for_entity_to_start(entity_id)).await;
            }
        }
    }

    ///
    /// Called when an entity in this context is awaiting its first message
    ///
    pub(crate) fn entity_has_started(&self, entity_id: EntityId) {
        if let Ok(scene_core) = self.scene_core() {
            scene_core.desync(move |core| core.entity_has_started(entity_id));
        }
    }

    ///
    /// Called when an entity in this context has finished
    ///
    pub(crate) fn finish_entity<TMessage>(&self, entity_id: EntityId)
        where
            TMessage: 'static + Send,
    {
        if let Ok(scene_core) = self.scene_core() {
            scene_core.desync(move |core| core.finish_entity(entity_id));
        }
    }

    ///
    /// Called whenever all of the entities in the scene are waiting for new messages
    ///
    pub(crate) fn send_heartbeat(&self) {
        if let Ok(scene_core) = self.scene_core() {
            scene_core
                .future_desync(move |core| async move {
                    core.send_heartbeat().await;
                }.boxed())
                .detach();
        }
    }

    ///
    /// Adds a future to run in the background of the current entity 
    ///
    /// These background futures will be dropped when the main entity runtime terminates, and are scheduled alongside each other and the main runtime
    /// (ie, all of the main runtime and the background futures will get scheduled on the same thread)
    ///
    pub fn run_in_background(&self, future: impl 'static + Send + Future<Output=()>) -> Result<(), EntityFutureError> {
        let scene_core = self.scene_core()?;

        if let Some(entity_id) = self.entity {
            scene_core.sync(move |core| {
                core.run_in_background(entity_id, future)
            })?;

            Ok(())
        } else {
            Err(EntityFutureError::NoCurrentEntity)
        }
    }

    ///
    /// 'Seals' an entity, which leaves it running but makes it impossible to open new channels to it
    ///
    /// This is useful when an entity is in use but shouldn't be accessible from any new entities added to the
    /// scene.
    ///
    pub fn seal_entity(&self, entity_id: EntityId) -> Result<(), EntityChannelError> {
        self.scene_core()?.sync(|core| core.seal_entity(entity_id))?;

        Ok(())
    }

    ///
    /// Closes the main channel to an entity, preventing it from receiving any further messages, and usually causing it
    /// to exit its main loop and shut down.
    ///
    /// Entities usually stop in response to their main channel closing, but are capable of running beyond this point.
    /// The channel will initially be retrievable but unable to receive new messages, and will only stop existing at the
    /// point the entity fully stops.
    ///
    pub fn close_entity(&self, entity_id: EntityId) -> Result<(), EntityChannelError> {
        self.scene_core()?.sync(|core| core.close_entity(entity_id))?;

        Ok(())
    }

    ///
    /// Drops the running futures for the specified entity, causing it to be immediately and impolitely shut down
    ///
    /// Generally 'close_entity' should be used instead of this, but this will also shut the entity down.
    ///
    pub fn kill_entity(&self, entity_id: EntityId) -> Result<(), EntityChannelError> {
        self.scene_core()?.sync(|core| core.stop_entity(entity_id))?;

        Ok(())
    }

    ///
    /// Stops the running scene
    ///
    pub(crate) fn stop_scene(&self) -> Result<(), EntityChannelError> {
        self.scene_core()?.sync(|core| core.stop_scene());

        Ok(())
    }
}

impl Drop for DropContext {
    fn drop(&mut self) {
        let previous_context = self.previous_context.take();
        CURRENT_CONTEXT.try_with(move |ctxt| *(ctxt.borrow_mut()) = previous_context).ok();
    }
}

///
/// Retrieves the entity ID that the current context is executing for
///
pub fn scene_current_entity() -> Option<EntityId> {
    SceneContext::current().entity()
}

///
/// Runs a future in the background of the current entity
///
/// These background futures will be dropped when the main entity runtime terminates, and are scheduled alongside each other and the main runtime
/// (ie, all of the main runtime and the background futures will get scheduled on the same thread)
///
pub fn scene_run_in_background(future: impl 'static + Send + Future<Output=()>) -> Result<(), EntityFutureError> {
    SceneContext::current().run_in_background(future)
}

///
/// Creates a channel for sending messages to a entity (in the current context)
///
pub fn scene_send_to<TMessage>(entity_id: EntityId) -> Result<BoxedEntityChannel<'static, TMessage>, EntityChannelError>
    where
        TMessage: 'static + Send,
{
    SceneContext::current().send_to(entity_id)
}

///
/// Sends a single message to an entity
///
pub async fn scene_send<TMessage>(entity_id: EntityId, message: TMessage) -> Result<(), EntityChannelError>
    where
        TMessage: 'static + Send,
{
    SceneContext::current().send(entity_id, message).await
}

///
/// Sends a stream of data to an entity
///
/// This will use the `<TMessage, ()>` interface of the entity to send the data
///
pub fn scene_send_stream<TMessage>(entity_id: EntityId, stream: impl 'static + Send + Stream<Item=TMessage>) -> Result<impl Send + Future<Output=()>, EntityChannelError>
    where
        TMessage: 'static + Send,
{
    SceneContext::current().send_stream(entity_id, stream)
}

///
/// Creates a new entity in the current scene
///
pub fn scene_create_entity<TMessage, TFn, TFnFuture>(entity_id: EntityId, runtime: TFn) -> Result<SimpleEntityChannel<TMessage>, CreateEntityError>
    where
        TMessage: 'static + Send,
        TFn: 'static + Send + FnOnce(Arc<SceneContext>, BoxStream<'static, TMessage>) -> TFnFuture,
        TFnFuture: 'static + Send + Future<Output=()>,
{
    SceneContext::current().create_entity(entity_id, runtime)
}
