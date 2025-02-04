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
/// The example entity is used for sample code and demonstrations where an explicit entity implementation is not required
///
pub enum ExampleRequest {
    Example,
}

///
/// Creates an example entity
///
pub fn create_example_entity(
    entity_id: EntityId,
    context: &Arc<SceneContext>,
) -> Result<impl EntityChannel<Message = ExampleRequest>, CreateEntityError> {
    context.create_entity(entity_id, move |_context, mut requests| async move {
        while let Some(_req) = requests.next().await {}
    })
}
