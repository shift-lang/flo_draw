/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use futures::prelude::*;

use crate::context::*;
use crate::error::*;

///
/// Extension methods for futures in a scene context
///
pub trait SceneFutureExt {
    ///
    /// Runs this future in the background of the active entity
    ///
    fn run_in_background(self) -> Result<(), EntityFutureError>;
}

impl<T> SceneFutureExt for T
where
    T: 'static + Send + Future<Output = ()>,
{
    fn run_in_background(self) -> Result<(), EntityFutureError> {
        SceneContext::current().run_in_background(self)
    }
}
