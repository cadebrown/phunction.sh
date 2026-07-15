//! A phunctor defined entirely by one WGSL fragment shader — the workhorse.
//! Most visuals are "fullscreen triangle + math"; this hosts exactly that,
//! and is the seed of the live-coding shader editor (naga validates user
//! WGSL before it ever reaches the GPU).

use crate::context::GfxContext;
use crate::phunctor::{FrameInput, Phunctor};

/// Uniforms shared by every shader phunctor. Layout mirrors the WGSL
/// `struct U` in `shaders/prelude.wgsl` — keep the two in sync by hand (a
/// build-time check lands with the shader editor).
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    time: f32,
    aspect: f32,
    mod0: f32,
    mod1: f32,
    mod2: f32,
    mod3: f32,
    _pad: [f32; 2],
}

/// Vertex stage + uniform declaration shared by all shader phunctors; each
/// phunctor appends a `fs_main`.
const PRELUDE: &str = include_str!("../shaders/prelude.wgsl");

/// One fullscreen-shader visual.
pub struct ShaderPhunctor {
    pipeline: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl ShaderPhunctor {
    /// Compile `fragment_wgsl` (appended to the shared prelude) into a
    /// ready-to-draw pipeline.
    #[must_use]
    pub fn new(gfx: &GfxContext, fragment_wgsl: &str) -> Self {
        let source = format!("{PRELUDE}\n{fragment_wgsl}");
        let module = gfx
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("phunctor"),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });

        let uniforms = gfx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("phunctor uniforms"),
            size: size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bgl = gfx
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let bind_group = gfx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            }],
        });

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
                label: Some("phunctor"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
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
            bind_group,
        }
    }
}

impl Phunctor for ShaderPhunctor {
    fn frame(&mut self, gfx: &GfxContext, view: &wgpu::TextureView, input: &FrameInput) {
        let u = Uniforms {
            time: input.time,
            aspect: input.aspect,
            mod0: input.mods[0],
            mod1: input.mods[1],
            mod2: input.mods[2],
            mod3: input.mods[3],
            _pad: [0.0; 2],
        };
        gfx.queue
            .write_buffer(&self.uniforms, 0, bytemuck::bytes_of(&u));

        let mut encoder = gfx.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("phunctor"),
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
