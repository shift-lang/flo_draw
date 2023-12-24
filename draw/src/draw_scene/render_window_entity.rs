use std::sync::*;

use flo_scene::*;

use flo_canvas_events::*;

#[cfg(all(feature = "render-opengl", not(feature = "render-wgpu")))]
use super::glutin_render_window_entity::*;
#[cfg(feature = "render-wgpu")]
use super::wgpu_render_window_entity::*;

///
/// Creates a render window in a scene with the specified entity ID
///
#[cfg(all(feature = "render-opengl", not(feature = "render-wgpu")))]
pub fn create_render_window_entity(
    context: &Arc<SceneContext>,
    entity_id: EntityId,
    initial_size: (u64, u64),
) -> Result<SimpleEntityChannel<RenderWindowRequest>, CreateEntityError> {
    create_glutin_render_window_entity(context, entity_id, initial_size)
}

///
/// Retrieves or creates a scene context for flo_draw
///
#[cfg(all(feature = "render-wgpu"))]
pub fn create_render_window_entity(
    context: &Arc<SceneContext>,
    entity_id: EntityId,
    initial_size: (u64, u64),
) -> Result<SimpleEntityChannel<RenderWindowRequest>, CreateEntityError> {
    create_wgpu_render_window_entity(context, entity_id, initial_size)
}

///
/// Retrieves or creates a scene context for flo_draw
///
#[cfg(all(not(feature = "render-wgpu"), not(feature = "render-opengl")))]
pub fn create_render_window_entity(
    context: &Arc<SceneContext>,
    entity_id: EntityId,
    initial_size: (u64, u64),
) -> Result<SimpleEntityChannel<RenderWindowRequest>, CreateEntityError> {
    panic!("No default renderer was specified when flo_draw was compiled (use `render-wgpu` or `render-opengl`)")
}
