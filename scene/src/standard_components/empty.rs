/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::sync::*;

use futures::prelude::*;

use crate::context::*;
use crate::entity_channel::*;
use crate::entity_id::*;
use crate::error::*;

///
/// An empty entity is an entity that is solely used as an identifier or a placeholder:
/// for example, it could be used for an entity that only needs properties. While it
/// performs no actions, it still responds to some requests, in particular, it can be
/// shut down.
///
pub enum EmptyRequest {
    /// Requests that this entity shuts down
    Stop,
}

///
/// Creates a new empty entity. These are entities that perform no actions themselves, other
/// than a request to stop the entity. They can be useful as places to store properties.
///
pub fn empty_entity(
    entity_id: EntityId,
    context: &Arc<SceneContext>,
) -> Result<impl EntityChannel<Message = EmptyRequest>, CreateEntityError> {
    context.create_entity(entity_id, move |_, mut messages| async move {
        while let Some(msg) = messages.next().await {
            let msg: EmptyRequest = msg;

            match msg {
                EmptyRequest::Stop => {
                    break;
                }
            }
        }
    })
}
