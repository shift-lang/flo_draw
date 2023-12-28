/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use flo_canvas as canvas;
use flo_render as render;

///
/// Ued to indicate the state of a gradient: these are loaded as 1-dimensional textures when they are used
///
#[derive(Clone)]
pub enum RenderGradient {
    Defined(Vec<canvas::GradientOp>),
    Ready(render::TextureId, Vec<canvas::GradientOp>),
}
