/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::scene_context_error::*;

///
/// Errors that can occur while creating a default behaviour
///
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum CreateDefaultError {
    /// Default behaviour is already defined for the requested message type
    AlreadyExists,

    /// Tried to create an entity without a current scene
    NoCurrentScene,

    /// The scene context is not available because the scene has finished
    SceneFinished,

    /// The scene was requested from a point where the context was no longer available
    ThreadShuttingDown,
}

impl From<SceneContextError> for CreateDefaultError {
    fn from(error: SceneContextError) -> CreateDefaultError {
        CreateDefaultError::from(&error)
    }
}

impl From<&SceneContextError> for CreateDefaultError {
    fn from(error: &SceneContextError) -> CreateDefaultError {
        match error {
            SceneContextError::NoCurrentScene => CreateDefaultError::NoCurrentScene,
            SceneContextError::SceneFinished => CreateDefaultError::SceneFinished,
            SceneContextError::ThreadShuttingDown => CreateDefaultError::ThreadShuttingDown,
        }
    }
}
