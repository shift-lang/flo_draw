use super::layer_handle::*;

use flo_render as render;
use flo_canvas as canvas;

///
/// Requests to render vertex data to textures
///
/// These actions are taken after layer tessellation has completed but before any other rendering instructions (including the setup instructions)
///
#[derive(Clone, Copy, Debug)]
pub enum TextureRenderRequest {
    ///
    /// Apply mipmaps to the specified texture
    ///
    CreateMipMaps(render::TextureId),

    ///
    /// The specified sprite bounds should be made to fill the texture
    ///
    /// Once this instruction has been completed by a stream, the texture will not be rendered again
    ///
    FromSprite(render::TextureId, LayerHandle, canvas::SpriteBounds),

    ///
    /// A dynamic texture is re-rendered any time the layer or the canvas size changes
    ///
    DynamicTexture(render::TextureId, LayerHandle, canvas::SpriteBounds, canvas::CanvasSize, canvas::Transform2D),

    ///
    /// Copy the first texture to the second texture, then decrease the usage count of the first texture
    ///
    CopyTexture(render::TextureId, render::TextureId),
}
