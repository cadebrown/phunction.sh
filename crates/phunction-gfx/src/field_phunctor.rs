//! A phunctor with a live input field: fragment shader + one texture the
//! host streams into (camera frames, video, another phunctor's output).
//! This is the media half of "crazy multimedia synthesis": light in,
//! folded light out.

use crate::context::GfxContext;
use crate::phunctor::{FrameInput, Phunctor};

/// Uniform block — layout identical to [`crate::shader_phunctor`]'s.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    time: f32,
    aspect: f32,
    mods: [f32; 8],
    _pad: [f32; 2],
}

const PRELUDE: &str = include_str!("../shaders/prelude.wgsl");
/// Field bindings appended to the shared prelude.
const FIELD_BINDINGS: &str = "
@group(0) @binding(1) var field_tex: texture_2d<f32>;
@group(0) @binding(2) var field_samp: sampler;
";

/// A fullscreen shader with a streamed input texture.
pub struct FieldPhunctor {
    pipeline: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    bgl: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    bind_group: wgpu::BindGroup,
    texture: wgpu::Texture,
    size: (u32, u32),
}

impl FieldPhunctor {
    /// Compile `fragment_wgsl` over the prelude + field bindings.
    #[must_use]
    pub fn new(gfx: &GfxContext, fragment_wgsl: &str) -> Self {
        let source = format!("{PRELUDE}\n{FIELD_BINDINGS}\n{fragment_wgsl}");
        let module = gfx
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("field phunctor"),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });
        let uniforms = gfx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("field uniforms"),
            size: size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bgl = gfx
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
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
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        let sampler = gfx.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("field sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let size = (2, 2);
        let texture = Self::make_texture(gfx, size);
        let bind_group = Self::make_bind_group(gfx, &bgl, &uniforms, &texture, &sampler);
        let layout = gfx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[Some(&bgl)],
                immediate_size: 0,
            });
        let pipeline = gfx
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("field phunctor"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some("fs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(gfx.format.into())],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });
        Self {
            pipeline,
            uniforms,
            bgl,
            sampler,
            bind_group,
            texture,
            size,
        }
    }

    fn make_texture(gfx: &GfxContext, size: (u32, u32)) -> wgpu::Texture {
        gfx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("field input"),
            size: wgpu::Extent3d {
                width: size.0.max(1),
                height: size.1.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
    }

    fn make_bind_group(
        gfx: &GfxContext,
        bgl: &wgpu::BindGroupLayout,
        uniforms: &wgpu::Buffer,
        texture: &wgpu::Texture,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        gfx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniforms.as_entire_binding(),
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

    /// Resize the input texture if the stream's dimensions changed.
    pub fn ensure_size(&mut self, gfx: &GfxContext, size: (u32, u32)) {
        if size != self.size && size.0 > 0 && size.1 > 0 {
            self.size = size;
            self.texture = Self::make_texture(gfx, size);
            self.bind_group =
                Self::make_bind_group(gfx, &self.bgl, &self.uniforms, &self.texture, &self.sampler);
        }
    }

    /// The input texture the host streams into (`COPY_DST`).
    #[must_use]
    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }
}

impl Phunctor for FieldPhunctor {
    fn frame(&mut self, gfx: &GfxContext, view: &wgpu::TextureView, input: &FrameInput) {
        let u = Uniforms {
            time: input.time,
            aspect: input.aspect,
            mods: input.mods,
            _pad: [0.0; 2],
        };
        gfx.queue
            .write_buffer(&self.uniforms, 0, bytemuck::bytes_of(&u));
        let mut encoder = gfx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("field phunctor"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        gfx.queue.submit([encoder.finish()]);
    }
}
