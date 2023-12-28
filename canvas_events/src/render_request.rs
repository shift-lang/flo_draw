/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use flo_render::*;

///
/// A request to a low-level render target
///
pub enum RenderRequest {
    /// Performs the specified set of render actions immediately
    Render(Vec<RenderAction>),
}
