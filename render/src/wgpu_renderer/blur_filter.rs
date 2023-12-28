/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::pipeline::*;
use super::texture::*;
use super::to_buffer::*;
use super::wgpu_shader::*;

use crate::buffer::*;

use wgpu;
use wgpu::util::DeviceExt;

use std::mem;
use std::num::*;
use std::sync::*;

///
/// Runs one of the fixed-size blur filters on a source texture
///
pub(crate) fn blur_fixed(
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    blur_pipeline: &Pipeline,
    source_texture: &WgpuTexture,
    weights: Vec<f32>,
    offsets: Vec<f32>,
) -> WgpuTexture {
    // Set up buffers
    let vertices = vec![
        Vertex2D::with_pos(-1.0, -1.0),
        Vertex2D::with_pos(-1.0, 1.0),
        Vertex2D::with_pos(1.0, 1.0),
        Vertex2D::with_pos(-1.0, -1.0),
        Vertex2D::with_pos(1.0, -1.0),
        Vertex2D::with_pos(1.0, 1.0),
    ]
    .to_buffer(device, wgpu::BufferUsages::VERTEX);

    let offset_factor = match blur_pipeline.blur_filter_direction() {
        BlurDirection::Horizontal => source_texture.descriptor.size.width as f32,
        BlurDirection::Vertical => source_texture.descriptor.size.height as f32,
    };

    // Offsets are in a 30 entry array (15 weights, 15 offsets)
    let offsets_weights = (0..30)
        .into_iter()
        .flat_map(|p| {
            if p < 15 {
                [
                    *(offsets.get(p).unwrap_or(&0.0)) / offset_factor,
                    0.0,
                    0.0,
                    0.0,
                ]
            } else {
                [*(weights.get(p - 15).unwrap_or(&0.0)), 0.0, 0.0, 0.0]
            }
        })
        .collect::<Vec<f32>>();
    let offsets_weights = offsets_weights.to_buffer(device, wgpu::BufferUsages::UNIFORM);

    // Create a target texture
    let mut target_descriptor = source_texture.descriptor.clone();
    target_descriptor.usage |= wgpu::TextureUsages::RENDER_ATTACHMENT;
    let target_texture = device.create_texture(&target_descriptor);

    // Create the blur sampler
    let blur_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("blur_sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Linear,
        lod_min_clamp: 0.0,
        lod_max_clamp: 0.0,
        compare: None,
        anisotropy_clamp: 1,
        border_color: None,
    });

    // Bind the resources
    let source_view = source_texture
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());
    let layout = &*blur_pipeline.blur_fixed_layout;
    let offsets_weights_binding = wgpu::BufferBinding {
        buffer: &offsets_weights,
        offset: 0,
        size: NonZeroU64::new(30 * 4 * mem::size_of::<f32>() as u64),
    };
    let offsets_weights_binding = wgpu::BindingResource::Buffer(offsets_weights_binding);

    let filter_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("blur_fixed"),
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&source_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&blur_sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: offsets_weights_binding,
            },
        ],
    });

    // Run a render pass to apply the filter
    {
        let target_view = target_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let color_attachments = vec![Some(wgpu::RenderPassColorAttachment {
            view: &target_view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.0,
                }),
                store: wgpu::StoreOp::Store,
            },
        })];
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("blur_fixed"),
            depth_stencil_attachment: None,
            color_attachments: &color_attachments,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // Draw the vertices
        let vertex_size = mem::size_of::<Vertex2D>();
        let start_pos = (0 * vertex_size) as u64;
        let end_pos = (6 * vertex_size) as u64;

        render_pass.set_pipeline(&*blur_pipeline.pipeline);
        render_pass.set_bind_group(0, &filter_bind_group, &[]);
        render_pass.set_vertex_buffer(0, vertices.slice(start_pos..end_pos));
        render_pass.draw(0..6, 0..1);
    }

    // Result is the new texture
    WgpuTexture {
        descriptor: target_descriptor,
        texture: Arc::new(target_texture),
        is_premultiplied: source_texture.is_premultiplied,
    }
}

///
/// Runs one of the texture-sized blur filters on a source texture
///
pub(crate) fn blur_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    encoder: &mut wgpu::CommandEncoder,
    blur_pipeline: &Pipeline,
    source_texture: &WgpuTexture,
    weights: Vec<f32>,
    offsets: Vec<f32>,
) -> WgpuTexture {
    // Set up buffers
    let vertices = vec![
        Vertex2D::with_pos(-1.0, -1.0),
        Vertex2D::with_pos(-1.0, 1.0),
        Vertex2D::with_pos(1.0, 1.0),
        Vertex2D::with_pos(-1.0, -1.0),
        Vertex2D::with_pos(1.0, -1.0),
        Vertex2D::with_pos(1.0, 1.0),
    ]
    .to_buffer(device, wgpu::BufferUsages::VERTEX);

    let offset_factor = match blur_pipeline.blur_filter_direction() {
        BlurDirection::Horizontal => source_texture.descriptor.size.width as f32,
        BlurDirection::Vertical => source_texture.descriptor.size.height as f32,
    };
    let offsets = offsets
        .into_iter()
        .map(|offset| offset / offset_factor)
        .collect::<Vec<_>>();

    // Create a target texture
    let mut target_descriptor = source_texture.descriptor.clone();
    target_descriptor.usage |= wgpu::TextureUsages::RENDER_ATTACHMENT;
    let target_texture = device.create_texture(&target_descriptor);

    // Create the blur sampler
    let blur_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("blur_sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Linear,
        lod_min_clamp: 0.0,
        lod_max_clamp: 0.0,
        compare: None,
        anisotropy_clamp: 1,
        border_color: None,
    });

    // Create the weights and offset textures
    let weights_offsets_descriptor = wgpu::TextureDescriptor {
        label: Some("blur_texture"),
        size: wgpu::Extent3d {
            width: weights.len() as _,
            ..Default::default()
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D1,
        format: wgpu::TextureFormat::R32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    };

    let weights_texture =
        device.create_texture_with_data(queue, &weights_offsets_descriptor, weights.to_u8_slice());
    let offsets_texture =
        device.create_texture_with_data(queue, &weights_offsets_descriptor, offsets.to_u8_slice());

    // Bind the resources
    let source_view = source_texture
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());
    let weights_view = weights_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let offsets_view = offsets_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let layout = &*blur_pipeline.blur_texture_layout;

    let filter_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("blur_texture"),
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&source_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&blur_sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(&offsets_view),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(&weights_view),
            },
        ],
    });

    // Run a render pass to apply the filter
    {
        let target_view = target_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let color_attachments = vec![Some(wgpu::RenderPassColorAttachment {
            view: &target_view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.0,
                }),
                store: wgpu::StoreOp::Store,
            },
        })];
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("blur_texture"),
            depth_stencil_attachment: None,
            color_attachments: &color_attachments,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // Draw the vertices
        let vertex_size = mem::size_of::<Vertex2D>();
        let start_pos = (0 * vertex_size) as u64;
        let end_pos = (6 * vertex_size) as u64;

        render_pass.set_pipeline(&*blur_pipeline.pipeline);
        render_pass.set_bind_group(0, &filter_bind_group, &[]);
        render_pass.set_vertex_buffer(0, vertices.slice(start_pos..end_pos));
        render_pass.draw(0..6, 0..1);
    }

    // Result is the new texture
    WgpuTexture {
        descriptor: target_descriptor,
        texture: Arc::new(target_texture),
        is_premultiplied: source_texture.is_premultiplied,
    }
}
