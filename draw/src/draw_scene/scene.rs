/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::sync::*;

use flo_scene::*;

#[cfg(feature = "render-wgpu")]
use super::wgpu_scene::*;

///
/// Retrieves or creates a scene context for flo_draw
///
#[cfg(all(feature = "render-opengl", not(feature = "render-wgpu")))]
pub fn flo_draw_scene_context() -> Arc<SceneContext> {
    super::glutin_scene::flo_draw_glutin_scene_context()
}

///
/// Retrieves or creates a scene context for flo_draw
///
#[cfg(all(feature = "render-wgpu"))]
pub fn flo_draw_scene_context() -> Arc<SceneContext> {
    flo_draw_wgpu_scene_context()
}

///
/// Retrieves or creates a scene context for flo_draw
///
#[cfg(all(not(feature = "render-wgpu"), not(feature = "render-opengl")))]
pub fn flo_draw_scene_context() -> Arc<SceneContext> {
    panic!("No default renderer was specified when flo_draw was compiled (use `render-wgpu` or `render-opengl`)")
}
