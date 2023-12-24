use std::any::{Any, type_name, TypeId};
use std::sync::*;

use ::desync::scheduler::*;
use futures::channel::oneshot;
use futures::prelude::*;

use crate::any_entity_channel::*;
use crate::simple_entity_channel::*;

///
/// Stores the data associated with an entity
///
pub(crate) struct EntityCore {
    /// A conversion channel, which has a message type of `Box<dyn Any + Send>`.
    create_conversion_channel: Box<dyn Send + Fn() -> AnyEntityChannel>,

    /// The channel for sending requests to this entity, stored in an 'Any' box. This is of type `SimpleEntityChannel<TMessage>`
    channel: Box<dyn Send + Any>,

    /// Given the channel for this entity, causes it to close
    close_channel: Box<dyn Send + Fn(&mut Box<dyn Send + Any>) -> ()>,

    /// Everything to signal when this entity becomes ready to accept messages (None if the entity is already ready)
    when_ready: Option<Vec<oneshot::Sender<()>>>,

    /// The queue used for running the entity (this runs the entities main future)
    queue: Arc<JobQueue>,

    /// The type ID of the message processed 'natively' by this entity
    message_type_id: TypeId,

    /// The name of the message type for this entity
    message_type_name: &'static str,
}

impl EntityCore {
    ///
    /// Creates a new entity that receives messages on the specified channel
    ///
    pub fn new<TMessage>(channel: SimpleEntityChannel<TMessage>) -> EntityCore
        where
            TMessage: 'static + Send,
    {
        let conversion_channel = channel.clone();
        let create_conversion_channel = move || { AnyEntityChannel::from_channel(conversion_channel.clone()) };
        let close_channel = move |channel: &mut Box<dyn Send + Any>| {
            if let Some(channel) = channel.downcast_mut::<SimpleEntityChannel<TMessage>>() {
                channel.close();
            }
        };

        EntityCore {
            create_conversion_channel: Box::new(create_conversion_channel),
            channel: Box::new(channel),
            close_channel: Box::new(close_channel),
            when_ready: Some(vec![]),
            queue: scheduler().create_job_queue(),
            message_type_id: TypeId::of::<TMessage>(),
            message_type_name: type_name::<TMessage>(),
        }
    }

    ///
    /// Closes the channel associated with this entity
    ///
    pub fn close(&mut self) {
        let channel = &mut self.channel;
        let close_channel = &self.close_channel;

        (close_channel)(channel);
    }

    ///
    /// Retrieves the message processed 'natively' by this channel
    ///
    pub fn message_type_id(&self) -> TypeId {
        self.message_type_id
    }

    ///
    /// Retrieves the message processed 'natively' by this channel
    ///
    pub fn message_type_name(&self) -> String {
        self.message_type_name.to_string()
    }

    ///
    /// Returns the queue for this entity
    ///
    /// The queue typically just has a single future scheduled on it, so this is usually not useful as nothing
    /// else can run here untl the entity has been finalized
    ///
    pub fn queue(&self) -> Arc<JobQueue> {
        Arc::clone(&self.queue)
    }

    ///
    /// If this entity has an implementation of a particular channel, returns it
    ///
    pub fn attach_channel<TMessage>(&self) -> Option<SimpleEntityChannel<TMessage>>
        where
            TMessage: 'static + Send,
    {
        // Downcast the channel back to a concrete type
        let channel = self.channel.downcast_ref::<SimpleEntityChannel<TMessage>>()?;

        // Clone it to create the channel for the receiver
        Some(channel.clone())
    }

    ///
    /// Returns the channel with polymorphic messages. The messages here unwrap to `Option<TMessage>`
    ///
    pub fn attach_channel_any(&self) -> AnyEntityChannel {
        (self.create_conversion_channel)()
    }

    ///
    /// Signals that this entity core is ready (waiting to accept messages)
    ///
    pub fn signal_start(&mut self) {
        if let Some(when_ready) = self.when_ready.take() {
            for signal_ready in when_ready.into_iter() {
                signal_ready.send(()).ok();
            }
        }
    }

    ///
    /// Returns a future that completes when this entity is ready (or is destroyed)
    ///
    pub fn wait_for_start(&mut self) -> impl Send + Future<Output=()> {
        let wait_for_ready = if let Some(when_ready) = &mut self.when_ready {
            // Create a channel to signal once this entity becomes ready
            let (sender, receiver) = oneshot::channel();

            // Add it to the list of things to signal when signal_ready() is called
            when_ready.push(sender);

            Some(receiver)
        } else {
            // The entity is already ready
            None
        };

        // Result is a future that waits for readiness
        async move {
            if let Some(wait_for_ready) = wait_for_ready {
                // Entity was not ready yet: wait for it to signal that it's active
                wait_for_ready.await.ok();
            }
        }
    }

    ///
    /// Stops the tasks associated with this entity from running
    ///
    pub fn stop(&self) {}
}
