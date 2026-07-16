//! The viewport rack: the workstation's eye. Four minds share one canvas —
//! citadel (folded space), gyroid (twisted minimal surface), cortex (a
//! per-pixel neural field), and specter (the live camera pulled through
//! the phase pipeline). All of them breathe with the engine: RMS, beats,
//! and the 16-band spectrum ride the modulation bus. Each mind names its
//! own controls (same bus, true labels — customizability per item).

use crate::rack::{Fader, Led, RackPanel};
use leptos::prelude::*;

/// Shared viewport parameter block (base values; modulation stacks on top).
#[derive(Clone, Copy)]
pub struct CitadelParams {
    /// mod0 — structure / thickness / seed / folds (mind-named).
    pub scale: f32,
    /// mod1 — warp / twist / zoom / paint (mind-named).
    pub warp: f32,
    /// mod2 — palette phase, always.
    pub hue: f32,
    /// mod3 — dolly / speed / flow / zoom (mind-named).
    pub dolly: f32,
    /// Auto-explore engaged.
    pub auto: bool,
    /// Preset epoch (kept for compatibility with preset application).
    pub gen: u32,
}

impl Default for CitadelParams {
    fn default() -> Self {
        Self {
            scale: 0.45,
            warp: 0.5,
            hue: 0.2,
            dolly: 0.45,
            auto: true,
            gen: 0,
        }
    }
}

/// The selectable minds and their per-mind control names — every fader
/// tells the truth about what it does *for this visual*.
const MINDS: [(&str, &str, [&str; 4]); 4] = [
    ("citadel", "◬ citadel", ["scale", "warp", "hue", "dolly"]),
    ("gyroid", "▚ gyroid", ["thickness", "twist", "hue", "speed"]),
    ("cortex", "ψ cortex", ["seed", "zoom", "hue", "flow"]),
    ("specter", "☉ specter", ["folds", "paint", "hue", "zoom"]),
];

/// The viewport + control panels, as one rack row.
#[component]
pub fn CitadelRack(
    /// Lifted so presets can rewrite the whole viewport state.
    params: RwSignal<CitadelParams>,
) -> impl IntoView {
    let canvas_ref = NodeRef::<leptos::html::Canvas>::new();
    let error = RwSignal::new(None::<String>);
    let mind = RwSignal::new("citadel");

    #[cfg(target_arch = "wasm32")]
    {
        use phunction_gfx::{
            wgpu, FieldPhunctor, FrameInput, GfxContext, Phunctor as _, ShaderPhunctor,
        };
        use std::cell::Cell;
        use wgpu::CurrentSurfaceTexture as Cst;

        enum Vp {
            Shader(ShaderPhunctor),
            Field(FieldPhunctor),
        }

        let started = Cell::new(false);
        Effect::new(move |_| {
            let Some(canvas) = canvas_ref.get() else {
                return;
            };
            if started.replace(true) {
                return;
            }
            leptos::task::spawn_local(async move {
                let mut ctx = match GfxContext::from_canvas(canvas.clone()).await {
                    Ok(c) => {
                        crate::gfx_gate::mark_ready();
                        c
                    }
                    Err(e) => {
                        crate::gfx_gate::mark_ready();
                        error.set(Some(e.to_string()));
                        return;
                    }
                };
                let webgpu = ctx.backend() == "webgpu";
                let mut current = "citadel";
                let mut vp = Vp::Shader(ShaderPhunctor::new(&ctx, phunction_gfx::CITADEL_WGSL));
                let t0 = web_time::Instant::now();
                let mut last_beat = 0u64;
                let mut pulse = 0.0f32;

                crate::raf::raf_loop(move || {
                    if !canvas.is_connected() {
                        return false;
                    }
                    let want = mind.get_untracked();
                    if want != current {
                        current = want;
                        error.set(None);
                        vp = match want {
                            "gyroid" => {
                                Vp::Shader(ShaderPhunctor::new(&ctx, phunction_gfx::GYROID_WGSL))
                            }
                            "cortex" => {
                                Vp::Shader(ShaderPhunctor::new(&ctx, phunction_gfx::CORTEX_WGSL))
                            }
                            "specter" => {
                                if webgpu {
                                    crate::camera::request();
                                    Vp::Field(FieldPhunctor::new(&ctx, phunction_gfx::SPECTER_WGSL))
                                } else {
                                    error.set(Some(
                                        "specter needs WebGPU for the camera→GPU path; this browser fell back to WebGL2".into(),
                                    ));
                                    Vp::Shader(ShaderPhunctor::new(
                                        &ctx,
                                        phunction_gfx::CITADEL_WGSL,
                                    ))
                                }
                            }
                            _ => Vp::Shader(ShaderPhunctor::new(&ctx, phunction_gfx::CITADEL_WGSL)),
                        };
                    }

                    let dpr = web_sys::window().map_or(1.0, |w| w.device_pixel_ratio());
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let size = (
                        (f64::from(canvas.client_width()) * dpr * 0.6).max(1.0) as u32,
                        (f64::from(canvas.client_height()) * dpr * 0.6).max(1.0) as u32,
                    );
                    if size.0 == 0 || size.1 == 0 {
                        return true;
                    }
                    if (canvas.width(), canvas.height()) != size {
                        canvas.set_width(size.0);
                        canvas.set_height(size.1);
                    }
                    ctx.resize_if_needed(size);

                    // stream the camera into the field, if we're the specter
                    if let Vp::Field(f) = &mut vp {
                        if let Some(video) = crate::camera::video() {
                            let (vw, vh) = (video.video_width(), video.video_height());
                            if video.ready_state() >= 2 && vw > 0 {
                                f.ensure_size(&ctx, (vw, vh));
                                ctx.queue.copy_external_image_to_texture(
                                    &wgpu::CopyExternalImageSourceInfo {
                                        source: wgpu::ExternalImageSource::HTMLVideoElement(video),
                                        origin: wgpu::Origin2d::ZERO,
                                        flip_y: false,
                                    },
                                    wgpu::CopyExternalImageDestInfo {
                                        texture: f.texture(),
                                        mip_level: 0,
                                        origin: wgpu::Origin3d::ZERO,
                                        aspect: wgpu::TextureAspect::All,
                                        color_space: wgpu::PredefinedColorSpace::Srgb,
                                        premultiplied_alpha: false,
                                    },
                                    wgpu::Extent3d {
                                        width: vw,
                                        height: vh,
                                        depth_or_array_layers: 1,
                                    },
                                );
                            }
                        }
                    }

                    let frame = match ctx.surface.get_current_texture() {
                        Cst::Success(f) => f,
                        Cst::Suboptimal(f) => {
                            ctx.configure(ctx.size);
                            f
                        }
                        _ => {
                            ctx.configure(ctx.size);
                            return true;
                        }
                    };
                    let view = frame
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default());

                    let now = t0.elapsed().as_secs_f32();
                    let par = params.get_untracked();
                    let met = crate::phazor_panel::wiring::last_meter();

                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let beat_now = met.beats as u64;
                    if met.playing && beat_now != last_beat {
                        last_beat = beat_now;
                        pulse = 1.0;
                    }
                    pulse *= 0.94;

                    let (ds, dw, dd) = if par.auto {
                        (
                            0.16 * (now * 0.043).sin(),
                            0.18 * (now * 0.031 + 1.7).sin(),
                            0.12 * (now * 0.019 + 4.2).sin(),
                        )
                    } else {
                        (0.0, 0.0, 0.0)
                    };
                    let rms = (met.rms_l + met.rms_r) * 1.5;
                    let bands = met.bands;
                    #[allow(clippy::cast_precision_loss)]
                    let coarse = |a: usize, z: usize| -> f32 {
                        (bands[a..z].iter().sum::<f32>() / (z - a) as f32).min(1.0)
                    };

                    let input = FrameInput {
                        time: now,
                        aspect: size.0 as f32 / size.1 as f32,
                        mods: [
                            (par.scale + ds).clamp(0.0, 1.0),
                            (par.warp + dw + rms).clamp(0.0, 1.0),
                            par.hue + pulse * 0.12,
                            (par.dolly + dd).clamp(0.0, 1.0),
                            coarse(0, 4),
                            coarse(4, 8),
                            coarse(8, 12),
                            coarse(12, 16),
                        ],
                    };
                    match &mut vp {
                        Vp::Shader(s) => s.frame(&ctx, &view, &input),
                        Vp::Field(f) => f.frame(&ctx, &view, &input),
                    }
                    ctx.queue.present(frame);
                    true
                });
            });
        });
    }

    // fader labels follow the active mind — same bus, true names
    let labels = Memo::new(move |_| {
        MINDS
            .iter()
            .find(|(id, _, _)| *id == mind.get())
            .map_or(["scale", "warp", "hue", "dolly"], |(_, _, l)| *l)
    });

    view! {
        <RackPanel title="viewport · four minds" class="span7">
            <div class="vp-select">
                {MINDS
                    .map(|(id, label, _)| {
                        view! {
                            <button
                                class="xport vp"
                                class:lit=move || mind.get() == id
                                on:click=move |_| mind.set(id)
                            >
                                {label}
                            </button>
                        }
                    })}
            </div>
            {move || error.get().map(|e| view! { <p class="gfx-error">"✗ " {e}</p> })}
            <canvas
                node_ref=canvas_ref
                class="scene-canvas fractal-canvas"
                aria-label="the workstation viewport: fractal, gyroid, neural field, or your own kaleidoscoped camera"
            ></canvas>
        </RackPanel>
        <RackPanel title="mind controls" class="span5">
            {move || {
                let l = labels.get();
                view! {
                    <Fader label=l[0] init=0.45 hue=145.0
                        sync=Signal::derive(move || params.get().scale)
                        on_value=move |v: f32| params.update(|p| p.scale = v) />
                    <Fader label=l[1] init=0.5 hue=325.0
                        sync=Signal::derive(move || params.get().warp)
                        on_value=move |v: f32| params.update(|p| p.warp = v) />
                    <Fader label=l[2] init=0.2 hue=100.0
                        sync=Signal::derive(move || params.get().hue)
                        on_value=move |v: f32| params.update(|p| p.hue = v) />
                    <Fader label=l[3] init=0.45 hue=235.0
                        sync=Signal::derive(move || params.get().dolly)
                        on_value=move |v: f32| params.update(|p| p.dolly = v) />
                }
            }}
            <div class="fold-side">
                <button
                    class="xport"
                    class:lit=move || params.get().auto
                    on:click=move |_| params.update(|p| p.auto = !p.auto)
                >
                    "∞ explore"
                </button>
                <Led on={Signal::derive(move || params.get().auto)} hue=280.0 label="drift" />
            </div>
        </RackPanel>
    }
}
