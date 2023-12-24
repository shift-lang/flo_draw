use std::mem;
use std::sync::*;

use ::desync::*;
use futures::channel::oneshot;
use futures::future;
use futures::prelude::*;
use futures::stream::BoxStream;
use futures::task;
use futures::task::Poll;

use crate::context::*;
use crate::entity_channel::*;
use crate::entity_id::*;
use crate::error::*;
use crate::simple_entity_channel::*;
use crate::standard_components::*;

use super::scene_core::*;
use super::scene_waker::*;

///
/// A `Scene` is a container for a set of entities. It provides a way to connect a set of small components into a larger program 
/// or system. Software can be designed around a single scene containing everything, but multiple and even nested scenes are
/// supported (so software could be designed around scene per user, per document or per session as needed)
///
/// Create the default scene (which is set up to have the standard set of components):
///
/// ```
/// # use flo_scene::*;
/// let scene = Scene::default();
/// ```
///
/// Create an empty scene (with no components, even the entity registry):
///
/// ```
/// # use flo_scene::*;
/// let scene = Scene::empty();
/// ```
///
/// Run all of the components in a scene:
///
/// ```
/// # use flo_scene::*;
/// # let scene = Scene::default();
/// # scene.create_entity::<(), _, _>(EntityId::new(), |context, _| async move {
/// #   context.send_to(SCENE_CONTROL).unwrap().send(SceneControlRequest::StopScene).await.unwrap();
/// # }).unwrap();
/// use futures::executor;
/// executor::block_on(async move { scene.run().await });
/// ```
///
/// Retrieve the base scene context, which can be used for setting up more components and interacting with the scene
/// (though note that the scene is paused until the `run()` function is called):
///
/// ```
/// # use flo_scene::*;
/// # use futures::prelude::*;
/// # let scene = Scene::empty();
/// let context = scene.context();
///
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
pub struct Scene {
    /// The shared state for all entities in this scene
    core: Arc<Desync<SceneCore>>,
}

impl Default for Scene {
    ///
    /// Creates a scene with the default set of 'well-known' entities
    ///
    fn default() -> Scene {
        // Create an empty scene
        let scene = Scene::empty();
        let context = scene.context();

        // Add the standard components
        create_entity_registry_entity(&context).unwrap();
        create_scene_control_entity(SCENE_CONTROL, &context).unwrap();
        create_heartbeat_entity(&context).unwrap();
        create_example_entity(EXAMPLE, &context).unwrap();

        #[cfg(feature = "timer")]
        create_timer_entity(TIMER, &context).unwrap();

        #[cfg(feature = "properties")]
        create_properties_entity(PROPERTIES, &context).unwrap();

        scene
    }
}

impl Scene {
    ///
    /// Creates a new scene with no entities defined
    ///
    pub fn empty() -> Scene {
        let core = SceneCore::default();
        let core = Arc::new(Desync::new(core));

        Scene {
            core
        }
    }

    ///
    /// Returns the context for this scene
    ///
    pub fn context(&self) -> Arc<SceneContext> {
        Arc::new(SceneContext::with_no_entity(&self.core))
    }

    ///
    /// Creates a channel to send messages in this context
    ///
    pub fn send_to<TMessage>(&self, entity_id: EntityId) -> Result<impl EntityChannel<Message=TMessage>, EntityChannelError>
        where
            TMessage: 'static + Send,
    {
        SceneContext::with_no_entity(&self.core).send_to(entity_id)
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
        SceneContext::with_no_entity(&self.core).create_entity(entity_id, runtime)
    }

    ///
    /// Specify that entities that can process messages of type `TNewMessage` can also process messages of type `TOriginalMessage`
    ///
    /// That is, if an entity can be addressed using `EntityChannel<Message=TNewMessage>` it will automatically convert from `TOriginalMessage`
    /// so that `EntityChannel<Message=TSourceMessage>` also works.
    ///
    pub fn convert_message<TOriginalMessage, TNewMessage>(&self) -> Result<(), SceneContextError>
        where
            TOriginalMessage: 'static + Send,
            TNewMessage: 'static + Send + From<TOriginalMessage>,
    {
        SceneContext::with_no_entity(&self.core).convert_message::<TOriginalMessage, TNewMessage>()
    }

    ///
    /// Runs this scene
    ///
    pub async fn run(self) {
        // Prepare state (gets moved into the poll function)
        let mut running_futures = vec![];

        let (sender, receiver) = oneshot::channel();
        self.core.sync(move |core| {
            core.wake_scene = Some(sender);
        });
        let mut wake_receiver = receiver;

        // Run the scene
        future::poll_fn::<(), _>(move |context| {
            loop {
                // Drain the waiting futures from the core, and load them into our scheduler
                let waiting_futures = self.core.sync(|core| {
                    if core.is_stopped {
                        // Core is stopped, so abort this future
                        None
                    } else {
                        // Core is still running
                        let waiting_futures = mem::take(&mut core.waiting_futures);

                        if !waiting_futures.is_empty() || core.wake_scene.is_none() {
                            let (sender, receiver) = oneshot::channel();
                            core.wake_scene = Some(sender);
                            wake_receiver = receiver;
                        }

                        Some(waiting_futures)
                    }
                });

                // Stop running if the scene core is stopped
                let waiting_futures = if let Some(waiting_futures) = waiting_futures { waiting_futures } else { return Poll::Ready(()); };

                // Each future gets its own waker
                let waiting_futures = waiting_futures.into_iter()
                    .map(|future| {
                        let waker = Arc::new(SceneWaker::from_context(context));
                        Some((waker, future))
                    });
                running_futures.extend(waiting_futures);

                // Run futures until they're all asleep again, or the core wakes us
                loop {
                    let mut is_awake = false;
                    let mut complete_futures = false;

                    for maybe_future in running_futures.iter_mut() {
                        if let Some((waker, future)) = maybe_future {
                            // Nothing to do if this future isn't awake yet
                            if !waker.is_awake() {
                                continue;
                            }

                            is_awake = true;

                            // Poll the future to put it back to sleep
                            waker.go_to_sleep(context);

                            let future_waker = task::waker(Arc::clone(&waker));
                            let mut future_context = task::Context::from_waker(&future_waker);

                            match future.poll_unpin(&mut future_context) {
                                Poll::Pending => {}
                                Poll::Ready(_) => {
                                    complete_futures = true;
                                    *maybe_future = None;
                                }
                            }
                        } else {
                            complete_futures = true;
                        }
                    }

                    // Tidy up any complete futures
                    if complete_futures {
                        running_futures.retain(|future| future.is_some());
                    }

                    // See if the core has woken us up once the futures are polled
                    if let Poll::Ready(_) = wake_receiver.poll_unpin(context) {
                        // Break out of the inner loop to service the core
                        break;
                    }

                    // Stop running once all of the futures are asleep
                    if !is_awake {
                        // Core is asleep, and all of the internal futures are asleep too
                        return Poll::Pending;
                    }
                }   // Inner loop
            }       // Outer loop
        }).await;
    }
}
