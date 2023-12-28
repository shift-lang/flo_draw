/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::thread;

///
/// Errors relating to scene contexts
///
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum SceneContextError {
    /// The program is not executing in a context where a scene is available
    NoCurrentScene,

    /// The scene context is not available because the scene has finished
    SceneFinished,

    /// The scene was requested from a point where the context was no longer available
    ThreadShuttingDown,
}

impl From<&SceneContextError> for SceneContextError {
    fn from(err: &SceneContextError) -> SceneContextError {
        err.clone()
    }
}

impl From<thread::AccessError> for SceneContextError {
    fn from(_err: thread::AccessError) -> SceneContextError {
        SceneContextError::ThreadShuttingDown
    }
}
