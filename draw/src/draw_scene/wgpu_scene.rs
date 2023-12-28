/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::sync::*;

use futures::prelude::*;
use once_cell::sync::Lazy;

use flo_scene::*;

use crate::wgpu::*;

/// The scene context used for flo_draw, or None if a scene context has not been created yet
static DRAW_SCENE_CONTEXT: Lazy<Mutex<Option<Arc<SceneContext>>>> = Lazy::new(|| Mutex::new(None));

///
/// Retrieves or creates a scene context for flo_draw
///
pub fn flo_draw_wgpu_scene_context() -> Arc<SceneContext> {
    let mut context = DRAW_SCENE_CONTEXT.lock().unwrap();

    // Start a new scene if none was running
    if context.is_none() {
        // Create a new scene context, and run it on the winit thread
        let scene = Scene::default();
        let new_context = scene.context();

        // Run on the winit thread
        winit_thread().send_event(WinitThreadEvent::RunProcess(Box::new(move || {
            async move {
                scene.run().await;
            }
            .boxed()
        })));

        // Store as the active context
        *context = Some(new_context);
    }

    // Unwrap the scene context
    context.as_ref().unwrap().clone()
}
