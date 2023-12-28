/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::bindings::*;
use super::buffer::*;
use super::matrix_buffer::*;
use super::pipeline_configuration::*;
use super::render_target::*;

use crate::action::*;
use crate::buffer::*;

use flo_canvas;

use metal;

use std::collections::HashMap;
use std::ops::Range;
use std::sync::*;

///
/// Renderer that can write to a surface using Apple's Metal API
///
pub struct MetalRenderer {
    /// The device that this will render to
    device: metal::Device,

    /// True if the y coordinates should be flipped (eg, for off-screen rendering)
    flip_y: bool,

    /// The shader library for this renderer
    shader_library: metal::Library,

    /// The command queue we're using to render to this device
    command_queue: metal::CommandQueue,

    /// The vertex buffers allocated to this renderer
    vertex_buffers: Vec<Option<Buffer>>,

    /// The index buffers defined for this renderer
    index_buffers: Vec<Option<Buffer>>,

    /// The render targets for this renderer
    render_targets: Vec<Option<RenderTarget>>,

    /// The tetures for this renderer
    textures: Vec<Option<metal::Texture>>,

    /// The cache of render pipeline states used by this renderer
    pipeline_states: HashMap<PipelineConfiguration, metal::RenderPipelineState>,
}

///
/// The current state of a renderer
///
struct RenderState<'a> {
    /// The main render buffer texture
    main_texture: metal::Texture,

    /// The current target render buffer
    target_texture: metal::Texture,

    /// The texture that is being used for a fill operation
    fill_texture: Option<metal::Texture>,

    /// The texture used in the clipping slot
    clip_texture: Option<metal::Texture>,

    /// Buffer containing the current transformation matrix
    matrix: MatrixBuffer,

    /// Buffer containing the texture transform to apply
    texture_transform: Option<MatrixBuffer>,

    /// The alpha value to apply to the texture
    texture_alpha: Option<f64>,

    /// The active pipeline configuration
    pipeline_config: PipelineConfiguration,

    /// The active pipeline state corresponding to the pipeline configuration
    pipeline_state: metal::RenderPipelineState,

    /// The command buffer we're using to send rendering actions
    command_buffer: &'a metal::CommandBufferRef,

    /// The command encoder we're currently writing to
    command_encoder: &'a metal::RenderCommandEncoderRef,
}

impl MetalRenderer {
    ///
    /// Creates a new metal renderer using the system default device
    ///
    pub fn with_default_device() -> MetalRenderer {
        let device = metal::Device::system_default().expect("No Metal device available");
        let command_queue = device.new_command_queue();
        let shader_library = device
            .new_library_with_data(include_bytes![concat!(env!("OUT_DIR"), "/flo.metallib")])
            .unwrap();

        MetalRenderer {
            device: device,
            flip_y: false,
            command_queue: command_queue,
            vertex_buffers: vec![],
            index_buffers: vec![],
            render_targets: vec![],
            textures: vec![],
            shader_library: shader_library,
            pipeline_states: HashMap::new(),
        }
    }

    ///
    /// Creates a new metal renderer using the system default device
    ///
    pub fn with_device(device: &metal::Device, flip_y: bool) -> MetalRenderer {
        let device = device.clone();
        let command_queue = device.new_command_queue();
        let shader_library = device
            .new_library_with_data(include_bytes![concat!(env!("OUT_DIR"), "/flo.metallib")])
            .unwrap();

        MetalRenderer {
            device: device,
            flip_y: flip_y,
            command_queue: command_queue,
            vertex_buffers: vec![],
            index_buffers: vec![],
            render_targets: vec![],
            textures: vec![],
            shader_library: shader_library,
            pipeline_states: HashMap::new(),
        }
    }

    ///
    /// Returns a pipeline state for a configuration
    ///
    fn get_pipeline_state(&mut self, config: &PipelineConfiguration) -> metal::RenderPipelineState {
        // Borrow the fields
        let pipeline_states = &mut self.pipeline_states;
        let device = &self.device;
        let shader_library = &self.shader_library;

        // Retrieve the pipeline state for this configuration
        if let Some(pipeline) = pipeline_states.get(config) {
            pipeline.clone()
        } else {
            let pipeline = config.to_pipeline_state(&device, &shader_library);
            pipeline_states.insert(config.clone(), pipeline.clone());

            pipeline
        }
    }

    ///
    /// Creates a command encoder for rendering to the specified texture
    ///
    fn get_command_encoder<'a>(
        &mut self,
        command_buffer: &'a metal::CommandBufferRef,
        render_target: &metal::Texture,
    ) -> &'a metal::RenderCommandEncoderRef {
        let render_descriptor = metal::RenderPassDescriptor::new();
        let color_attachment = render_descriptor.color_attachments().object_at(0).unwrap();

        color_attachment.set_texture(Some(render_target));
        color_attachment.set_load_action(metal::MTLLoadAction::Load);
        color_attachment.set_store_action(metal::MTLStoreAction::Store);

        command_buffer.new_render_command_encoder(&render_descriptor)
    }

    ///
    /// Creates a blitting command encoder
    ///
    fn get_blit_command_encoder<'a>(
        &self,
        command_buffer: &'a metal::CommandBufferRef,
    ) -> &'a metal::BlitCommandEncoderRef {
        command_buffer.new_blit_command_encoder()
    }

    ///
    /// Creates a command encoder for rendering to the specified texture, after clearing it
    ///
    fn get_command_encoder_with_clear<'a>(
        &mut self,
        command_buffer: &'a metal::CommandBufferRef,
        render_target: &metal::Texture,
        clear_color: Rgba8,
    ) -> &'a metal::RenderCommandEncoderRef {
        let render_descriptor = metal::RenderPassDescriptor::new();
        let color_attachment = render_descriptor.color_attachments().object_at(0).unwrap();
        let Rgba8([r, g, b, a]) = clear_color;
        let clear_color = metal::MTLClearColor::new(
            (r as f64) / 255.0,
            (g as f64) / 255.0,
            (b as f64) / 255.0,
            (a as f64) / 255.0,
        );

        color_attachment.set_texture(Some(render_target));
        color_attachment.set_clear_color(clear_color);
        color_attachment.set_load_action(metal::MTLLoadAction::Clear);
        color_attachment.set_store_action(metal::MTLStoreAction::Store);

        command_buffer.new_render_command_encoder(&render_descriptor)
    }

    ///
    /// Sets all the values in the command encoder for the specified state
    ///
    fn setup_command_encoder(&mut self, state: &RenderState) {
        // Reset the pipeline state
        state
            .command_encoder
            .set_render_pipeline_state(&state.pipeline_state);

        // Set the constant buffers
        state.command_encoder.set_vertex_buffer(
            VertexInputIndex_VertexInputIndexMatrix as u64,
            Some(&state.matrix),
            0,
        );
        state.command_encoder.set_fragment_texture(
            FragmentInputIndex_FragmentIndexClipMaskTexture as u64,
            state
                .clip_texture
                .as_ref()
                .map::<&metal::TextureRef, _>(|t| t),
        );
        state.command_encoder.set_fragment_texture(
            FragmentInputIndex_FragmentIndexTexture as u64,
            state
                .fill_texture
                .as_ref()
                .map::<&metal::TextureRef, _>(|t| t),
        );

        if let Some(texture_matrix) = &state.texture_transform {
            state.command_encoder.set_vertex_buffer(
                VertexInputIndex_VertexTextureMatrix as u64,
                Some(texture_matrix),
                0,
            );
        }

        if let Some(texture_alpha) = &state.texture_alpha {
            let alpha = *texture_alpha as f32;
            let alpha = alpha.to_ne_bytes();
            state.command_encoder.set_fragment_bytes(
                FragmentInputIndex_FragmentAlpha as u64,
                4,
                alpha.as_ptr() as _,
            );
        }
    }

    ///
    /// Performs some rendering instructions and returns the resulting command buffer
    ///
    pub fn render_to_buffer<Actions: IntoIterator<Item = RenderAction>>(
        &mut self,
        actions: Actions,
        target_texture: &metal::Texture,
    ) -> metal::CommandBuffer {
        // Create the render state
        let command_queue = self.command_queue.clone();
        let matrix = if self.flip_y {
            Matrix::identity().flip_y()
        } else {
            Matrix::identity()
        };
        let matrix = MatrixBuffer::from_matrix(&self.device, matrix);
        let pipeline_config = PipelineConfiguration::for_texture(target_texture);
        let pipeline_state = self.get_pipeline_state(&pipeline_config);
        let command_buffer = command_queue.new_command_buffer();
        let command_encoder = self.get_command_encoder_with_clear(
            command_buffer,
            target_texture,
            Rgba8([0, 0, 0, 0]),
        );

        let mut render_state = RenderState {
            main_texture: target_texture.clone(),
            target_texture: target_texture.clone(),
            fill_texture: None,
            clip_texture: None,
            matrix: matrix,
            texture_transform: None,
            texture_alpha: None,
            pipeline_config: pipeline_config,
            pipeline_state: pipeline_state,
            command_buffer: command_buffer,
            command_encoder: command_encoder,
        };

        self.setup_command_encoder(&render_state);

        // Evaluate the actions
        for action in actions {
            use self::RenderAction::*;

            match action {
                SetTransform(matrix) => {
                    self.set_transform(matrix, &mut render_state);
                }
                CreateVertex2DBuffer(id, vertices) => {
                    self.create_vertex_buffer_2d(id, vertices);
                }
                CreateIndexBuffer(id, indices) => {
                    self.create_index_buffer(id, indices);
                }
                FreeVertexBuffer(id) => {
                    self.free_vertex_buffer(id);
                }
                FreeIndexBuffer(id) => {
                    self.free_index_buffer(id);
                }
                BlendMode(blend_mode) => {
                    self.blend_mode(blend_mode, &mut render_state);
                }
                CreateRenderTarget(render_id, texture_id, Size2D(width, height), render_type) => {
                    self.create_render_target(render_id, texture_id, width, height, render_type);
                }
                FreeRenderTarget(render_id) => {
                    self.free_render_target(render_id);
                }
                SelectRenderTarget(render_id) => {
                    self.select_render_target(render_id, &mut render_state);
                }
                RenderToFrameBuffer => {
                    self.select_main_frame_buffer(&mut render_state);
                }
                DrawFrameBuffer(render_id, region, Alpha(alpha)) => {
                    self.draw_frame_buffer(render_id, region, alpha, &mut render_state);
                }
                ShowFrameBuffer => { /* This doesn't double-buffer so nothing to do */ }
                CreateTextureBgra(texture_id, Size2D(width, height)) => {
                    self.create_bgra_texture(texture_id, width, height);
                }
                CreateTextureMono(texture_id, Size2D(width, height)) => {
                    self.create_mono_texture(texture_id, width, height);
                }
                Create1DTextureBgra(texture_id, Size1D(width)) => {
                    self.create_bgra_1d_texture(texture_id, width);
                }
                Create1DTextureMono(texture_id, Size1D(width)) => {
                    self.create_mono_1d_texture(texture_id, width);
                }
                WriteTextureData(texture_id, Position2D(x1, y1), Position2D(x2, y2), data) => {
                    self.write_texture_data_2d(texture_id, x1, y1, x2, y2, data);
                }
                WriteTexture1D(texture_id, Position1D(x1), Position1D(x2), data) => {
                    self.write_texture_data_1d(texture_id, x1, x2, data);
                }
                CreateMipMaps(texture_id) => {
                    self.create_mipmaps(texture_id, &mut render_state);
                }
                CopyTexture(src_texture, tgt_texture) => {
                    self.copy_texture(src_texture, tgt_texture, &mut render_state);
                }
                FilterTexture(texture, filter) => {
                    self.filter_texture(texture, filter, &mut render_state);
                }
                FreeTexture(texture_id) => {
                    self.free_texture(texture_id);
                }
                Clear(color) => {
                    self.clear(color, &mut render_state);
                }
                UseShader(shader_type) => {
                    self.use_shader(shader_type, &mut render_state);
                }
                DrawTriangles(buffer_id, buffer_range) => {
                    self.draw_triangles(buffer_id, buffer_range, &mut render_state);
                }
                DrawIndexedTriangles(vertex_buffer, index_buffer, num_vertices) => {
                    self.draw_indexed_triangles(
                        vertex_buffer,
                        index_buffer,
                        num_vertices,
                        &mut render_state,
                    );
                }
            }
        }

        // Finish up
        render_state.command_encoder.end_encoding();

        command_buffer.to_owned()
    }

    ///
    /// Performs rendering of the specified actions to this device target
    ///
    pub fn render<Actions: IntoIterator<Item = RenderAction>>(
        &mut self,
        actions: Actions,
        target_drawable: &metal::Drawable,
        target_texture: &metal::Texture,
    ) {
        // Perform the rendering
        let command_buffer = self.render_to_buffer(actions, target_texture);

        // Present the result
        command_buffer.present_drawable(target_drawable);
        command_buffer.commit();
    }

    ///
    /// Sets the active transformation matrix
    ///
    fn set_transform(&mut self, matrix: Matrix, state: &mut RenderState) {
        let matrix = if self.flip_y { matrix.flip_y() } else { matrix };

        // Replace the matrix buffer with a new one
        state.matrix = MatrixBuffer::from_matrix(&self.device, matrix);
        state.command_encoder.set_vertex_buffer(
            VertexInputIndex_VertexInputIndexMatrix as u64,
            Some(&state.matrix),
            0,
        );
    }

    ///
    /// Loads a vertex buffer and associates it with an ID
    ///
    fn create_vertex_buffer_2d(
        &mut self,
        VertexBufferId(vertex_id): VertexBufferId,
        vertices: Vec<Vertex2D>,
    ) {
        // Reserve space for the buffer ID
        if vertex_id >= self.vertex_buffers.len() {
            self.vertex_buffers.extend(
                (self.vertex_buffers.len()..(vertex_id + 1))
                    .into_iter()
                    .map(|_| None),
            );
        }

        // Free any existing buffer
        self.vertex_buffers[vertex_id] = None;

        // Do nothing if there are no vertexes in the buffer (just won't render)
        if vertices.len() == 0 {
            return;
        }

        // Load and store the new buffer
        self.vertex_buffers[vertex_id] = Some(Buffer::from_vertices(&self.device, vertices));
    }

    ///
    /// Loads an index buffer and associates it with an ID
    ///
    fn create_index_buffer(&mut self, IndexBufferId(index_id): IndexBufferId, indices: Vec<u16>) {
        // Reserve space for the buffer ID
        if index_id >= self.index_buffers.len() {
            self.index_buffers.extend(
                (self.index_buffers.len()..(index_id + 1))
                    .into_iter()
                    .map(|_| None),
            );
        }

        // Free any existing buffer
        self.index_buffers[index_id] = None;

        // Do nothing if there's no data to store in this buffer
        if indices.len() == 0 {
            return;
        }

        // Load and store the new buffer
        self.index_buffers[index_id] = Some(Buffer::from_indices(&self.device, indices));
    }

    ///
    /// Releases the memory associated with a vertex buffer
    ///
    fn free_vertex_buffer(&mut self, VertexBufferId(vertex_id): VertexBufferId) {
        self.vertex_buffers[vertex_id] = None;
    }

    ///
    /// Frees the index buffer with the specified ID
    ///
    fn free_index_buffer(&mut self, IndexBufferId(id): IndexBufferId) {
        self.index_buffers[id] = None;
    }

    ///
    /// Updates the blend mode for a render state
    ///
    fn blend_mode(&mut self, blend_mode: BlendMode, state: &mut RenderState) {
        state.pipeline_config.blend_mode = blend_mode;
        state.pipeline_state = self.get_pipeline_state(&state.pipeline_config);
        state
            .command_encoder
            .set_render_pipeline_state(&state.pipeline_state);
    }

    ///
    /// Creates a render target and its backing texture
    ///
    fn create_render_target(
        &mut self,
        RenderTargetId(render_id): RenderTargetId,
        TextureId(texture_id): TextureId,
        width: usize,
        height: usize,
        render_target_type: RenderTargetType,
    ) {
        // Allocate space for the texture and render target
        if render_id >= self.render_targets.len() {
            self.render_targets.extend(
                (self.render_targets.len()..(render_id + 1))
                    .into_iter()
                    .map(|_| None),
            );
        }

        if texture_id >= self.textures.len() {
            self.textures.extend(
                (self.textures.len()..(texture_id + 1))
                    .into_iter()
                    .map(|_| None),
            );
        }

        // Free any existing texture or render target
        self.render_targets[render_id] = None;
        self.textures[texture_id] = None;

        // Create the render target
        let new_render_target = RenderTarget::new(&self.device, width, height, render_target_type);

        // Store in this object
        self.textures[texture_id] = Some(new_render_target.render_texture().clone());
        self.render_targets[render_id] = Some(new_render_target);
    }

    ///
    /// Frees up a render target for this renderer
    ///
    fn free_render_target(&mut self, RenderTargetId(render_id): RenderTargetId) {
        self.render_targets[render_id] = None;
    }

    ///
    /// Selects an alternative render target
    ///
    fn select_render_target(
        &mut self,
        RenderTargetId(render_id): RenderTargetId,
        state: &mut RenderState,
    ) {
        // Fetch the render texture
        let render_target = match &self.render_targets[render_id] {
            Some(texture) => texture,
            None => {
                return;
            }
        };

        // Set the state to point at the new texture
        state.target_texture = render_target.render_texture().clone();

        // Create a command encoder that will use this texture
        state.command_encoder.end_encoding();
        state.command_encoder =
            self.get_command_encoder(state.command_buffer, &state.target_texture);

        state
            .pipeline_config
            .update_for_texture(&state.target_texture);
        state.pipeline_state = self.get_pipeline_state(&state.pipeline_config);
        state
            .command_encoder
            .set_render_pipeline_state(&state.pipeline_state);

        self.setup_command_encoder(state);
    }

    ///
    /// Sets the main frame buffer to be the current render target
    ///
    fn select_main_frame_buffer(&mut self, state: &mut RenderState) {
        // Reset the state to point at the main texture
        state.target_texture = state.main_texture.clone();

        // Create a command encoder that will use this texture
        state.command_encoder.end_encoding();
        state.command_encoder =
            self.get_command_encoder(state.command_buffer, &state.target_texture);

        state
            .pipeline_config
            .update_for_texture(&state.target_texture);
        state.pipeline_state = self.get_pipeline_state(&state.pipeline_config);
        state
            .command_encoder
            .set_render_pipeline_state(&state.pipeline_state);

        self.setup_command_encoder(state);
    }

    ///
    /// Renders a frame buffer to another texture (resolving multi-sampling if there is any)
    ///
    fn draw_frame_buffer(
        &mut self,
        RenderTargetId(source_buffer): RenderTargetId,
        region: FrameBufferRegion,
        alpha: f64,
        state: &mut RenderState,
    ) {
        let render_targets = &self.render_targets;

        if let Some(source_buffer) = &render_targets[source_buffer] {
            // Read information about the source texture
            let source_texture = source_buffer.render_texture().clone();
            let source_width = source_texture.width() as f32;
            let source_height = source_texture.height() as f32;

            // Create a pipeline state for rendering this framebuffer
            let mut config = PipelineConfiguration::for_texture(&state.target_texture);

            // Basic vertex shader and blend mode
            config.vertex_shader = String::from("simple_vertex");
            config.blend_mode = BlendMode::SourceOver;
            config.source_is_premultiplied = true;
            config.fragment_shader = if source_buffer.is_multisampled() {
                String::from("texture_multisample_fragment")
            } else {
                String::from("texture_fragment")
            };

            // Convert to a pipeline state
            let pipeline_state = self.get_pipeline_state(&config);

            // Change the state of the encoder so we're ready to render this frame buffer
            state
                .command_encoder
                .set_render_pipeline_state(&pipeline_state);

            // Generate a viewport matrix
            let target_width = state.target_texture.width() as f32;
            let target_height = state.target_texture.height() as f32;

            let scale_transform =
                flo_canvas::Transform2D::scale(2.0 / target_width, 2.0 / target_height);
            let viewport_transform = scale_transform
                * flo_canvas::Transform2D::translate(-(target_width / 2.0), -(target_height / 2.0));

            let viewport_matrix = transform_to_matrix(&viewport_transform);
            let viewport_matrix = MatrixBuffer::from_matrix(&self.device, viewport_matrix);

            // Work out the region that's being rendered
            let min_x = region.min_x();
            let min_y = region.min_y();
            let max_x = region.max_x();
            let max_y = region.max_y();

            let min_x = (min_x + 1.0) / 2.0;
            let min_y = (min_y + 1.0) / 2.0;
            let max_x = (max_x + 1.0) / 2.0;
            let max_y = (max_y + 1.0) / 2.0;

            let min_x = min_x * source_width;
            let min_y = min_y * source_height;
            let max_x = max_x * source_width;
            let max_y = max_y * source_height;

            // The rendering is a simple triangle strip
            let triangle_strip = vec![
                Vertex2D {
                    pos: [min_x, min_y],
                    tex_coord: [min_x, source_height - min_y],
                    color: [0, 0, 0, 0],
                },
                Vertex2D {
                    pos: [min_x, max_y],
                    tex_coord: [min_x, source_height - max_y],
                    color: [0, 0, 0, 0],
                },
                Vertex2D {
                    pos: [max_x, min_y],
                    tex_coord: [max_x, source_height - min_y],
                    color: [0, 0, 0, 0],
                },
                Vertex2D {
                    pos: [max_x, max_y],
                    tex_coord: [max_x, source_height - max_y],
                    color: [0, 0, 0, 0],
                },
            ];
            let triangle_strip = Buffer::from_vertices(&self.device, triangle_strip);

            // Set up the command encoder parameters
            state.command_encoder.set_vertex_buffer(
                VertexInputIndex_VertexInputIndexMatrix as u64,
                Some(&viewport_matrix),
                0,
            );
            state.command_encoder.set_vertex_buffer(
                VertexInputIndex_VertexInputIndexVertices as u64,
                Some(&triangle_strip),
                0,
            );
            state.command_encoder.set_fragment_texture(
                FragmentInputIndex_FragmentIndexTexture as u64,
                Some(&source_texture),
            );

            let alpha = alpha as f32;
            let alpha = alpha.to_ne_bytes();
            state.command_encoder.set_fragment_bytes(
                FragmentInputIndex_FragmentAlpha as u64,
                4,
                alpha.as_ptr() as _,
            );

            // Draw the texture
            state
                .command_encoder
                .draw_primitives(metal::MTLPrimitiveType::TriangleStrip, 0, 4);

            // Reset the pipeline state to the one in the render state
            state
                .command_encoder
                .set_fragment_texture(FragmentInputIndex_FragmentIndexTexture as u64, None);

            state
                .command_encoder
                .set_render_pipeline_state(&state.pipeline_state);
            self.setup_command_encoder(state);
        }
    }

    ///
    /// Stores a texture with the specified texture ID
    ///
    #[inline]
    fn store_texture(&mut self, texture_id: usize, texture: metal::Texture) {
        while self.textures.len() <= texture_id {
            self.textures.push(None);
        }

        self.textures[texture_id] = Some(texture);
    }

    ///
    /// Creates a BGRA formatted 2D texture
    ///
    fn create_bgra_texture(
        &mut self,
        TextureId(texture_id): TextureId,
        width: usize,
        height: usize,
    ) {
        // Create the texture descriptor
        let texture_descriptor = metal::TextureDescriptor::new();

        texture_descriptor.set_texture_type(metal::MTLTextureType::D2);
        texture_descriptor.set_width(width as u64);
        texture_descriptor.set_height(height as u64);
        texture_descriptor.set_pixel_format(metal::MTLPixelFormat::BGRA8Unorm);
        texture_descriptor.set_usage(metal::MTLTextureUsage::ShaderRead);
        texture_descriptor.set_mipmap_level_count_for_size(metal::MTLSize {
            width: width as _,
            height: height as _,
            depth: 1,
        });

        // Turn into a texture
        let texture = self.device.new_texture(&texture_descriptor);

        // Store in the textures
        self.store_texture(texture_id, texture);
    }

    ///
    /// Creates a monochrome 2D texture
    ///
    fn create_mono_texture(
        &mut self,
        TextureId(texture_id): TextureId,
        width: usize,
        height: usize,
    ) {
        // Create the texture descriptor
        let texture_descriptor = metal::TextureDescriptor::new();

        texture_descriptor.set_texture_type(metal::MTLTextureType::D2);
        texture_descriptor.set_width(width as u64);
        texture_descriptor.set_height(height as u64);
        texture_descriptor.set_pixel_format(metal::MTLPixelFormat::R8Unorm);
        texture_descriptor.set_usage(metal::MTLTextureUsage::ShaderRead);
        texture_descriptor.set_mipmap_level_count_for_size(metal::MTLSize {
            width: width as _,
            height: height as _,
            depth: 1,
        });

        // Turn into a texture
        let texture = self.device.new_texture(&texture_descriptor);

        // Store in the textures
        self.store_texture(texture_id, texture);
    }

    ///
    /// Creates a BGRA formatted 1D texture
    ///
    fn create_bgra_1d_texture(&mut self, TextureId(texture_id): TextureId, width: usize) {
        // Create the texture descriptor
        let texture_descriptor = metal::TextureDescriptor::new();

        texture_descriptor.set_texture_type(metal::MTLTextureType::D1);
        texture_descriptor.set_width(width as u64);
        texture_descriptor.set_pixel_format(metal::MTLPixelFormat::BGRA8Unorm);
        texture_descriptor.set_usage(metal::MTLTextureUsage::ShaderRead);

        // Turn into a texture
        let texture = self.device.new_texture(&texture_descriptor);

        // Store in the textures
        self.store_texture(texture_id, texture);
    }

    ///
    /// Creates a monochrome 1D texture
    ///
    fn create_mono_1d_texture(&mut self, TextureId(texture_id): TextureId, width: usize) {
        // Create the texture descriptor
        let texture_descriptor = metal::TextureDescriptor::new();

        texture_descriptor.set_texture_type(metal::MTLTextureType::D1);
        texture_descriptor.set_width(width as u64);
        texture_descriptor.set_pixel_format(metal::MTLPixelFormat::R8Unorm);
        texture_descriptor.set_usage(metal::MTLTextureUsage::ShaderRead);

        // Turn into a texture
        let texture = self.device.new_texture(&texture_descriptor);

        // Store in the textures
        self.store_texture(texture_id, texture);
    }

    ///
    /// Writes texture data to a 2D texture
    ///
    fn write_texture_data_2d(
        &mut self,
        TextureId(texture_id): TextureId,
        x1: usize,
        y1: usize,
        x2: usize,
        y2: usize,
        data: Arc<Vec<u8>>,
    ) {
        // Sanity check
        if x2 < x1 {
            return;
        }
        if y2 < y1 {
            return;
        }

        // Load the texture
        let texture = if texture_id < self.textures.len() {
            self.textures[texture_id].as_ref()
        } else {
            None
        };
        let texture = if let Some(texture) = texture {
            texture
        } else {
            return;
        };

        // Work out the region that will be written
        let region = metal::MTLRegion {
            origin: metal::MTLOrigin {
                x: x1 as _,
                y: y1 as _,
                z: 0,
            },
            size: metal::MTLSize {
                width: (x2 - x1) as _,
                height: (y2 - y1) as _,
                depth: 1,
            },
        };

        // Check that the bytes are the right size (need to know the texture pixel format)
        let bytes_per_pixel = match texture.pixel_format() {
            metal::MTLPixelFormat::R8Unorm => 1,
            metal::MTLPixelFormat::BGRA8Unorm => 4,
            _ => todo!("Unsupported texture pixel format"),
        };

        let expected_size = (x2 - x1) * (y2 - y1) * bytes_per_pixel;
        if data.len() < expected_size {
            return;
        }

        // Write the bytes to the texture
        texture.replace_region(
            region,
            0,
            data.as_ptr() as _,
            (bytes_per_pixel * (x2 - x1)) as _,
        );
    }

    ///
    /// Writes texture data to a 1D texture
    ///
    fn write_texture_data_1d(
        &mut self,
        TextureId(texture_id): TextureId,
        x1: usize,
        x2: usize,
        data: Arc<Vec<u8>>,
    ) {
        // Sanity check
        if x2 < x1 {
            return;
        }

        // Load the texture
        let texture = if texture_id < self.textures.len() {
            self.textures[texture_id].as_ref()
        } else {
            None
        };
        let texture = if let Some(texture) = texture {
            texture
        } else {
            return;
        };

        // Work out the region that will be written
        let region = metal::MTLRegion {
            origin: metal::MTLOrigin {
                x: x1 as _,
                y: 0,
                z: 0,
            },
            size: metal::MTLSize {
                width: (x2 - x1) as _,
                height: 1,
                depth: 1,
            },
        };

        // Check that the bytes are the right size (need to know the texture pixel format)
        let bytes_per_pixel = match texture.pixel_format() {
            metal::MTLPixelFormat::R8Unorm => 1,
            metal::MTLPixelFormat::BGRA8Unorm => 4,
            _ => todo!("Unsupported texture pixel format"),
        };

        let expected_size = (x2 - x1) * bytes_per_pixel;
        if data.len() < expected_size {
            return;
        }

        // Write the bytes to the texture
        texture.replace_region(
            region,
            0,
            data.as_ptr() as _,
            (bytes_per_pixel * (x2 - x1)) as _,
        );
    }

    ///
    /// Creates the mipmaps for a particular texture
    ///
    fn create_mipmaps(&mut self, TextureId(texture_id): TextureId, state: &mut RenderState) {
        let texture = if texture_id < self.textures.len() {
            self.textures[texture_id].as_ref()
        } else {
            None
        };
        let texture = if let Some(texture) = texture {
            texture
        } else {
            return;
        };

        // Must be mipmap levels defined for the texture
        if texture.mipmap_level_count() <= 1 {
            return;
        }

        // Will need to recycle the command encoder
        state.command_encoder.end_encoding();

        // Use a blit encoder to generate the mipmaps
        let blit_encoder = self.get_blit_command_encoder(state.command_buffer);
        blit_encoder.generate_mipmaps(texture);
        blit_encoder.end_encoding();

        // Generate a new command encoder
        state.command_encoder =
            self.get_command_encoder(state.command_buffer, &state.target_texture);
        self.setup_command_encoder(state);
    }

    ///
    /// Generates a copy of an existing texture
    ///
    fn copy_texture(
        &mut self,
        TextureId(src_texture_id): TextureId,
        TextureId(tgt_texture_id): TextureId,
        state: &mut RenderState,
    ) {
        // Degenerate cases
        if src_texture_id == tgt_texture_id {
            return;
        }

        // Free the target texture if it exists
        while self.textures.len() <= tgt_texture_id {
            self.textures.push(None);
        }
        self.textures[tgt_texture_id] = None;

        // Fetch the source texture
        let src_texture = if src_texture_id < self.textures.len() {
            self.textures[src_texture_id].as_ref()
        } else {
            None
        };
        let src_texture = if let Some(src_texture) = src_texture {
            src_texture
        } else {
            return;
        };

        // Create a target texture from the source texture
        let texture_descriptor = metal::TextureDescriptor::new();
        let texture_type = src_texture.texture_type();
        let width = src_texture.width();
        let height = src_texture.height();

        texture_descriptor.set_texture_type(texture_type);
        texture_descriptor.set_width(width);
        texture_descriptor.set_pixel_format(src_texture.pixel_format());
        texture_descriptor.set_usage(metal::MTLTextureUsage::ShaderRead);

        if texture_type == metal::MTLTextureType::D2 {
            texture_descriptor.set_height(height);
            texture_descriptor.set_mipmap_level_count_for_size(metal::MTLSize {
                width: width as _,
                height: height as _,
                depth: 1,
            });
        }

        // Create the texture from the descriptor
        let tgt_texture = self.device.new_texture(&texture_descriptor);

        // Copy the texture using a blit encoder
        state.command_encoder.end_encoding();

        // Use a blit encoder to generate the mipmaps
        let blit_encoder = self.get_blit_command_encoder(state.command_buffer);
        blit_encoder.copy_from_texture(
            &src_texture,
            0,
            0,
            metal::MTLOrigin { x: 0, y: 0, z: 0 },
            metal::MTLSize {
                width,
                height,
                depth: 1,
            },
            &tgt_texture,
            0,
            0,
            metal::MTLOrigin { x: 0, y: 0, z: 0 },
        );
        blit_encoder.end_encoding();

        // Generate a new command encoder
        state.command_encoder =
            self.get_command_encoder(state.command_buffer, &state.target_texture);
        self.setup_command_encoder(state);

        // Store the target texture
        self.store_texture(tgt_texture_id, tgt_texture);
    }

    ///
    /// Applies a filter to an existing texture
    ///
    fn filter_texture(
        &mut self,
        TextureId(texture_id): TextureId,
        filter: Vec<TextureFilter>,
        state: &mut RenderState,
    ) {
        todo!()
    }

    ///
    /// Frees up an existing texture
    ///
    fn free_texture(&mut self, TextureId(texture_id): TextureId) {
        if texture_id < self.textures.len() {
            self.textures[texture_id] = None;
        }
    }

    ///
    /// Clears the current texture
    ///
    fn clear(&mut self, color: Rgba8, state: &mut RenderState) {
        // Metal forces clears to be done at the start of a new render pass
        state.command_encoder.end_encoding();
        state.command_encoder =
            self.get_command_encoder_with_clear(state.command_buffer, &state.target_texture, color);

        self.setup_command_encoder(state);
    }

    ///
    /// Chooses a shader for the following rendering instructions
    ///
    fn use_shader(&mut self, shader_type: ShaderType, state: &mut RenderState) {
        // Reset the current shader state
        state.pipeline_config.vertex_shader = String::from("simple_vertex");
        state.fill_texture = None;
        state.clip_texture = None;
        state.texture_transform = None;

        // Update the state according to the shader type
        match shader_type {
            ShaderType::DashedLine {
                dash_texture: _,
                clip_texture: _,
            } => {
                // Not currently supported
                todo!()
            }

            ShaderType::Simple { clip_texture: None } => {
                state.pipeline_config.fragment_shader = String::from("simple_fragment")
            }

            ShaderType::Simple {
                clip_texture: Some(TextureId(clip_texture)),
            } => {
                state.pipeline_config.fragment_shader =
                    String::from("simple_clip_mask_multisample_fragment");
                state.clip_texture = self.textures[clip_texture].clone();
            }

            ShaderType::Texture {
                texture: TextureId(fill_texture),
                texture_transform,
                repeat,
                alpha,
                clip_texture: None,
            } => {
                state.pipeline_config.vertex_shader = String::from("texture_vertex");
                state.pipeline_config.fragment_shader = String::from("texture_fragment");
                state.texture_transform =
                    Some(MatrixBuffer::from_matrix(&self.device, texture_transform));
                state.texture_alpha = Some(alpha as _);

                state.fill_texture = self.textures[fill_texture].clone();
            }

            ShaderType::Texture {
                texture: TextureId(fill_texture),
                texture_transform,
                repeat,
                alpha,
                clip_texture: Some(TextureId(clip_texture)),
            } => {
                state.pipeline_config.vertex_shader = String::from("texture_vertex");
                state.pipeline_config.fragment_shader =
                    String::from("texture_clip_mask_multisample_fragment");
                state.texture_transform =
                    Some(MatrixBuffer::from_matrix(&self.device, texture_transform));
                state.texture_alpha = Some(alpha as _);

                state.fill_texture = self.textures[fill_texture].clone();
                state.clip_texture = self.textures[clip_texture].clone();
            }

            ShaderType::LinearGradient {
                texture: TextureId(gradient_texture),
                texture_transform,
                repeat,
                alpha,
                clip_texture: None,
            } => {
                state.pipeline_config.vertex_shader = String::from("gradient_vertex");
                state.pipeline_config.fragment_shader = String::from("gradient_fragment");
                state.texture_transform =
                    Some(MatrixBuffer::from_matrix(&self.device, texture_transform));
                state.texture_alpha = Some(alpha as _);

                state.fill_texture = self.textures[gradient_texture].clone();
            }

            ShaderType::LinearGradient {
                texture: TextureId(gradient_texture),
                texture_transform,
                repeat,
                alpha,
                clip_texture: Some(TextureId(clip_texture)),
            } => {
                state.pipeline_config.vertex_shader = String::from("gradient_vertex");
                state.pipeline_config.fragment_shader =
                    String::from("gradient_clip_mask_multisample_fragment");
                state.texture_transform =
                    Some(MatrixBuffer::from_matrix(&self.device, texture_transform));
                state.texture_alpha = Some(alpha as _);

                state.fill_texture = self.textures[gradient_texture].clone();
                state.clip_texture = self.textures[clip_texture].clone();
            }
        }

        // Update the command encoder with the new state
        state.pipeline_state = self.get_pipeline_state(&state.pipeline_config);
        self.setup_command_encoder(state);
    }

    ///
    /// Draws triangles from a vertex buffer
    ///
    fn draw_triangles(
        &mut self,
        VertexBufferId(vertex_buffer_id): VertexBufferId,
        range: Range<usize>,
        state: &mut RenderState,
    ) {
        // Fetch the buffer to draw
        let buffer = match &self.vertex_buffers[vertex_buffer_id] {
            Some(buffer) => buffer,
            None => {
                return;
            }
        };

        // Draw these vertices
        state.command_encoder.set_vertex_buffer(
            VertexInputIndex_VertexInputIndexVertices as u64,
            Some(buffer),
            0,
        );
        state.command_encoder.draw_primitives(
            metal::MTLPrimitiveType::Triangle,
            range.start as u64,
            range.len() as u64,
        );
    }

    ///
    /// Draws triangles using vertices referenced by an index buffer
    ///
    fn draw_indexed_triangles(
        &mut self,
        VertexBufferId(vertex_buffer_id): VertexBufferId,
        IndexBufferId(index_buffer_id): IndexBufferId,
        num_vertices: usize,
        state: &mut RenderState,
    ) {
        // Fetch the buffer and index buffer to draw
        let vertex_buffer = match &self.vertex_buffers[vertex_buffer_id] {
            Some(buffer) => buffer,
            None => {
                return;
            }
        };
        let index_buffer = match &self.index_buffers[index_buffer_id] {
            Some(buffer) => buffer,
            None => {
                return;
            }
        };

        // Draw these vertices
        state.command_encoder.set_vertex_buffer(
            VertexInputIndex_VertexInputIndexVertices as u64,
            Some(vertex_buffer),
            0,
        );
        state.command_encoder.draw_indexed_primitives(
            metal::MTLPrimitiveType::Triangle,
            num_vertices as u64,
            metal::MTLIndexType::UInt16,
            index_buffer,
            0,
        );
    }
}

///
/// Converts a canvas transform to a rendering matrix
///
fn transform_to_matrix(transform: &flo_canvas::Transform2D) -> Matrix {
    let flo_canvas::Transform2D(t) = transform;

    Matrix([
        [t[0][0], t[0][1], 0.0, t[0][2]],
        [t[1][0], t[1][1], 0.0, t[1][2]],
        [t[2][0], t[2][1], 1.0, t[2][2]],
        [0.0, 0.0, 0.0, 1.0],
    ])
}
