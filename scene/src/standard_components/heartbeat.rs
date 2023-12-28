/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::collections::HashMap;
use std::sync::*;

use futures::prelude::*;

use crate::context::*;
use crate::entity_channel::*;
use crate::entity_id::*;
use crate::ergonomics::*;
use crate::error::*;

use super::entity_ids::*;
use super::entity_registry::*;

///
/// The reason a scene is currently 'awake'
///
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum HeartbeatState {
    /// Message queue awoken 'organically'
    Tick,

    /// Message queue awoken due to a heartbeat
    Tock,
}

///
/// The 'native' format for the heartbeat entity
///
#[derive(Debug)]
pub(crate) enum InternalHeartbeatRequest {
    /// From the scene core: indicates that a heartbeat has occurred
    GenerateHeartbeat,

    /// Request from the entity registry
    EntityUpdate(EntityUpdate),

    /// Send Heartbeat messages to the specified entity ID
    RequestHeartbeat(BoxedEntityChannel<'static, Heartbeat>),
}

///
/// Requests that can be made of the heartbeat entity
///
#[derive(Debug)]
pub enum HeartbeatRequest {
    /// Send Heartbeat messages to the specified entity ID
    RequestHeartbeat(BoxedEntityChannel<'static, Heartbeat>),
}

///
/// The heartbeat message
///
/// Typically an entity that needs to receive heartbeats would convert this into an internal message type
///
#[derive(Clone, Debug, PartialEq)]
pub struct Heartbeat;

impl From<HeartbeatRequest> for InternalHeartbeatRequest {
    fn from(req: HeartbeatRequest) -> InternalHeartbeatRequest {
        match req {
            HeartbeatRequest::RequestHeartbeat(entity_id) => {
                InternalHeartbeatRequest::RequestHeartbeat(entity_id)
            }
        }
    }
}

impl From<EntityUpdate> for InternalHeartbeatRequest {
    fn from(req: EntityUpdate) -> InternalHeartbeatRequest {
        InternalHeartbeatRequest::EntityUpdate(req)
    }
}

///
/// Creates the heartbeat entity in a context
///
pub fn create_heartbeat_entity(
    context: &Arc<SceneContext>,
) -> Result<impl EntityChannel<Message = HeartbeatRequest>, CreateEntityError> {
    // Set up converting the messages that the heartbeat entity can receive
    context.convert_message::<EntityUpdate, InternalHeartbeatRequest>()?;
    context.convert_message::<HeartbeatRequest, InternalHeartbeatRequest>()?;

    // Set up the state for the heartbeat entity
    let mut receivers = HashMap::<EntityId, BoxedEntityChannel<'static, Heartbeat>>::new();

    // Create the heartbeat entity itself
    Ok(context
        .create_entity(HEARTBEAT, move |context, mut requests| async move {
            // Request details on the entities (we track what gets destroyed so we can stop them receiving heartbeats)
            if let Ok(our_channel) = context.send_to(HEARTBEAT) {
                context
                    .send(
                        ENTITY_REGISTRY,
                        EntityRegistryRequest::TrackEntities(our_channel),
                    )
                    .await
                    .ok();
            }

            // Main message loop for the heartbeat entity
            while let Some(message) = requests.next().await {
                match message {
                    InternalHeartbeatRequest::GenerateHeartbeat => {
                        // Send heartbeats to everything that's listening (stop on any error)
                        let mut stopped = vec![];

                        for (entity_id, channel) in receivers.iter_mut() {
                            // Try to send to the channel
                            if channel.send(Heartbeat).await.is_err() {
                                // Any error adds to the stopped list
                                stopped.push(*entity_id);
                            }
                        }

                        // Remove stopped items from the receivers
                        stopped.into_iter().for_each(|id| {
                            receivers.remove(&id);
                        });
                    }

                    InternalHeartbeatRequest::EntityUpdate(EntityUpdate::CreatedEntity(
                        _entity_id,
                    )) => {
                        // Nothing to do
                    }

                    InternalHeartbeatRequest::EntityUpdate(EntityUpdate::DestroyedEntity(
                        entity_id,
                    )) => {
                        // Stop sending heartbeats to this entity
                        receivers.remove(&entity_id);
                    }

                    InternalHeartbeatRequest::RequestHeartbeat(channel) => {
                        // Add this channel to the list that get heartbeat messages
                        receivers.insert(channel.entity_id(), channel);
                    }
                }
            }
        })?
        .convert())
}
