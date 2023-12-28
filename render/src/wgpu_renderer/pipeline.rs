/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::pipeline_configuration::*;
use super::shader_cache::*;
use super::texture_settings::*;
use super::wgpu_shader::*;

use wgpu;

use std::mem;
use std::num::NonZeroU64;
use std::sync::*;

///
/// A render pipeline and its binding groups
///
pub(crate) struct Pipeline {
    /// The shader module for this pipeline
    pub(crate) shader_module: WgpuShader,

    /// Set to true if the coordinates should be flipped vertically
    pub(crate) flip_vertical: bool,

    /// The render pipeline
    pub(crate) pipeline: Arc<wgpu::RenderPipeline>,

    /// The bind group layout for the transformation matrix
    pub(crate) matrix_layout: Arc<wgpu::BindGroupLayout>,

    /// The bind group layout for the clip mask
    pub(crate) clip_mask_layout: Arc<wgpu::BindGroupLayout>,

    /// The bind group layout for the input texture
    pub(crate) texture_layout: Arc<wgpu::BindGroupLayout>,

    /// The bind group layout for a linear gradient
    pub(crate) linear_gradient_layout: Arc<wgpu::BindGroupLayout>,

    /// Bind group layout for the alpha blend filter
    pub(crate) alpha_blend_layout: Arc<wgpu::BindGroupLayout>,

    /// Bind group layout for the fixed kernel size gaussian blur filter
    pub(crate) blur_fixed_layout: Arc<wgpu::BindGroupLayout>,

    /// Bind group layout for the fixed kernel size gaussian blur filter
    pub(crate) blur_texture_layout: Arc<wgpu::BindGroupLayout>,

    /// Bind group layout for the mask filter
    pub(crate) mask_layout: Arc<wgpu::BindGroupLayout>,

    /// Bind group layout for the displacement map filter
    pub(crate) displacement_map_layout: Arc<wgpu::BindGroupLayout>,

    /// Bind group layout for the reduce filter
    pub(crate) reduce_layout: Arc<wgpu::BindGroupLayout>,
}

impl Pipeline {
    ///
    /// Creates a pipeline from a pipline configuration
    ///
    pub fn from_configuration(
        config: &PipelineConfiguration,
        device: &wgpu::Device,
        shader_cache: &mut ShaderCache<WgpuShader>,
    ) -> Pipeline {
        let mut temp_data = PipelineDescriptorTempStorage::default();

        let matrix_bind_layout = config.matrix_bind_group_layout();
        let clip_bind_layout = config.clip_mask_bind_group_layout();
        let texture_layout = config.texture_bind_group_layout();
        let linear_gradient_layout = config.linear_gradient_bind_group_layout();
        let matrix_bind_layout = device.create_bind_group_layout(&matrix_bind_layout);
        let clip_bind_layout = device.create_bind_group_layout(&clip_bind_layout);
        let texture_layout = device.create_bind_group_layout(&texture_layout);
        let linear_gradient_layout = device.create_bind_group_layout(&linear_gradient_layout);

        let alpha_blend_layout = config.filter_alpha_blend_bind_group_layout();
        let alpha_blend_layout = device.create_bind_group_layout(&alpha_blend_layout);
        let blur_fixed_layout = config.filter_fixed_blur_bind_group_layout();
        let blur_fixed_layout = device.create_bind_group_layout(&blur_fixed_layout);
        let blur_texture_layout = config.filter_texture_blur_bind_group_layout();
        let blur_texture_layout = device.create_bind_group_layout(&blur_texture_layout);

        let mask_layout = config.filter_mask_bind_group_layout();
        let mask_layout = device.create_bind_group_layout(&mask_layout);
        let displacement_map_layout = config.filter_displacement_map_bind_group_layout();
        let displacement_map_layout = device.create_bind_group_layout(&displacement_map_layout);
        let reduce_layout = config.filter_reduce_bind_group_layout();
        let reduce_layout = device.create_bind_group_layout(&reduce_layout);

        let bind_layout = match config.shader_module {
            WgpuShader::LinearGradient(..) => vec![
                &matrix_bind_layout,
                &clip_bind_layout,
                &linear_gradient_layout,
            ],
            WgpuShader::Texture(..) => {
                vec![&matrix_bind_layout, &clip_bind_layout, &texture_layout]
            }
            WgpuShader::Simple(..) => vec![&matrix_bind_layout, &clip_bind_layout],
            WgpuShader::Filter(FilterShader::AlphaBlend(..)) => vec![&alpha_blend_layout],
            WgpuShader::Filter(FilterShader::BlurFixed(..)) => vec![&blur_fixed_layout],
            WgpuShader::Filter(FilterShader::BlurTexture(..)) => vec![&blur_texture_layout],
            WgpuShader::Filter(FilterShader::Mask(..)) => vec![&mask_layout],
            WgpuShader::Filter(FilterShader::DisplacementMap) => vec![&displacement_map_layout],
            WgpuShader::Filter(FilterShader::Reduce) => vec![&reduce_layout],
        };
        let pipeline_layout = wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline::from_configuration"),
            bind_group_layouts: &bind_layout,
            push_constant_ranges: &[],
        };
        let pipeline_layout = device.create_pipeline_layout(&pipeline_layout);

        let descriptor =
            config.render_pipeline_descriptor(shader_cache, &pipeline_layout, &mut temp_data);
        let new_pipeline = device.create_render_pipeline(&descriptor);

        Pipeline {
            shader_module: config.shader_module.clone(),
            flip_vertical: config.flip_vertical,
            pipeline: Arc::new(new_pipeline),
            matrix_layout: Arc::new(matrix_bind_layout),
            clip_mask_layout: Arc::new(clip_bind_layout),
            texture_layout: Arc::new(texture_layout),
            linear_gradient_layout: Arc::new(linear_gradient_layout),
            alpha_blend_layout: Arc::new(alpha_blend_layout),
            blur_fixed_layout: Arc::new(blur_fixed_layout),
            blur_texture_layout: Arc::new(blur_texture_layout),
            mask_layout: Arc::new(mask_layout),
            displacement_map_layout: Arc::new(displacement_map_layout),
            reduce_layout: Arc::new(reduce_layout),
        }
    }

    ///
    /// Returns the index of the matrix binding group
    ///
    #[inline]
    pub fn matrix_group_index(&self) -> u32 {
        0
    }

    ///
    /// Returns the index of the clip mask binding group
    ///
    #[inline]
    pub fn clip_mask_group_index(&self) -> u32 {
        1
    }

    ///
    /// Returns the index of the texture binding group
    ///
    #[inline]
    pub fn input_texture_group_index(&self) -> u32 {
        2
    }

    ///
    /// If this pipeline is using a blur filter, returns the direction of that filter
    ///
    #[inline]
    pub fn blur_filter_direction(&self) -> BlurDirection {
        match self.shader_module {
            WgpuShader::Filter(FilterShader::BlurFixed(direction, ..)) => direction,
            WgpuShader::Filter(FilterShader::BlurTexture(direction)) => direction,

            _ => BlurDirection::Horizontal,
        }
    }

    ///
    /// Binds the transformation matrix buffer for this pipeline (filling in or replacing the `matrix_binding` entry)
    ///
    #[inline]
    pub fn bind_matrix_buffer(
        &self,
        device: &wgpu::Device,
        matrix_buffer: &wgpu::Buffer,
        offset: usize,
    ) -> wgpu::BindGroup {
        let buffer_binding = wgpu::BufferBinding {
            buffer: matrix_buffer,
            offset: offset as u64,
            size: NonZeroU64::new(mem::size_of::<[[f32; 4]; 4]>() as u64),
        };
        let buffer_binding = wgpu::BindingResource::Buffer(buffer_binding);

        let matrix_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bind_matrix_buffer"),
            layout: &*self.matrix_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer_binding,
            }],
        });

        matrix_bind_group
    }

    ///
    /// Creates the clip mask binding group for this pipeline configuration
    ///
    /// This is stored in bind group 1. The clip texture must be supplied for a valid bind group to be generated if the shader is using the clipping mask
    /// (it's optional because it's not otherwise required)
    ///
    pub fn bind_clip_mask(
        &self,
        device: &wgpu::Device,
        clip_texture: Option<&wgpu::Texture>,
    ) -> wgpu::BindGroup {
        match (&self.shader_module, clip_texture) {
            (
                WgpuShader::LinearGradient(StandardShaderVariant::ClippingMask, _, _, _),
                Some(clip_texture),
            )
            | (
                WgpuShader::Texture(StandardShaderVariant::ClippingMask, _, _, _, _),
                Some(clip_texture),
            )
            | (WgpuShader::Simple(StandardShaderVariant::ClippingMask, _), Some(clip_texture)) => {
                // Create a view of the texture
                let view = clip_texture.create_view(&wgpu::TextureViewDescriptor::default());

                // Bind to group 1
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("create_clip_mask_bind_group_with_texture"),
                    layout: &*self.clip_mask_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    }],
                })
            }

            (_, None)
            | (WgpuShader::Filter(_), _)
            | (WgpuShader::LinearGradient(StandardShaderVariant::NoClipping, _, _, _), _)
            | (WgpuShader::Texture(StandardShaderVariant::NoClipping, _, _, _, _), _)
            | (WgpuShader::Simple(StandardShaderVariant::NoClipping, _), _) => {
                // Group 1 is bound to an empty set if clipping is off or no texture is defined
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("create_clip_mask_bind_group_no_texture"),
                    layout: &*self.clip_mask_layout,
                    entries: &[],
                })
            }
        }
    }

    ///
    /// Creates the texture binding for the current shader
    ///
    pub fn bind_input_texture(
        &self,
        device: &wgpu::Device,
        texture_settings: &wgpu::Buffer,
        texture_settings_offset: usize,
        texture: Option<&wgpu::Texture>,
        sampler: Option<&wgpu::Sampler>,
    ) -> wgpu::BindGroup {
        let texture_settings_binding = wgpu::BufferBinding {
            buffer: texture_settings,
            offset: texture_settings_offset as u64,
            size: NonZeroU64::new(mem::size_of::<TextureSettings>() as u64),
        };
        let texture_settings_binding = wgpu::BindingResource::Buffer(texture_settings_binding);

        match (self.shader_module, texture, sampler) {
            (WgpuShader::LinearGradient(..), Some(texture), Some(sampler)) => {
                // Create a view of the texture
                let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

                // Bind to group 2
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("bind_input_linear_gradient_sampler"),
                    layout: &*self.linear_gradient_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: texture_settings_binding,
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::Sampler(sampler),
                        },
                    ],
                })
            }

            (
                WgpuShader::Texture(_, InputTextureType::Sampler, _, _, _),
                Some(texture),
                Some(sampler),
            ) => {
                // Create a view of the texture
                let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

                // Bind to group 2
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("bind_input_texture_sampler"),
                    layout: &*self.texture_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: texture_settings_binding,
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::Sampler(sampler),
                        },
                    ],
                })
            }

            (WgpuShader::Texture(_, InputTextureType::Multisampled, _, _, _), Some(texture), _) => {
                // Create a view of the texture
                let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

                // Bind to group 2
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("bind_input_texture_multisampled"),
                    layout: &*self.texture_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: texture_settings_binding,
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&view),
                        },
                    ],
                })
            }

            (WgpuShader::LinearGradient(..), _, None)
            | (WgpuShader::Texture(_, InputTextureType::Sampler, ..), _, None) => {
                // Group 2 is bound to an empty set if no texture is defined (or the sampler is missing when it was expected)
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("bind_input_texture_no_sampler"),
                    layout: &*self.clip_mask_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: texture_settings_binding,
                    }],
                })
            }

            (_, None, _) | (WgpuShader::Filter(_), _, _) | (WgpuShader::Simple(..), _, _) => {
                // Group 2 is bound to an empty set if not using a texture shader
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("bind_input_texture_not_texture_shader"),
                    layout: &*self.clip_mask_layout,
                    entries: &[],
                })
            }
        }
    }
}
