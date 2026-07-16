//! The citadel rack: generative fractal geometry in the phazor
//! workstation, synced to the engine (VISION §II, cranked).
//!
//! Sliders drive the fold parameters; **auto-explore** walks them through
//! known-interesting regions on slow incommensurate sines (deterministic
//! wandering — no RNG, so a set is reproducible); the music reaches in
//! live: RMS pushes the warp, and every beat kicks a palette-phase pulse
//! that decays like a plucked string.

use crate::rack::{Fader, Led, RackPanel};
use leptos::prelude::*;

/// Shared fractal parameter block (base values; modulation stacks on top).
#[derive(Clone, Copy)]
pub struct CitadelParams {
    /// Fold scale `0..=1` (structure: cathedral ↔ dust).
    pub scale: f32,
    /// Warp `0..=1` (plane-fold twist; RMS adds on top).
    pub warp: f32,
    /// Palette phase `0..=1` (beats pulse this).
    pub hue: f32,
    /// Camera dolly `0..=1`.
    pub dolly: f32,
    /// Auto-explore engaged.
    pub auto: bool,
    /// Bumped by presets: remounts the fader bank so caps jump to the
    /// new truth (controls must never lie about the state they control).
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

/// The viewport + control panels, as one rack row.
#[component]
pub fn CitadelRack(
    /// Lifted so presets can rewrite the whole fractal state.
    params: RwSignal<CitadelParams>,
) -> impl IntoView {
    let canvas_ref = NodeRef::<leptos::html::Canvas>::new();
    let error = RwSignal::new(None::<String>);

    #[cfg(target_arch = "wasm32")]
    {
        use phunction_gfx::wgpu::CurrentSurfaceTexture as Cst;
        use phunction_gfx::{FrameInput, GfxContext, Phunctor as _, ShaderPhunctor};
        use std::cell::Cell;
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
                    Ok(c) => c,
                    Err(e) => {
                        error.set(Some(e.to_string()));
                        return;
                    }
                };
                let mut fractal = ShaderPhunctor::new(&ctx, phunction_gfx::CITADEL_WGSL);
                let t0 = web_time::Instant::now();
                // beat-pulse state: last integer beat + a decaying envelope
                let mut last_beat = 0u64;
                let mut pulse = 0.0f32;
                crate::raf::raf_loop(move || {
                    if !canvas.is_connected() {
                        return false;
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
                        .create_view(&phunction_gfx::wgpu::TextureViewDescriptor::default());

                    let t = t0.elapsed().as_secs_f32();
                    let p = params.get_untracked();
                    let m = crate::phazor_panel::wiring::last_meter();

                    // the beat kicks; the pulse rings down
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let beat_now = m.beats as u64;
                    if m.playing && beat_now != last_beat {
                        last_beat = beat_now;
                        pulse = 1.0;
                    }
                    pulse *= 0.94;

                    // auto-explore: incommensurate sines wander the folds
                    let (ds, dw, dd) = if p.auto {
                        (
                            0.16 * (t * 0.043).sin(),
                            0.18 * (t * 0.031 + 1.7).sin(),
                            0.12 * (t * 0.019 + 4.2).sin(),
                        )
                    } else {
                        (0.0, 0.0, 0.0)
                    };
                    let rms = (m.rms_l + m.rms_r) * 1.5;

                    // coarse spectrum → mods 4..7 (bass, low-mid, high-mid, air)
                    let b = m.bands;
                    let coarse = |a: usize, z: usize| -> f32 {
                        (b[a..z].iter().sum::<f32>() / (z - a) as f32).min(1.0)
                    };
                    let input = FrameInput {
                        time: t,
                        aspect: size.0 as f32 / size.1 as f32,
                        mods: [
                            (p.scale + ds).clamp(0.0, 1.0),
                            (p.warp + dw + rms).clamp(0.0, 1.0),
                            p.hue + pulse * 0.12,
                            (p.dolly + dd).clamp(0.0, 1.0),
                            coarse(0, 4),
                            coarse(4, 8),
                            coarse(8, 12),
                            coarse(12, 16),
                        ],
                    };
                    fractal.frame(&ctx, &view, &input);
                    ctx.queue.present(frame);
                    true
                });
            });
        });
    }

    view! {
        <RackPanel title="citadel · folded space" class="span7">
            {move || error.get().map(|e| view! { <p class="gfx-error">"✗ " {e}</p> })}
            <canvas
                node_ref=canvas_ref
                class="scene-canvas fractal-canvas"
                aria-label="a kaleidoscopic fractal citadel, breathing with the music"
            ></canvas>
        </RackPanel>
        <RackPanel title="fold controls" class="span5">
            <Fader label="scale" init=0.45 hue=145.0
                sync=Signal::derive(move || params.get().scale)
                on_value=move |v: f32| params.update(|p| p.scale = v) />
            <Fader label="warp" init=0.5 hue=325.0
                sync=Signal::derive(move || params.get().warp)
                on_value=move |v: f32| params.update(|p| p.warp = v) />
            <Fader label="hue" init=0.2 hue=100.0
                sync=Signal::derive(move || params.get().hue)
                on_value=move |v: f32| params.update(|p| p.hue = v) />
            <Fader label="dolly" init=0.45 hue=235.0
                sync=Signal::derive(move || params.get().dolly)
                on_value=move |v: f32| params.update(|p| p.dolly = v) />
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
