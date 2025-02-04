/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::entity_channel_error::*;

///
/// Errors that can occur while executing a recipe
///
#[derive(Clone, Debug, PartialEq)]
pub enum RecipeError {
    /// A channel that the recipe was trying to send to experienced an error
    ChannelError(EntityChannelError),

    /// A channel did not generate the response that was expected
    UnexpectedResponse,

    /// A channel expected more responses before it was dropped
    ExpectedMoreResponses,

    /// A recipe timed out before it could be completed
    Timeout,

    /// Scene stopped before the recipe could be completed
    SceneStopped,

    /// Several things failed simultaneously
    ManyErrors(Vec<RecipeError>),
}

impl From<EntityChannelError> for RecipeError {
    fn from(error: EntityChannelError) -> RecipeError {
        RecipeError::ChannelError(error)
    }
}
