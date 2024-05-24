use std::{
    collections::HashMap,
    num::{NonZeroU32, NonZeroU64},
};

use super::{
    assets::{texture::Texture, LoadedAssets},
    chunk::{RenderChunk, Vertex},
    mesher::MeshUpdate,
};

pub struct World {
    chunks: Vec<(glam::IVec3, RenderChunk)>,
}

impl World {
    pub fn new() -> Self {
        Self {
            chunks: Vec::default(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WorldUniform {
    camera: [[f32; 4]; 4],
}

pub struct WorldRenderer {
    global_bind_group: wgpu::BindGroup,
    chunk_bind_layout: wgpu::BindGroupLayout,

    main_pipeline: wgpu::RenderPipeline,
    world_uniform_buffer: wgpu::Buffer,
    world_uniform: WorldUniform,
    depth: Texture,

    world: World,
}

impl WorldRenderer {
    const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        assets: &LoadedAssets,
    ) -> Result<Self, image::ImageError> {
        let shader = unsafe {
            device.create_shader_module_spirv(&wgpu::include_spirv_raw!(env!("shaders.spv")))
        };

        let world_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("World Uniform"),
            size: core::mem::size_of::<WorldUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let world_uniform = WorldUniform {
            camera: glam::Mat4::IDENTITY.to_cols_array_2d(),
        };

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: NonZeroU32::new(assets.textures.len() as u32),
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: NonZeroU32::new(assets.textures.len() as u32),
                    },
                ],
                label: Some("diffuse_bind_group_layout"),
            });

        let chunk_bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: None,
        });

        let samplers: Vec<&wgpu::Sampler> = assets.textures.iter().map(|t| &t.sampler).collect();

        let texture_views: Vec<&wgpu::TextureView> =
            assets.textures.iter().map(|t| &t.view).collect();

        let global_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &world_uniform_buffer,
                        offset: 0,
                        size: NonZeroU64::new(world_uniform_buffer.size()),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureViewArray(&texture_views),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::SamplerArray(&samplers),
                },
            ],
            label: Some("global_bind_group"),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&texture_bind_group_layout, &chunk_bind_layout],
                push_constant_ranges: &[],
            });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "main_vs",
                buffers: &[Vertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions {
                    ..Default::default()
                },
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "main_fs",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions {
                    ..Default::default()
                },
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: Self::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less, // 1.
                stencil: wgpu::StencilState::default(),     // 2.
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let depth = Texture::new_depth(
            device,
            wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            Self::DEPTH_FORMAT,
        );

        Ok(Self {
            main_pipeline: pipeline,
            chunk_bind_layout,
            global_bind_group,
            world: World::new(),

            world_uniform,
            world_uniform_buffer,
            depth,
        })
    }

    pub fn set_camera(&mut self, camera: glam::Mat4) {
        self.world_uniform.camera = camera.to_cols_array_2d();
    }

    pub fn resize(&mut self, device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) {
        self.depth = Texture::new_depth(
            device,
            wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            Self::DEPTH_FORMAT,
        );
    }

    pub fn update_chunk(&mut self, device: &wgpu::Device, update: &MeshUpdate) {
        let pos = glam::IVec3::new(update.pos.x, update.pos.y, update.pos.z);
        let render_chunk = RenderChunk::from_vertex_index(
            device,
            &self.chunk_bind_layout,
            &update.vertices,
            &update.indices,
            pos,
        );

        self.world.chunks.push((pos, render_chunk));
    }

    pub fn update(&mut self, queue: &wgpu::Queue) {
        queue.write_buffer(
            &self.world_uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.world_uniform]),
        );
    }

    pub fn draw<'a>(&'a self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Main Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.4,
                        b: 0.8,
                        a: 0.3,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        render_pass.set_pipeline(&self.main_pipeline);
        render_pass.set_bind_group(0, &self.global_bind_group, &[]);

        for (_, chunk) in &self.world.chunks {
            render_pass.set_bind_group(1, &chunk.bind_group, &[]);

            render_pass.set_vertex_buffer(0, chunk.vertex_buffer.slice(..));
            render_pass.set_index_buffer(chunk.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..chunk.len, 0, 0..1);
        }
    }
}
