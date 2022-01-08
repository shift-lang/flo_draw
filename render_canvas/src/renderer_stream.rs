use super::matrix::*;
use super::resource_ids::*;
use super::render_entity::*;
use super::renderer_core::*;

use flo_canvas as canvas;
use flo_render as render;

use ::desync::*;

use futures::prelude::*;
use futures::task::{Context, Poll};
use futures::future::{BoxFuture};

use std::mem;
use std::pin::*;
use std::sync::*;
use std::collections::{VecDeque};

///
/// Tri-state version of 'option' that supports 'Unknown' as well as None and Some
///
#[derive(Clone, Copy, PartialEq)]
enum Maybe<T> {
    Unknown,
    None,
    Some(T)
}

///
/// Modifier to apply to the active shader
///
#[derive(Clone, PartialEq)]
enum ShaderModifier {
    /// The simple shader should be used
    Simple,

    /// Shader should use a dash pattern
    DashPattern(Vec<f32>),

    /// Shader should use a texture
    Texture(render::TextureId, render::Matrix, bool, f32),

    /// Shader should use a gradient
    Gradient(render::TextureId, render::Matrix, bool, f32),
}

///
/// Stream of rendering actions resulting from a draw instruction
///
pub struct RenderStream<'a> {
    /// The core where the render instructions are read from
    core: Arc<Desync<RenderCore>>,

    /// The ID of the buffer to use for rendering the background quad
    background_vertex_buffer: render::VertexBufferId,

    /// True if the frame is suspended (we're not going to generate any direct rendering due to this drawing operation)
    frame_suspended: bool,

    /// The future that is processing new drawing instructions
    processing_future: Option<BoxFuture<'a, ()>>,

    /// The current layer ID that we're processing
    layer_id: usize,

    /// The render entity within the layer that we're processing
    render_index: usize,

    /// Render actions waiting to be sent
    pending: VecDeque<render::RenderAction>,

    /// The operations to run when the rendering is complete (None if they've already been rendered)
    final_actions: Option<Vec<render::RenderAction>>,

    /// The transformation for the viewport
    viewport_transform: canvas::Transform2D
}

///
/// Represents the active state of the render stream
///
#[derive(Clone)]
struct RenderStreamState {
    /// The render target
    render_target: Option<render::RenderTargetId>,

    /// The blend mode to use
    blend_mode: Option<render::BlendMode>,

    /// The texture to use as the eraser mask (None for no eraser texture)
    erase_mask: Maybe<render::TextureId>,

    /// The texture to use for the clip mask (None for no clip mask)
    clip_mask: Maybe<render::TextureId>,

    /// The modifier to apply to the shader, if present
    shader_modifier: Option<ShaderModifier>,

    /// The transform to apply to the rendering instructions
    transform: Option<canvas::Transform2D>,

    /// The buffers to use to render the clipping region
    clip_buffers: Option<Vec<(render::VertexBufferId, render::IndexBufferId, usize)>>
}

impl<'a> RenderStream<'a> {
    ///
    /// Creates a new render stream
    ///
    pub fn new<ProcessFuture>(core: Arc<Desync<RenderCore>>, frame_suspended: bool, processing_future: ProcessFuture, viewport_transform: canvas::Transform2D, background_vertex_buffer: render::VertexBufferId, initial_actions: Vec<render::RenderAction>, final_actions: Vec<render::RenderAction>) -> RenderStream<'a>
    where   ProcessFuture: 'a+Send+Future<Output=()> {
        RenderStream {
            core:                       core,
            frame_suspended:            frame_suspended,
            background_vertex_buffer:   background_vertex_buffer,
            processing_future:          Some(processing_future.boxed()),
            pending:                    VecDeque::from(initial_actions),
            final_actions:              Some(final_actions),
            viewport_transform:         viewport_transform,
            layer_id:                   0,
            render_index:               0
        }
    }
}

impl<T> Maybe<T> {
    ///
    /// Converts to an optional value
    ///
    pub fn value(self) -> Option<Option<T>> {
        match self {
            Maybe::Unknown      => None,
            Maybe::None         => Some(None),
            Maybe::Some(val)    => Some(Some(val))
        }
    }
}

impl RenderStreamState {
    ///
    /// Creates a new render stream state
    ///
    fn new() -> RenderStreamState {
        RenderStreamState {
            render_target:      None,
            blend_mode:         None,
            erase_mask:         Maybe::Unknown,
            clip_mask:          Maybe::Unknown, 
            shader_modifier:    None,
            transform:          None,
            clip_buffers:       None
        }
    }

    ///
    /// Generates the actions required to set a particular dash pattern
    ///
    fn generate_dash_pattern(&self, pattern: &[f32]) -> Vec<render::RenderAction> {
        // Number of pixels in the dash pattern texture
        const DASH_WIDTH: usize = 256;

        // Total length determines how many bytes each dash uses
        let total_length: f32   = pattern.iter().cloned().sum();
        let pixel_length        = total_length / DASH_WIDTH as f32;

        // Do not generate a pattern for the case where the total length doesn't add up
        if total_length <= 0.0 {
            return vec![];
        }

        // Write the pixels for the dash pattern
        let mut pixels      = vec![];
        let mut pos         = 0.0;
        let mut col         = 255u8;
        let mut cur_pos     = pattern.iter();
        let mut dash_end    = *cur_pos.next().unwrap_or(&total_length);

        for _ in 0..DASH_WIDTH {
            // Switch colours while we're over the end of the dash position
            while dash_end < pos {
                let next_dash_len = cur_pos.next().unwrap_or(&total_length);
                col = if col == 0 { 255 } else { 0 };

                dash_end += next_dash_len;
            }

            // Write this pixel
            pixels.push(col);

            // Update the position
            pos += pixel_length;
        }

        // Generate the dash texture by clobbering any existing texture
        vec![
            render::RenderAction::Create1DTextureMono(DASH_TEXTURE, DASH_WIDTH),
            render::RenderAction::WriteTexture1D(DASH_TEXTURE, 0, DASH_WIDTH, Arc::new(pixels)),
            render::RenderAction::CreateMipMaps(DASH_TEXTURE)
        ]
    }

    ///
    /// Returns the render actions needed to update from the specified state to this state
    ///
    fn update_from_state(&self, from: &RenderStreamState) -> Vec<render::RenderAction> {
        let mut updates = vec![];
        let mut reset_render_target = false;

        // Update the content of the clip mask render target
        if let (Some(clip_buffers), Some(transform)) = (&self.clip_buffers, self.transform) {
            if Some(clip_buffers) != from.clip_buffers.as_ref() && clip_buffers.len() > 0 {
                let render_clip_buffers = clip_buffers.iter()
                    .rev()
                    .map(|(vertices, indices, length)| render::RenderAction::DrawIndexedTriangles(*vertices, *indices, *length));

                // Set up to render the clip buffers
                updates.extend(vec![
                    render::RenderAction::SelectRenderTarget(CLIP_RENDER_TARGET),
                    render::RenderAction::UseShader(render::ShaderType::Simple { clip_texture: None, erase_texture: None }),
                    render::RenderAction::Clear(render::Rgba8([0,0,0,255])),
                    render::RenderAction::BlendMode(render::BlendMode::AllChannelAlphaSourceOver),
                    render::RenderAction::SetTransform(transform_to_matrix(&transform)),
                ]);

                // Render the clip buffers once the state is set up
                updates.extend(render_clip_buffers);
            }
        }

        // If the clip buffers are different, make sure we reset the render target state
        if let Some(clip_buffers) = &self.clip_buffers {
            if Some(clip_buffers) != from.clip_buffers.as_ref() && clip_buffers.len() > 0 {
                reset_render_target = true;
            }
        }

        // Choose the render target
        if let Some(render_target) = self.render_target {
            if Some(render_target) != from.render_target || reset_render_target {
                updates.push(render::RenderAction::SelectRenderTarget(render_target));
            }
        }

        // Update the transform state
        if let Some(transform) = self.transform {
            if Some(transform) != from.transform || (self.render_target != from.render_target && self.render_target.is_some()) || reset_render_target {
                updates.push(render::RenderAction::SetTransform(transform_to_matrix(&transform)));
            }
        }

        // Update the shader we're using
        if let (Some(erase), Some(clip), Some(modifier)) = (self.erase_mask.value(), self.clip_mask.value(), &self.shader_modifier) {
            let mask_textures_changed   = Some(erase) != from.erase_mask.value() || Some(clip) != from.clip_mask.value();
            let render_target_changed   = self.render_target != from.render_target && self.render_target.is_some();
            let modifier_changed        = Some(modifier) != from.shader_modifier.as_ref();

            if mask_textures_changed || render_target_changed || reset_render_target || modifier_changed {
                // Pick the shader based on the modifier
                let shader = match modifier {
                    ShaderModifier::Simple                                      => render::ShaderType::Simple { erase_texture: erase, clip_texture: clip },
                    ShaderModifier::DashPattern(_)                              => render::ShaderType::DashedLine { dash_texture: DASH_TEXTURE, erase_texture: erase, clip_texture: clip },
                    ShaderModifier::Texture(texture_id, matrix, repeat, alpha)  => render::ShaderType::Texture { texture: *texture_id, texture_transform: *matrix, repeat: *repeat, alpha: *alpha, erase_texture: erase, clip_texture: clip },
                    ShaderModifier::Gradient(texture_id, matrix, repeat, alpha) => render::ShaderType::LinearGradient { texture: *texture_id, texture_transform: *matrix, repeat: *repeat, alpha: *alpha, erase_texture: erase, clip_texture: clip }
                };

                // Add to the updates
                updates.push(render::RenderAction::UseShader(shader));
            }

            // Generate the texture for the modifier if that's changed
            if modifier_changed {
                match modifier {
                    ShaderModifier::Simple                          => { }
                    ShaderModifier::DashPattern(new_dash_pattern)   => { updates.extend(self.generate_dash_pattern(new_dash_pattern).into_iter().rev()); }
                    ShaderModifier::Texture(_, _, _, _)             => { }
                    ShaderModifier::Gradient(_, _, _, _)            => { }
                }
            }
        }

        // Set the blend mode
        if let Some(blend_mode) = self.blend_mode {
            if Some(blend_mode) != from.blend_mode || (self.render_target != from.render_target && self.render_target.is_some()) || reset_render_target {
                updates.push(render::RenderAction::BlendMode(blend_mode));
            }
        }

        updates
    }
}

impl RenderCore {
    ///
    /// Generates the rendering actions for the layer with the specified handle
    ///
    fn render_layer(&mut self, viewport_transform: canvas::Transform2D, layer_handle: LayerHandle, render_state: &mut RenderStreamState) -> Vec<render::RenderAction> {
        use self::RenderEntity::*;

        let core = self;

        // Render the layer
        let mut render_order            = vec![];
        let mut active_transform        = canvas::Transform2D::identity();
        let mut layer                   = core.layer(layer_handle);

        render_state.transform          = Some(viewport_transform);
        render_state.blend_mode         = Some(render::BlendMode::DestinationOver);
        render_state.render_target      = Some(MAIN_RENDER_TARGET);
        render_state.erase_mask         = Maybe::None;
        render_state.clip_mask          = Maybe::None;
        render_state.clip_buffers       = Some(vec![]);
        render_state.shader_modifier    = Some(ShaderModifier::Simple);

        for render_idx in 0..layer.render_order.len() {
            match &layer.render_order[render_idx] {
                Missing => {
                    // Temporary state while sending a vertex buffer?
                    panic!("Tessellation is not complete (vertex buffer went missing)");
                },

                Tessellating(_id) => { 
                    // Being processed? (shouldn't happen)
                    panic!("Tessellation is not complete (tried to render too early)");
                },

                VertexBuffer(_buffers, _) => {
                    // Should already have sent all the vertex buffers
                    panic!("Tessellation is not complete (found unexpected vertex buffer in layer)");
                },

                DrawIndexed(vertex_buffer, index_buffer, num_items) => {
                    // Draw the triangles
                    render_order.push(render::RenderAction::DrawIndexedTriangles(*vertex_buffer, *index_buffer, *num_items));
                },

                RenderSprite(sprite_id, sprite_transform) => { 
                    let sprite_id           = *sprite_id;
                    let sprite_transform    = *sprite_transform;

                    if let Some(sprite_layer) = core.sprites.get(&sprite_id) {
                        let sprite_layer = *sprite_layer;

                        // The sprite transform is appended to the viewport transform
                        let combined_transform  = &viewport_transform * &active_transform;
                        let sprite_transform    = combined_transform * sprite_transform;

                        // The items from before the sprite should be rendered using the current state
                        let old_state           = render_state.clone();

                        // Render the layer associated with the sprite
                        let render_sprite       = core.render_layer(sprite_transform, sprite_layer, render_state);

                        // Render the sprite
                        render_order.extend(render_sprite);

                        // Restore the state back to the state before the sprite was rendered
                        render_order.extend(old_state.update_from_state(&render_state));

                        // Following instructions are rendered using the state before the sprite
                        *render_state           = old_state;
                    }

                    // Reborrow the layer
                    layer                   = core.layer(layer_handle);
                },

                SetTransform(new_transform) => {
                    // The new transform will apply to all the following render instructions
                    active_transform        = *new_transform;

                    // Update the state to a state with the new transformation applied
                    let old_state           = render_state.clone();
                    render_state.transform  = Some(&viewport_transform * &active_transform);

                    render_order.extend(render_state.update_from_state(&old_state));
                },

                SetBlendMode(new_blend_mode) => {
                    let old_state               = render_state.clone();

                    // Render the main buffer
                    render_state.blend_mode     = Some(*new_blend_mode);
                    render_state.render_target  = Some(MAIN_RENDER_TARGET);
                    render_state.erase_mask     = Maybe::None;

                    // Update to the new state
                    render_order.extend(render_state.update_from_state(&old_state));
                },

                EnableClipping(vertex_buffer, index_buffer, buffer_size) => {
                    // The preceding instructions should render according to the previous state
                    let old_state               = render_state.clone();
                    render_state.clip_mask      = Maybe::Some(CLIP_RENDER_TEXTURE);
                    render_state.clip_buffers.get_or_insert_with(|| vec![]).push((*vertex_buffer, *index_buffer, *buffer_size));

                    // Update to the new state
                    render_order.extend(render_state.update_from_state(&old_state));
                }

                DisableClipping => {
                    // Remove the clip mask from the state
                    let old_state               = render_state.clone();
                    render_state.clip_mask      = Maybe::None;
                    render_state.clip_buffers   = Some(vec![]);

                    // Update to the new state
                    render_order.extend(render_state.update_from_state(&old_state));
                }

                SetFlatColor => {
                    // Set the shader modifier to use the dash pattern (overriding any other shader modifier)
                    let old_state                   = render_state.clone();
                    render_state.shader_modifier    = Some(ShaderModifier::Simple);

                    // Update to the new state
                    render_order.extend(render_state.update_from_state(&old_state));
                }

                SetDashPattern(dash_pattern) => {
                    // Set the shader modifier to use the dash pattern (overriding any other shader modifier)
                    let old_state               = render_state.clone();
                    if dash_pattern.len() > 0 {
                        render_state.shader_modifier = Some(ShaderModifier::DashPattern(dash_pattern.clone()));
                    } else {
                        render_state.shader_modifier = Some(ShaderModifier::Simple);
                    }

                    // Update to the new state
                    render_order.extend(render_state.update_from_state(&old_state));
                }

                SetFillTexture(texture_id, matrix, repeat, alpha) => {
                    // Set the shader modifier to use the fill texture (overriding any other shader modifier)
                    let old_state               = render_state.clone();
                    render_state.shader_modifier = Some(ShaderModifier::Texture(*texture_id, *matrix, *repeat, *alpha));

                    // Update to the new state
                    render_order.extend(render_state.update_from_state(&old_state));
                }

                SetFillGradient(texture_id, matrix, repeat, alpha) => {
                    // Set the shader modifier to use the gradient texture (overriding any other shader modifier)
                    let old_state                   = render_state.clone();
                    render_state.shader_modifier    = Some(ShaderModifier::Gradient(*texture_id, *matrix, *repeat, *alpha));

                    // Update to the new state
                    render_order.extend(render_state.update_from_state(&old_state));
                }
            }
        }

        // Generate a pending set of actions for the current layer
        return render_order;
    }
}

impl<'a> RenderStream<'a> {
    ///
    /// Adds the instructions required to render the background colour to the pending queue
    ///
    fn render_background(&mut self) {
        let background_color = self.core.sync(|core| core.background_color);

        // If there's a background colour, then the finalize step should draw it (the OpenGL renderer has issues blitting alpha blended multisampled textures, so this hides that the 'clear' step above doesn't work there)
        let render::Rgba8([br, bg, bb, ba]) = background_color;

        if ba > 0 {
            // Create the actions to render the background colour
            let background_color    = [br, bg, bb, ba];
            let background_actions  = vec![
                // Generate a full-screen quad
                render::RenderAction::CreateVertex2DBuffer(self.background_vertex_buffer, vec![
                    render::Vertex2D { pos: [-1.0, -1.0],   tex_coord: [0.0, 0.0], color: background_color },
                    render::Vertex2D { pos: [1.0, 1.0],     tex_coord: [0.0, 0.0], color: background_color },
                    render::Vertex2D { pos: [1.0, -1.0],    tex_coord: [0.0, 0.0], color: background_color },

                    render::Vertex2D { pos: [-1.0, -1.0],   tex_coord: [0.0, 0.0], color: background_color },
                    render::Vertex2D { pos: [1.0, 1.0],     tex_coord: [0.0, 0.0], color: background_color },
                    render::Vertex2D { pos: [-1.0, 1.0],    tex_coord: [0.0, 0.0], color: background_color },
                ]),

                // Render the quad using the default blend mode
                render::RenderAction::SetTransform(render::Matrix::identity()),
                render::RenderAction::BlendMode(render::BlendMode::SourceOver),
                render::RenderAction::UseShader(render::ShaderType::Simple { erase_texture: None, clip_texture: None }),
                render::RenderAction::DrawTriangles(self.background_vertex_buffer, 0..6),
            ];

            // Add to the end of the queue
            self.pending.extend(background_actions);
        }
    }
}

impl<'a> Stream for RenderStream<'a> {
    type Item = render::RenderAction;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<render::RenderAction>> { 
        // Return the next pending action if there is one
        if self.pending.len() > 0 {
            return Poll::Ready(self.pending.pop_front());
        }

        // Poll the tessellation process if it's still running
        if let Some(processing_future) = self.processing_future.as_mut() {
            // Poll the future and send over any vertex buffers that might be waiting
            if processing_future.poll_unpin(context) == Poll::Pending {
                // Still generating render buffers
                // TODO: can potentially send the buffers to the renderer when they're generated here
                return Poll::Pending;
            } else {
                // Finished processing the rendering: can send the actual rendering commands to the hardware layer
                self.processing_future  = None;
                self.layer_id           = self.core.sync(|core| core.layers.len());
                self.render_index       = 0;

                // Perform any setup actions that might exist or have been generated before proceeding
                let (setup_actions, release_textures)   = self.core.sync(|core| (mem::take(&mut core.setup_actions), core.free_unused_textures()));
                
                // TODO: would be more memory efficient to release the textures first, but it's possible for the texture setup to create and never use a texture that is then released...
                self.pending.extend(setup_actions.into_iter());
                self.pending.extend(release_textures);

                if let Some(next) = self.pending.pop_front() {
                    return Poll::Ready(Some(next));
                }
            }
        }

        // We've generated all the vertex buffers: if frame rendering is suspended, stop here
        if self.frame_suspended {
            if let Some(final_actions) = self.final_actions.take() {
                self.pending = final_actions.into();
                return Poll::Ready(self.pending.pop_front());
            } else {
                return Poll::Ready(None);
            }
        }

        // We've generated all the vertex buffers: generate the instructions to render them
        let mut layer_id        = self.layer_id;
        let viewport_transform  = self.viewport_transform;

        let result              = if layer_id == 0 {
            // Stop if we've processed all the layers
            None
        } else {
            // Move to the previous layer
            layer_id -= 1;

            self.core.sync(|core| {
                // Send any pending vertex buffers, then render the layer
                let layer_handle        = core.layers[layer_id];
                let send_vertex_buffers = core.send_vertex_buffers(layer_handle);
                let mut render_state    = RenderStreamState::new();

                let mut render_layer    = core.render_layer(viewport_transform, layer_handle, &mut render_state);
                render_layer.extend(send_vertex_buffers);
                render_layer.extend(render_state.update_from_state(&RenderStreamState::new()));

                Some(render_layer)
            })
        };

        // Update the layer ID to continue iterating
        self.layer_id       = layer_id;

        // Add the result to the pending queue
        if let Some(result) = result {
            // There are more actions to add to the pending actions
            self.pending = result.into();
            return Poll::Ready(self.pending.pop_front());
        } else if let Some(final_actions) = self.final_actions.take() {
            // There are no more drawing actions, but we have a set of final post-render instructions to execute
            self.pending = final_actions.into();
            self.render_background();
            return Poll::Ready(self.pending.pop_front());
        } else {
            // No further actions if the result was empty
            return Poll::Ready(None);
        }
    }
}
