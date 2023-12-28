/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::scene_context_error::*;

///
/// Errors that can occur while creating a new entity
///
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum CreateEntityError {
    /// The entity that is being created already exists
    AlreadyExists,

    /// Tried to create an entity without a current scene
    NoCurrentScene,

    /// The scene context is not available because the scene has finished
    SceneFinished,

    /// The scene was requested from a point where the context was no longer available
    ThreadShuttingDown,
}

impl From<SceneContextError> for CreateEntityError {
    fn from(error: SceneContextError) -> CreateEntityError {
        CreateEntityError::from(&error)
    }
}

impl From<&SceneContextError> for CreateEntityError {
    fn from(error: &SceneContextError) -> CreateEntityError {
        match error {
            SceneContextError::NoCurrentScene => CreateEntityError::NoCurrentScene,
            SceneContextError::SceneFinished => CreateEntityError::SceneFinished,
            SceneContextError::ThreadShuttingDown => CreateEntityError::ThreadShuttingDown,
        }
    }
}
