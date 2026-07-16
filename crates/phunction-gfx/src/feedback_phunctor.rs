//! A phunctor with memory: two ping-pong textures and a simulation pass.
//! Each frame the sim fragment reads state A and writes state B (a
//! reaction-diffusion step, a cellular rule, anything local), then the
//! present fragment maps B to the screen through the palette. This is the
//! feedback infrastructure lenia-like minds need — state that persists
//! across frames, entirely on the GPU.

use crate::context::GfxContext;
use crate::phunctor::{FrameInput, Phunctor};

/// Fixed simulation resolution: dynamics stay stable and cheap no matter
/// the canvas size; the present pass upscales with linear filtering.
pub const SIM_W: u32 = 512;
/// See [`SIM_W`].
pub const SIM_H: u32 = 288;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    time: f32,
    aspect: f32,
    mods: [f32; 8],
    _pad: [f32; 2],
}

const PRELUDE: &str = include_str!("../shaders/prelude.wgsl");
const FEEDBACK_BINDINGS: &str = "
@group(0) @binding(1) var state_tex: texture_2d<f32>;
@group(0) @binding(2) var state_samp: sampler;
";

/// A double-buffered simulation phunctor: `sim` advances state, `present`
/// draws it.
pub struct FeedbackPhunctor {
    sim_pipeline: wgpu::RenderPipeline,
    present_pipeline: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    textures: [wgpu::Texture; 2],
    bind_groups: [wgpu::BindGroup; 2],
    /// Which texture holds the CURRENT state (the sim reads it).
    front: usize,
}

impl FeedbackPhunctor {
    /// Compile the simulation and present fragments over the prelude.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn new(gfx: &GfxContext, sim_wgsl: &str, present_wgsl: &str) -> Self {
        let make_module = |src: &str| {
            gfx.device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("feedback"),
                    source: wgpu::ShaderSource::Wgsl(
                        format!("{PRELUDE}\n{FEEDBACK_BINDINGS}\n{src}").into(),
                    ),
                })
        };
        let sim_module = make_module(sim_wgsl);
        let present_module = make_module(present_wgsl);

        let uniforms = gfx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("feedback uniforms"),
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
            label: Some("feedback sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let make_texture = || {
            gfx.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("feedback state"),
                size: wgpu::Extent3d {
                    width: SIM_W,
                    height: SIM_H,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                // 16-bit float: reaction-diffusion needs the precision
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
        };
        let textures = [make_texture(), make_texture()];
        let make_bind = |tex: &wgpu::Texture| {
            let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
            gfx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &bgl,
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
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            })
        };
        let bind_groups = [make_bind(&textures[0]), make_bind(&textures[1])];

        let layout = gfx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[Some(&bgl)],
                immediate_size: 0,
            });
        let make_pipeline = |module: &wgpu::ShaderModule, format: wgpu::TextureFormat| {
            gfx.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("feedback"),
                    layout: Some(&layout),
                    vertex: wgpu::VertexState {
                        module,
                        entry_point: Some("vs_main"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        buffers: &[],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module,
                        entry_point: Some("fs_main"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        targets: &[Some(format.into())],
                    }),
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState::default(),
                    multiview_mask: None,
                    cache: None,
                })
        };
        let sim_pipeline = make_pipeline(&sim_module, wgpu::TextureFormat::Rgba16Float);
        let present_pipeline = make_pipeline(&present_module, gfx.format);

        Self {
            sim_pipeline,
            present_pipeline,
            uniforms,
            textures,
            bind_groups,
            front: 0,
        }
    }
}

impl Phunctor for FeedbackPhunctor {
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
        // sim: read front, write back
        let back = 1 - self.front;
        let back_view = self.textures[back].create_view(&wgpu::TextureViewDescriptor::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("feedback sim"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &back_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.sim_pipeline);
            pass.set_bind_group(0, &self.bind_groups[self.front], &[]);
            pass.draw(0..3, 0..1);
        }
        // present: read the fresh state to the screen
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("feedback present"),
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
            pass.set_pipeline(&self.present_pipeline);
            pass.set_bind_group(0, &self.bind_groups[back], &[]);
            pass.draw(0..3, 0..1);
        }
        gfx.queue.submit([encoder.finish()]);
        self.front = back;
    }
}
