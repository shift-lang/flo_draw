/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::*;

use flo_rope::*;
use futures::channel::oneshot;
use futures::future;
use futures::future::Either;
use futures::prelude::*;

use flo_binding::*;

use crate::context::*;
use crate::entity_channel::*;
use crate::entity_id::*;
use crate::ergonomics::*;
use crate::error::*;
use crate::simple_entity_channel::*;

use super::entity_ids::*;
use super::entity_registry::*;
use super::floating_binding::*;

#[cfg(feature = "properties")]
lazy_static! {
    static ref MESSAGE_PROCESSORS: RwLock<
        HashMap<
            TypeId,
            Box<
                dyn Send
                    + Sync
                    + Fn(Box<dyn Send + Any>, &mut PropertiesState, &Arc<SceneContext>) -> (),
            >,
        >,
    > = RwLock::new(HashMap::new());
}

///
/// A single value property is defined in a format that's suitable for use with the `flo_binding` library, which is to say the
/// `BindRef` type, which can be used as a reference to any other binding.
///
/// Note that while there's a standard property entity with the `PROPERTIES` entity ID, it's possible to create new property entities
/// to define properties with entirely independent 'namespaces'.
///
pub struct PropertyDefinition<TValue>
where
    TValue: 'static + PartialEq + Clone + Send + Sized,
{
    /// The entity that owns this property
    pub owner: EntityId,

    /// The name of this property
    pub name: Arc<String>,

    /// The stream of values for this property
    ///
    /// The property won't be created until this has returned at least one value. The property will stop updating but not be destroyed
    /// if this stream is closed.
    pub values: BindRef<TValue>,
}

///
/// A rope property definition is based around a `RopeBinding` instead of a `BindRef` and can track sequences of things (with optional
/// attributes)
///
/// Note that while there's a standard property entity with the `PROPERTIES` entity ID, it's possible to create new property entities
/// to define properties with entirely independent 'namespaces'.
///
pub struct RopePropertyDefinition<TCell, TAttribute>
where
    TCell: 'static + Send + Unpin + Clone + PartialEq,
    TAttribute: 'static + Send + Sync + Unpin + Clone + PartialEq + Default,
{
    /// The entity that owns this property
    pub owner: EntityId,

    /// The name of this property
    pub name: Arc<String>,

    /// The stream of values for this property
    ///
    /// The property won't be created until this has returned at least one value. The property will stop updating but not be destroyed
    /// if this stream is closed.
    pub values: RopeBinding<TCell, TAttribute>,
}

///
/// A reference to an existing property
///
#[derive(Clone, PartialEq, Hash, Debug)]
pub struct PropertyReference {
    /// The entity that owns this property
    pub owner: EntityId,

    /// The name of the property
    pub name: Arc<String>,
}

///
/// A message about an update to a property
///
#[derive(Clone, PartialEq, Hash, Debug)]
pub enum PropertyUpdate {
    /// A property has been created
    Created(PropertyReference),

    /// A property has been destroyed
    Destroyed(PropertyReference),
}

///
/// Requests that can be made of a property entity
///
pub enum PropertyRequest<TValue>
where
    TValue: 'static + PartialEq + Clone + Send + Sized,
{
    /// Creates a new property
    CreateProperty(PropertyDefinition<TValue>),

    /// Removes the property with the specified name
    DestroyProperty(PropertyReference),

    /// Retrieves the `BindRef<TValue>` by sending it to the specified binding target
    Get(PropertyReference, FloatingBindingTarget<BindRef<TValue>>),

    /// Sends the values for a property to a channel
    ///
    /// 'Follow' is less efficient than using 'Get' and then following the resulting property but differs
    /// from that result in that the stream will end when the property or entity is destroyed in the property
    /// bindings rather than when the underlying binding is dropped. It also sends directly to an entity
    /// channel, which makes it easy to use a property as a source of messages for another entity.
    Follow(PropertyReference, BoxedEntityChannel<'static, TValue>),

    /// Whenever a property with the specified name is created or destroyed, notify the specified channel
    TrackPropertiesWithName(String, BoxedEntityChannel<'static, PropertyUpdate>),
}

///
/// Requests that can be made of a property entity that contains a rope property
///
pub enum RopePropertyRequest<TCell, TAttribute>
where
    TCell: 'static + Send + Unpin + Clone + PartialEq,
    TAttribute: 'static + Send + Sync + Unpin + Clone + PartialEq + Default,
{
    /// Creates a new property
    CreateProperty(RopePropertyDefinition<TCell, TAttribute>),

    /// Removes the property with the specified name
    DestroyProperty(PropertyReference),

    /// Retrieves the `BindRef<TValue>` containing this property (this shares the data more efficiently than Follow does)
    Get(
        PropertyReference,
        FloatingBindingTarget<RopeBinding<TCell, TAttribute>>,
    ),

    /// Whenever a property with the specified name is created, notify the specified channel
    TrackPropertiesWithName(String, BoxedEntityChannel<'static, PropertyUpdate>),
}

///
/// An internal property request contains an 'Any' request for properties of a given type
///
enum InternalPropertyRequest {
    /// A PropertyRequest<x> that's wrapped in a Box<Any> for a type that is recognised by the property entity, along with the entity ID it is for
    AnyRequest(Option<EntityId>, Box<dyn Send + Any>),

    /// Pings the properties entity to ensure it's ready for requests
    Ready,

    /// A new entity was created
    CreatedEntity(EntityId),

    /// An entity was destroyed
    DestroyedEntity(EntityId),
}

impl<TValue> From<PropertyRequest<TValue>> for InternalPropertyRequest
where
    TValue: 'static + PartialEq + Clone + Send + Sized,
{
    fn from(req: PropertyRequest<TValue>) -> InternalPropertyRequest {
        // The internal value is Option<PropertyRequest<TValue>>, which allows the caller to take the value out of the box later on
        InternalPropertyRequest::AnyRequest(req.target_entity_id(), Box::new(Some(req)))
    }
}

impl<TCell, TAttribute> From<RopePropertyRequest<TCell, TAttribute>> for InternalPropertyRequest
where
    TCell: 'static + Send + Unpin + Clone + PartialEq,
    TAttribute: 'static + Send + Sync + Unpin + Clone + PartialEq + Default,
{
    fn from(req: RopePropertyRequest<TCell, TAttribute>) -> InternalPropertyRequest {
        // The internal value is Option<PropertyRequest<TValue>>, which allows the caller to take the value out of the box later on
        InternalPropertyRequest::AnyRequest(req.target_entity_id(), Box::new(Some(req)))
    }
}

impl<TValue> PropertyRequest<TValue>
where
    TValue: 'static + PartialEq + Clone + Send + Sized,
{
    /// Retrieves the entity ID that
    fn target_entity_id(&self) -> Option<EntityId> {
        use PropertyRequest::*;

        match self {
            CreateProperty(PropertyDefinition { owner, .. }) => Some(*owner),
            DestroyProperty(PropertyReference { owner, .. }) => Some(*owner),
            Get(PropertyReference { owner, .. }, _) => Some(*owner),
            Follow(PropertyReference { owner, .. }, _) => Some(*owner),
            TrackPropertiesWithName(_, _) => None,
        }
    }
}

impl<TCell, TAttribute> RopePropertyRequest<TCell, TAttribute>
where
    TCell: 'static + Send + Unpin + Clone + PartialEq,
    TAttribute: 'static + Send + Sync + Unpin + Clone + PartialEq + Default,
{
    /// Retrieves the entity ID that
    fn target_entity_id(&self) -> Option<EntityId> {
        use RopePropertyRequest::*;

        match self {
            CreateProperty(RopePropertyDefinition { owner, .. }) => Some(*owner),
            DestroyProperty(PropertyReference { owner, .. }) => Some(*owner),
            Get(PropertyReference { owner, .. }, _) => Some(*owner),
            TrackPropertiesWithName(_, _) => None,
        }
    }
}

impl From<EntityUpdate> for InternalPropertyRequest {
    fn from(req: EntityUpdate) -> InternalPropertyRequest {
        match req {
            EntityUpdate::CreatedEntity(entity_id) => {
                InternalPropertyRequest::CreatedEntity(entity_id)
            }
            EntityUpdate::DestroyedEntity(entity_id) => {
                InternalPropertyRequest::DestroyedEntity(entity_id)
            }
        }
    }
}

impl<TValue> PropertyDefinition<TValue>
where
    TValue: 'static + PartialEq + Clone + Send + Sized,
{
    ///
    /// Creates a new property definition that has the most recent value received on a stream
    ///
    pub fn from_stream(
        owner: EntityId,
        name: &str,
        values: impl 'static + Send + Unpin + Stream<Item = TValue>,
        default_value: TValue,
    ) -> PropertyDefinition<TValue> {
        PropertyDefinition {
            owner: owner,
            name: Arc::new(name.to_string()),
            values: BindRef::from(bind_stream(values, default_value, |_old, new| new)),
        }
    }

    ///
    /// Creates a new property definition from an existing bound value
    ///
    pub fn from_binding(
        owner: EntityId,
        name: &str,
        values: impl Into<BindRef<TValue>>,
    ) -> PropertyDefinition<TValue> {
        PropertyDefinition {
            owner: owner,
            name: Arc::new(name.to_string()),
            values: values.into(),
        }
    }
}

impl<TCell, TAttribute> RopePropertyDefinition<TCell, TAttribute>
where
    TCell: 'static + Send + Unpin + Clone + PartialEq,
    TAttribute: 'static + Send + Sync + Unpin + Clone + PartialEq + Default,
{
    ///
    /// Creates a new property definition that has the most recent value received on a stream
    ///
    pub fn from_stream(
        owner: EntityId,
        name: &str,
        values: impl 'static + Send + Unpin + Stream<Item = RopeAction<TCell, TAttribute>>,
    ) -> RopePropertyDefinition<TCell, TAttribute> {
        RopePropertyDefinition {
            owner: owner,
            name: Arc::new(name.to_string()),
            values: RopeBinding::from_stream(values),
        }
    }

    ///
    /// Creates a new property definition from an existing bound value
    ///
    pub fn from_binding(
        owner: EntityId,
        name: &str,
        values: impl Into<RopeBinding<TCell, TAttribute>>,
    ) -> RopePropertyDefinition<TCell, TAttribute> {
        RopePropertyDefinition {
            owner: owner,
            name: Arc::new(name.to_string()),
            values: values.into(),
        }
    }
}

impl PropertyReference {
    ///
    /// Creates a new property definition
    ///
    pub fn new(owner: EntityId, name: &str) -> PropertyReference {
        PropertyReference {
            owner: owner,
            name: Arc::new(name.to_string()),
        }
    }
}

impl<TValue> Drop for Property<TValue> {
    fn drop(&mut self) {
        use std::mem;

        // Take the list of channels to send to when this property is dropped
        let mut when_dropped = vec![];
        mem::swap(&mut *self.when_dropped.lock().unwrap(), &mut when_dropped);

        // Notify them all that the property has been dropped
        when_dropped.into_iter().for_each(|(_id, sender)| {
            sender.send(()).ok();
        });
    }
}

///
/// Retrieves an entity channel to talk to the properties entity about properties of type `TValue`. This is the same as calling `context.send_to()`
/// except this will ensure a suitable conversion for communicating with the properties entity is set up. That is `send_to()` won't work until this
/// has been called at least once for the scene with the property type.
///
/// `properties_entity_id` is the ID of the properties entity that the caller wants a channel for (it's not the ID of the entity that is having
/// properties attached to it)
///
/// Typically `properties_entity_id` should be `PROPERTIES` here, but it's possible to run multiple sets of properties in a given scene so other values are
/// possible if `create_properties_entity()` has been called for other entity IDs.
///
pub async fn properties_channel<TValue>(
    properties_entity_id: EntityId,
    context: &Arc<SceneContext>,
) -> Result<BoxedEntityChannel<'static, PropertyRequest<TValue>>, EntityChannelError>
where
    TValue: 'static + PartialEq + Clone + Send + Sized,
{
    // Add a processor for this type if one doesn't already exist
    {
        let mut message_processors = MESSAGE_PROCESSORS.write().unwrap();

        message_processors
            .entry(TypeId::of::<Option<PropertyRequest<TValue>>>())
            .or_insert_with(|| {
                Box::new(|message, state, context| {
                    process_message::<TValue>(message, state, context)
                })
            });
    }

    // Before returning a channel, wait for the properties entity to become ready
    // (This is most useful at startup when we need the entity tracking to start up before anything else)
    // We don't do this for the properties entity itself (so it has a chance to declare some properties before it becomes 'ready')
    if context.entity() != Some(properties_entity_id) {
        context
            .send::<_>(properties_entity_id, InternalPropertyRequest::Ready)
            .await
            .ok();
    }

    // Ensure that the message is converted to an internal request
    context.convert_message::<PropertyRequest<TValue>, InternalPropertyRequest>()?;

    // Send messages to the properties entity
    context.send_to(properties_entity_id)
}

///
/// Retrieves an entity channel to talk to the properties entity about rope properties of type `<TCell, TAttribute>.
///
/// `properties_entity_id` is the ID of the properties entity that the caller wants a channel for (it's not the ID of the entity that is having
/// properties attached to it)
///
/// Typically `properties_entity_id` should be `PROPERTIES` here, but it's possible to run multiple sets of properties in a given scene so other values are
/// possible if `create_properties_entity()` has been called for other entity IDs.
///
pub async fn rope_properties_channel<TCell, TAttribute>(
    properties_entity_id: EntityId,
    context: &Arc<SceneContext>,
) -> Result<BoxedEntityChannel<'static, RopePropertyRequest<TCell, TAttribute>>, EntityChannelError>
where
    TCell: 'static + Send + Unpin + Clone + PartialEq,
    TAttribute: 'static + Send + Sync + Unpin + Clone + PartialEq + Default,
{
    // Add a processor for this type if one doesn't already exist
    {
        let mut message_processors = MESSAGE_PROCESSORS.write().unwrap();

        message_processors
            .entry(TypeId::of::<Option<RopePropertyRequest<TCell, TAttribute>>>())
            .or_insert_with(|| {
                Box::new(|message, state, context| {
                    process_rope_message::<TCell, TAttribute>(message, state, context)
                })
            });
    }

    // Before returning a channel, wait for the properties entity to become ready
    // (This is most useful at startup when we need the entity tracking to start up before anything else)
    if context.entity() != Some(properties_entity_id) {
        context
            .send::<_>(properties_entity_id, InternalPropertyRequest::Ready)
            .await
            .ok();
    }

    // Ensure that the message is converted to an internal request
    context.convert_message::<RopePropertyRequest<TCell, TAttribute>, InternalPropertyRequest>()?;

    // Send messages to the properties entity
    context.send_to(properties_entity_id)
}

///
/// Used to represent the state of the properties entity at any given time
///
struct PropertiesState {
    /// ID used to identify resources
    next_id: usize,

    /// The properties for each entity in the scene. The value is an `Arc<Mutex<Property<TValue>>>` in an any box
    properties: HashMap<EntityId, HashMap<Arc<String>, AnyProperty>>,

    /// Binding containing the list of registered entities
    entities: RopeBindingMut<EntityId, ()>,

    /// Trackers for properties of particular types and names (type -> names -> channels)
    trackers:
        HashMap<TypeId, HashMap<String, Vec<Option<BoxedEntityChannel<'static, PropertyUpdate>>>>>,
}

///
/// Data associated with a property
///
struct Property<TValue> {
    /// The current value, if known
    current_value: BindRef<TValue>,

    /// What to notify when this property is dropped
    when_dropped: Mutex<Vec<(usize, oneshot::Sender<()>)>>,
}

///
/// Data associated with a rope property
///
struct RopeProperty<TCell, TAttribute>
where
    TCell: 'static + Send + Unpin + Clone + PartialEq,
    TAttribute: 'static + Send + Sync + Unpin + Clone + PartialEq + Default,
{
    /// The current value, if known
    current_value: RopeBinding<TCell, TAttribute>,
}

///
/// A property stripped of its type data
///
struct AnyProperty {
    /// An `Arc<Mutex<Property<TValue>>>` or an `Arc<Mutex<RopeProperty<TCell, TAttribute>>>` in an any box
    any_property: Box<dyn Send + Any>,

    /// The type of the property (`TypeId::of::<Arc<Property<TValue>>>` or `TypeId::of::<Arc<RopeProperty<TCell, TAttribute>>>`)
    type_id: TypeId,
}

///
/// Processes a message, where the message is expected to be of a particular type
///
fn process_message<TValue>(
    any_message: Box<dyn Send + Any>,
    state: &mut PropertiesState,
    context: &Arc<SceneContext>,
) where
    TValue: 'static + PartialEq + Clone + Send + Sized,
{
    // Try to unbox the message. The type is Option<PropertyRequest> so we can take it out of the 'Any' reference
    let mut any_message = any_message;
    let message = any_message
        .downcast_mut::<Option<PropertyRequest<TValue>>>()
        .and_then(|msg| msg.take());
    let message = if let Some(message) = message {
        message
    } else {
        return;
    };

    // The action depends on the message content
    use PropertyRequest::*;
    match message {
        CreateProperty(definition) => {
            let owner = definition.owner;
            let name = definition.name;
            let values = definition.values;

            // Create the property
            let property = Property::<TValue> {
                current_value: values,
                when_dropped: Mutex::new(vec![]),
            };
            let property = Arc::new(property);

            // If replacing an existing property, then notify of a deletion/recreation
            let replacing_property = state
                .properties
                .get(&owner)
                .and_then(|entity_properties| entity_properties.get(&name))
                .is_some();

            // If there are any trackers for this property type in the state, then notify them than this property was created
            let trackers = state
                .trackers
                .get_mut(&TypeId::of::<Arc<Property<TValue>>>())
                .and_then(|trackers| trackers.get_mut(&*name));

            if let Some(trackers) = trackers {
                let mut pending_messages = vec![];
                let new_reference = PropertyReference::new(owner, &*name);

                // Queue messages saying this property was created
                for maybe_tracker in trackers.iter_mut() {
                    if let Some(tracker) = maybe_tracker {
                        if tracker.is_closed() {
                            // Mark for later cleanup
                            *maybe_tracker = None;
                        } else {
                            // Send a 'destroyed' message if the property was destroyed
                            let send_destroyed = if replacing_property {
                                Some(
                                    tracker
                                        .send(PropertyUpdate::Destroyed(new_reference.clone()))
                                        .map(|_maybe_err| ()),
                                )
                            } else {
                                None
                            };

                            // Send the 'created' message to this tracker
                            let send_created = tracker
                                .send(PropertyUpdate::Created(new_reference.clone()))
                                .map(|_maybe_err| ());
                            pending_messages.push(async move {
                                if let Some(send_destroyed) = send_destroyed {
                                    send_destroyed.await
                                }
                                send_created.await;
                            });
                        }
                    }
                }

                // Finish the messages in the background
                if !pending_messages.is_empty() {
                    future::join_all(pending_messages)
                        .map(|_| ())
                        .run_in_background()
                        .ok();
                }

                // Throw out any trackers that are done
                trackers.retain(|tracker| tracker.is_some());
            }

            // Store a copy of the property in the state (we use the entity registry to know which entities exist)
            if let Some(entity_properties) = state.properties.get_mut(&owner) {
                let any_property = AnyProperty {
                    any_property: Box::new(Arc::clone(&property)),
                    type_id: TypeId::of::<Arc<Property<TValue>>>(),
                };

                entity_properties.insert(name, any_property);
            }
        }

        DestroyProperty(reference) => {
            // If there are any trackers for this property type in the state, then notify them than this property was destroyed
            let trackers = state
                .trackers
                .get_mut(&TypeId::of::<Arc<Property<TValue>>>())
                .and_then(|trackers| trackers.get_mut(&*reference.name));

            if let Some(trackers) = trackers {
                let mut pending_messages = vec![];

                // Queue messages saying this property was created
                for maybe_tracker in trackers.iter_mut() {
                    if let Some(tracker) = maybe_tracker {
                        if tracker.is_closed() {
                            // Mark for later cleanup
                            *maybe_tracker = None;
                        } else {
                            // Send to this tracker
                            let send_future = tracker
                                .send(PropertyUpdate::Destroyed(reference.clone()))
                                .map(|_maybe_err| ());
                            pending_messages.push(send_future);
                        }
                    }
                }

                // Finish the messages in the background
                if !pending_messages.is_empty() {
                    future::join_all(pending_messages)
                        .map(|_| ())
                        .run_in_background()
                        .ok();
                }

                // Throw out any trackers that are done
                trackers.retain(|tracker| tracker.is_some());
            }

            // Remove from the state
            if let Some(entity_properties) = state.properties.get_mut(&reference.owner) {
                entity_properties.remove(&reference.name);
            }
        }

        Get(reference, target) => {
            // See if there's a property with the appropriate name
            if let Some(property) = state
                .properties
                .get_mut(&reference.owner)
                .and_then(|entity| entity.get_mut(&reference.name))
            {
                if let Some(property) = property
                    .any_property
                    .downcast_mut::<Arc<Property<TValue>>>()
                {
                    // Return the binding to the caller
                    target.finish_binding(property.current_value.clone());
                } else {
                    target.missing();
                }
            } else {
                target.missing();
            }
        }

        Follow(reference, target) => {
            if let Some(property) = state
                .properties
                .get_mut(&reference.owner)
                .and_then(|entity| entity.get_mut(&reference.name))
            {
                if let Some(property) = property
                    .any_property
                    .downcast_mut::<Arc<Property<TValue>>>()
                {
                    // Create channel to detect when this property is released
                    let follow_id = state.next_id;
                    state.next_id += 1;

                    let (property_dropped_sender, property_dropped) = oneshot::channel();
                    property
                        .when_dropped
                        .lock()
                        .unwrap()
                        .push((follow_id, property_dropped_sender));

                    // Follow the property values in a stream
                    let mut property_values = follow(property.current_value.clone());

                    // We'll need a reference to the property when the stream shuts down
                    let weak_property = Arc::downgrade(&property);

                    // Run a background task to pass the values on to the target
                    context
                        .run_in_background(async move {
                            let mut target = target;
                            let mut property_dropped = property_dropped;

                            while let Either::Left((Some(next_value), prop_dropped)) =
                                future::select(property_values.next(), property_dropped).await
                            {
                                property_dropped = prop_dropped;

                                // Send to the target
                                if target.send(next_value).await.is_err() {
                                    // Stop if the target is no longer listening for changes
                                    break;
                                }
                            }

                            // Remove from the when_dropped list
                            if let Some(property) = weak_property.upgrade() {
                                property
                                    .when_dropped
                                    .lock()
                                    .unwrap()
                                    .retain(|(id, _)| *id != follow_id);
                            }
                        })
                        .ok();
                }
            }
        }

        TrackPropertiesWithName(name, channel) => {
            let our_type = TypeId::of::<Arc<Property<TValue>>>();
            let mut channel = channel;

            // Send messages about properties with this name and type (need to iterate across all entities)
            let mut pending_messages = vec![];

            for (entity_id, properties) in state.properties.iter() {
                if let Some(property) = properties.get(&name) {
                    if property.type_id == our_type {
                        let send_future = channel
                            .send(PropertyUpdate::Created(PropertyReference::new(
                                *entity_id, &name,
                            )))
                            .map(|_maybe_err| ());
                        pending_messages.push(send_future);
                    }
                }
            }

            future::join_all(pending_messages)
                .map(|_| ())
                .run_in_background()
                .ok();

            // Create a tracker for properties as they're created
            state
                .trackers
                .entry(our_type)
                .or_insert_with(|| HashMap::new())
                .entry(name)
                .or_insert_with(|| vec![])
                .push(Some(channel));
        }
    }
}

///
/// Processes a message, where the message is expected to be of a particular type
///
fn process_rope_message<TCell, TAttribute>(
    any_message: Box<dyn Send + Any>,
    state: &mut PropertiesState,
    _context: &Arc<SceneContext>,
) where
    TCell: 'static + Send + Unpin + Clone + PartialEq,
    TAttribute: 'static + Send + Sync + Unpin + Clone + PartialEq + Default,
{
    // Try to unbox the message. The type is Option<PropertyRequest> so we can take it out of the 'Any' reference
    let mut any_message = any_message;
    let message = any_message
        .downcast_mut::<Option<RopePropertyRequest<TCell, TAttribute>>>()
        .and_then(|msg| msg.take());
    let message = if let Some(message) = message {
        message
    } else {
        return;
    };

    // The action depends on the message content
    use RopePropertyRequest::*;
    match message {
        CreateProperty(definition) => {
            let owner = definition.owner;
            let name = definition.name;
            let values = definition.values;

            // Create the property
            let property = RopeProperty::<TCell, TAttribute> {
                current_value: values,
            };
            let property = Arc::new(property);

            // If there are any trackers for this property type in the state, then notify them than this property was created
            let trackers = state
                .trackers
                .get_mut(&TypeId::of::<Arc<RopeProperty<TCell, TAttribute>>>())
                .and_then(|trackers| trackers.get_mut(&*name));

            if let Some(trackers) = trackers {
                let mut pending_messages = vec![];
                let new_reference = PropertyReference::new(owner, &*name);

                // Queue messages saying this property was created
                for maybe_tracker in trackers.iter_mut() {
                    if let Some(tracker) = maybe_tracker {
                        if tracker.is_closed() {
                            // Mark for later cleanup
                            *maybe_tracker = None;
                        } else {
                            // Send to this tracker
                            let send_future = tracker
                                .send(PropertyUpdate::Created(new_reference.clone()))
                                .map(|_maybe_err| ());
                            pending_messages.push(send_future);
                        }
                    }
                }

                // Finish the messages in the background
                if !pending_messages.is_empty() {
                    future::join_all(pending_messages)
                        .map(|_| ())
                        .run_in_background()
                        .ok();
                }

                // Throw out any trackers that are done
                trackers.retain(|tracker| tracker.is_some());
            }

            // Store a copy of the property in the state (we use the entity registry to know which entities exist)
            if let Some(entity_properties) = state.properties.get_mut(&owner) {
                let any_property = AnyProperty {
                    any_property: Box::new(Arc::clone(&property)),
                    type_id: TypeId::of::<Arc<RopeProperty<TCell, TAttribute>>>(),
                };

                entity_properties.insert(name, any_property);
            }
        }

        DestroyProperty(reference) => {
            // If there are any trackers for this property type in the state, then notify them than this property was destroyed
            let trackers = state
                .trackers
                .get_mut(&TypeId::of::<Arc<RopeProperty<TCell, TAttribute>>>())
                .and_then(|trackers| trackers.get_mut(&*reference.name));

            if let Some(trackers) = trackers {
                let mut pending_messages = vec![];

                // Queue messages saying this property was created
                for maybe_tracker in trackers.iter_mut() {
                    if let Some(tracker) = maybe_tracker {
                        if tracker.is_closed() {
                            // Mark for later cleanup
                            *maybe_tracker = None;
                        } else {
                            // Send to this tracker
                            let send_future = tracker
                                .send(PropertyUpdate::Destroyed(reference.clone()))
                                .map(|_maybe_err| ());
                            pending_messages.push(send_future);
                        }
                    }
                }

                // Finish the messages in the background
                if !pending_messages.is_empty() {
                    future::join_all(pending_messages)
                        .map(|_| ())
                        .run_in_background()
                        .ok();
                }

                // Throw out any trackers that are done
                trackers.retain(|tracker| tracker.is_some());
            }

            // Remove the property from the state
            if let Some(entity_properties) = state.properties.get_mut(&reference.owner) {
                entity_properties.remove(&reference.name);
            }
        }

        Get(reference, target) => {
            // See if there's a property with the appropriate name
            if let Some(property) = state
                .properties
                .get_mut(&reference.owner)
                .and_then(|entity| entity.get_mut(&reference.name))
            {
                if let Some(property) = property
                    .any_property
                    .downcast_mut::<Arc<RopeProperty<TCell, TAttribute>>>()
                {
                    // Return the binding to the caller
                    target.finish_binding(property.current_value.clone());
                } else {
                    target.missing();
                }
            } else {
                target.missing();
            }
        }

        TrackPropertiesWithName(name, channel) => {
            let our_type = TypeId::of::<Arc<RopeProperty<TCell, TAttribute>>>();
            let mut channel = channel;

            // Send messages about properties with this name and type (need to iterate across all entities)
            let mut pending_messages = vec![];

            for (entity_id, properties) in state.properties.iter() {
                if let Some(property) = properties.get(&name) {
                    if property.type_id == our_type {
                        let send_future = channel
                            .send(PropertyUpdate::Created(PropertyReference::new(
                                *entity_id, &name,
                            )))
                            .map(|_maybe_err| ());
                        pending_messages.push(send_future);
                    }
                }
            }

            future::join_all(pending_messages)
                .map(|_| ())
                .run_in_background()
                .ok();

            // Create a tracker for properties as they're created
            state
                .trackers
                .entry(our_type)
                .or_insert_with(|| HashMap::new())
                .entry(name)
                .or_insert_with(|| vec![])
                .push(Some(channel));
        }
    }
}

///
/// Creates a new properties entity with the specified ID in the given context
///
/// The result here is '()' as the properties channel is defined per property type. Call `properties_channel()` to retrieve channels
/// for properties of particular types. Note that while calling `send_to()` on a scene context will also often work, it won't
/// automatically set up the needed type conversion, so it will fail if called for a type that hasn't been encountered before.
///
#[cfg(feature = "properties")]
pub fn create_properties_entity(
    entity_id: EntityId,
    context: &Arc<SceneContext>,
) -> Result<(), CreateEntityError> {
    // Create the state for the properties entity
    let mut state = PropertiesState {
        next_id: 0,
        properties: HashMap::new(),
        entities: RopeBindingMut::new(),
        trackers: HashMap::new(),
    };

    context
        .convert_message::<EntityUpdate, InternalPropertyRequest>()
        .unwrap();

    // Create the entity itself
    context.create_entity(entity_id, move |context, mut messages| async move {
        // Request updates from the entity registry
        let properties = context.send_to::<EntityUpdate>(entity_id);
        let properties = if let Ok(properties) = properties {
            properties
        } else {
            return;
        };

        if let Some(mut entity_registry) = context.send_to(ENTITY_REGISTRY).ok() {
            entity_registry
                .send(EntityRegistryRequest::TrackEntities(properties))
                .await
                .ok();
        }

        // Bind the properties for the properties entity itself
        let entities_channel = rope_properties_channel(entity_id, &context).await.ok();
        if let Some(mut entities_channel) = entities_channel {
            // Possible to fail if the scene is shut down very quickly
            entities_channel
                .send(RopePropertyRequest::CreateProperty(
                    RopePropertyDefinition::from_binding(entity_id, "Entities", &state.entities),
                ))
                .map(|maybe_err| {
                    maybe_err.ok();
                })
                .run_in_background()
                .ok();
        }

        while let Some(message) = messages.next().await {
            let message: InternalPropertyRequest = message;

            match message {
                InternalPropertyRequest::Ready => {
                    // This is just used to synchronise requests to the entity
                }

                InternalPropertyRequest::AnyRequest(target_entity_id, request) => {
                    // If the entity ID is not in the state, then read the entities from the entities channel (it is possible for messages about properties
                    // on an entity to reach us before the entity registry has sent us the CreatedEntity request)
                    if let (Some(target_entity_id), Some(mut entity_registry)) =
                        (target_entity_id, context.send_to(ENTITY_REGISTRY).ok())
                    {
                        // Have an entity ID and the ability to talk to the entity registry...
                        if !state.properties.contains_key(&target_entity_id) {
                            // Ask the entity registry for all the entities that exist
                            let (all_entities_list_sender, mut all_entities_list) =
                                SimpleEntityChannel::new(entity_id, 10);

                            if entity_registry
                                .send(EntityRegistryRequest::GetEntities(
                                    all_entities_list_sender.boxed(),
                                ))
                                .await
                                .is_ok()
                            {
                                // Make sure they are all in the properties list
                                while let Some(entity_id) = all_entities_list.next().await {
                                    // Note the entities rope is not updated here but later on when the 'CreatedEntity' request arrives
                                    state
                                        .properties
                                        .entry(entity_id)
                                        .or_insert_with(|| HashMap::new());
                                }
                            }
                        }
                    }

                    // Lock the message processors so we can read from them
                    let message_processors = MESSAGE_PROCESSORS.read().unwrap();

                    // Fetch the ID of the type in the request
                    let request_type = (&*request).type_id();

                    // Try to retrieve a processor for this type (these are created when properties_channel is called to retrieve properties of this type)
                    if let Some(request_processor) = message_processors.get(&request_type) {
                        // Process the request
                        request_processor(request, &mut state, &context);
                    } else {
                        // No processor available
                    }
                }

                InternalPropertyRequest::CreatedEntity(entity_id) => {
                    // Add a new set of properties for this entity, if we're not already tracking it
                    // (Main reason we will already be tracking it is if something tried to create a property on the entity before this request arrived)
                    state
                        .properties
                        .entry(entity_id)
                        .or_insert_with(|| HashMap::new());

                    // Add the new entity to the start of the entity list (note: the other place we add to the properties list is if this request arrived late, but
                    // we don't add to the entities rope at that point, so it's safe to always add here)
                    state.entities.replace(0..0, vec![entity_id]);
                }

                InternalPropertyRequest::DestroyedEntity(destroyed_entity_id) => {
                    // Remove the properties from the state
                    let removed_properties = state.properties.remove(&destroyed_entity_id);

                    // Send messages for all of the properties that were destroyed and have trackers
                    let mut pending_messages = vec![];

                    for (name, any_property) in removed_properties.into_iter().flatten() {
                        let reference = PropertyReference {
                            owner: destroyed_entity_id,
                            name: Arc::clone(&name),
                        };
                        let mut trackers = state
                            .trackers
                            .get_mut(&any_property.type_id)
                            .and_then(|trackers| trackers.get_mut(&*reference.name));

                        for maybe_tracker in
                            trackers.iter_mut().flat_map(|trackers| trackers.iter_mut())
                        {
                            if let Some(tracker) = maybe_tracker {
                                if tracker.is_closed() {
                                    // Mark for later cleanup
                                    *maybe_tracker = None;
                                } else {
                                    // Send to this tracker
                                    let send_future = tracker
                                        .send(PropertyUpdate::Destroyed(reference.clone()))
                                        .map(|_maybe_err| ());
                                    pending_messages.push(send_future);
                                }
                            }
                        }
                    }

                    if !pending_messages.is_empty() {
                        future::join_all(pending_messages)
                            .map(|_| ())
                            .run_in_background()
                            .ok();
                    }

                    // Remove the entity from the list
                    state
                        .entities
                        .retain_cells(|entity_id| entity_id != &destroyed_entity_id);
                }
            }
        }
    })?;

    Ok(())
}
