//! A minimal 3D scene phunctor: the icosahedron (VISION §II, "3D worlds").
//!
//! The solid is literally built from φ (its vertices are golden-ratio
//! rectangles), faces wear the hue of their normal's azimuth — *color is
//! phase* extended to 3D — and the camera orbit rides the modulation bus
//! (`mods[0]` = yaw, `mods[1]` = pitch), so any control surface can fly it.

use crate::context::GfxContext;
use crate::phunctor::{FrameInput, Phunctor};

/// One vertex: position + flat face normal.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    pos: [f32; 3],
    normal: [f32; 3],
}

/// Uniforms: model-view-projection + time.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    mvp: [[f32; 4]; 4],
    time: f32,
    _pad: [f32; 3],
}

const SHADER: &str = r"
struct U { mvp: mat4x4<f32>, time: f32 }
@group(0) @binding(0) var<uniform> u: U;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) normal: vec3<f32>,
}

@vertex
fn vs_main(@location(0) pos: vec3<f32>, @location(1) normal: vec3<f32>) -> VsOut {
    var out: VsOut;
    out.pos = u.mvp * vec4<f32>(pos, 1.0);
    out.normal = normal;
    return out;
}

const TAU: f32 = 6.28318530718;

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    // hue = azimuth of the face normal: color is phase, in 3D
    let hue = atan2(in.normal.y, in.normal.x) / TAU + 0.5 + u.time * 0.02;
    let base = 0.5 + 0.5 * cos(TAU * (vec3<f32>(hue) + vec3<f32>(0.0, 0.33, 0.67)));
    // one light, top-left-front, plus a floor of ambient
    let l = normalize(vec3<f32>(-0.5, 0.8, 0.6));
    let lit = 0.25 + 0.75 * max(dot(normalize(in.normal), l), 0.0);
    return vec4<f32>(base * lit, 1.0);
}
";

/// Icosahedron with flat normals (60 vertices, 20 faces), edge-scaled to
/// fit a unit-ish sphere.
const FACES: [[usize; 3]; 20] = [
    [0, 11, 5],
    [0, 5, 1],
    [0, 1, 7],
    [0, 7, 10],
    [0, 10, 11],
    [1, 5, 9],
    [5, 11, 4],
    [11, 10, 2],
    [10, 7, 6],
    [7, 1, 8],
    [3, 9, 4],
    [3, 4, 2],
    [3, 2, 6],
    [3, 6, 8],
    [3, 8, 9],
    [4, 9, 5],
    [2, 4, 11],
    [6, 2, 10],
    [8, 6, 7],
    [9, 8, 1],
];

fn icosahedron() -> Vec<Vertex> {
    let phi = f32::midpoint(1.0, 5.0f32.sqrt()); // φ, structurally
    let raw: [[f32; 3]; 12] = [
        [-1.0, phi, 0.0],
        [1.0, phi, 0.0],
        [-1.0, -phi, 0.0],
        [1.0, -phi, 0.0],
        [0.0, -1.0, phi],
        [0.0, 1.0, phi],
        [0.0, -1.0, -phi],
        [0.0, 1.0, -phi],
        [phi, 0.0, -1.0],
        [phi, 0.0, 1.0],
        [-phi, 0.0, -1.0],
        [-phi, 0.0, 1.0],
    ];
    let scale = 1.0 / phi.mul_add(phi, 1.0).sqrt();
    let vert = |i: usize| glam::Vec3::from(raw[i]) * scale;
    let mut out = Vec::with_capacity(60);
    for f in FACES {
        let (a, b, c) = (vert(f[0]), vert(f[1]), vert(f[2]));
        let n = (b - a).cross(c - a).normalize();
        for pos in [a, b, c] {
            out.push(Vertex {
                pos: pos.to_array(),
                normal: n.to_array(),
            });
        }
    }
    out
}

/// The scene renderer. Owns its depth buffer; recreates it on resize.
pub struct Scene3d {
    pipeline: wgpu::RenderPipeline,
    vertices: wgpu::Buffer,
    vertex_count: u32,
    uniforms: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    depth: Option<(wgpu::TextureView, (u32, u32))>,
}

impl Scene3d {
    /// Build the pipeline on `gfx`.
    #[must_use]
    pub fn new(gfx: &GfxContext) -> Self {
        use wgpu::util::DeviceExt as _;
        let module = gfx
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("scene3d"),
                source: wgpu::ShaderSource::Wgsl(SHADER.into()),
            });
        let mesh = icosahedron();
        let vertices = gfx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("icosahedron"),
                contents: bytemuck::cast_slice(&mesh),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let uniforms = gfx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scene3d uniforms"),
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
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
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
                label: Some("scene3d"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: size_of::<Vertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
                    })],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some("fs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(gfx.format.into())],
                }),
                primitive: wgpu::PrimitiveState {
                    cull_mode: Some(wgpu::Face::Back),
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::Less),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });
        Self {
            pipeline,
            vertices,
            vertex_count: 60,
            uniforms,
            bind_group,
            depth: None,
        }
    }

    fn depth_view(&mut self, gfx: &GfxContext, size: (u32, u32)) -> &wgpu::TextureView {
        if self.depth.as_ref().is_none_or(|(_, s)| *s != size) {
            let tex = gfx.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("scene3d depth"),
                size: wgpu::Extent3d {
                    width: size.0.max(1),
                    height: size.1.max(1),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            self.depth = Some((
                tex.create_view(&wgpu::TextureViewDescriptor::default()),
                size,
            ));
        }
        &self.depth.as_ref().expect("depth just created").0
    }
}

impl Phunctor for Scene3d {
    fn frame(&mut self, gfx: &GfxContext, view: &wgpu::TextureView, input: &FrameInput) {
        // camera orbit from the bus: mod0 = yaw, mod1 = pitch (both 0..1)
        let yaw = (f64::from(input.mods[0]) * core::f64::consts::TAU) as f32;
        let pitch = (f64::from(input.mods[1]) - 0.5) as f32 * 2.4;
        let eye = glam::Vec3::new(
            yaw.cos() * pitch.cos() * 2.6,
            pitch.sin() * 2.6,
            yaw.sin() * pitch.cos() * 2.6,
        );
        let vp = glam::Mat4::perspective_rh(0.9, input.aspect, 0.1, 20.0)
            * glam::Mat4::look_at_rh(eye, glam::Vec3::ZERO, glam::Vec3::Y);
        let model = glam::Mat4::from_rotation_y(input.time * 0.25);
        let u = Uniforms {
            mvp: (vp * model).to_cols_array_2d(),
            time: input.time,
            _pad: [0.0; 3],
        };
        gfx.queue
            .write_buffer(&self.uniforms, 0, bytemuck::bytes_of(&u));

        let size = gfx.size;
        let depth = self.depth_view(gfx, size).clone();
        let mut encoder = gfx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene3d"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.02,
                            g: 0.015,
                            b: 0.045,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_vertex_buffer(0, self.vertices.slice(..));
            pass.draw(0..self.vertex_count, 0..1);
        }
        gfx.queue.submit([encoder.finish()]);
    }
}
