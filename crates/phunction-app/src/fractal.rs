//! The viewport rack: the workstation's eye. Four minds share one canvas —
//! citadel (folded space), gyroid (twisted minimal surface), cortex (a
//! per-pixel neural field), and specter (the live camera pulled through
//! the phase pipeline). All of them breathe with the engine: RMS, beats,
//! and the 16-band spectrum ride the modulation bus. Each mind names its
//! own controls (same bus, true labels — customizability per item).

use crate::rack::{Fader, Led, RackPanel};
use leptos::prelude::*;

/// The live-wgsl starter: enough to see the mods breathe, small enough
/// to read whole. The prelude (U uniforms, palette, TAU, hash21) is
/// prepended automatically — authors write only the fragment.
pub const WGSL_STARTER: &str = "@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let p = in.uv * vec2<f32>(u.aspect, 1.0);
    let r = length(p);
    let a = atan2(p.y, p.x) / TAU;
    let v = 0.5 + 0.5 * sin(r * 9.0 - u.time * 0.4 + a * 6.0 + u.mod0 * 6.0);
    let col = palette(v * 0.5 + u.mod2 + u.time * 0.01, vec3<f32>(0.0)) * (0.25 + 0.75 * v);
    return vec4<f32>(col, 1.0);
}
";

#[cfg(target_arch = "wasm32")]
thread_local! {
    /// A pending mind switch from outside the component (worlds/presets).
    static REQUEST: std::cell::Cell<Option<&'static str>> =
        const { std::cell::Cell::new(None) };
    /// The live-wgsl source of truth (persisted as phazor:wgsl).
    static WGSL_SRC: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
    /// Source changed: rebuild the pipeline next frame.
    static WGSL_DIRTY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// Last compile error, for the pane's status line.
    static WGSL_ERR: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
    /// A validated candidate approved by the async error scope.
    static WGSL_APPROVED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// The candidate pipeline awaiting validation.
    static WGSL_PENDING: std::cell::RefCell<Option<phunction_gfx::ShaderPhunctor>> =
        const { std::cell::RefCell::new(None) };
    /// Build generation, so a stale approval can't swap a newer candidate.
    static WGSL_GEN: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// Submit new live-wgsl source (compiled next frame, kept only if valid).
#[cfg(target_arch = "wasm32")]
pub fn submit_wgsl(src: &str) {
    WGSL_SRC.with(|s| src.clone_into(&mut s.borrow_mut()));
    WGSL_DIRTY.with(|d| d.set(true));
    crate::phazor_panel::wiring::save_state("phazor:wgsl", src);
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub fn submit_wgsl(_src: &str) {}

/// Ask the viewport to switch minds (applied on the next frame).
#[cfg(target_arch = "wasm32")]
pub fn request_mind(id: &'static str) {
    REQUEST.with(|r| r.set(Some(id)));
}

#[cfg(target_arch = "wasm32")]
thread_local! {
    /// The patch's routed field source (`mind.field` on the board), if any.
    static ROUTED_FIELD: std::cell::Cell<Option<u32>> = const { std::cell::Cell::new(None) };
}

/// The patchbay routes (or unroutes) a field into the room.
#[cfg(target_arch = "wasm32")]
pub fn route_field(id: Option<u32>) {
    ROUTED_FIELD.with(|r| r.set(id.filter(|id| *id != 0)));
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub fn route_field(_id: Option<u32>) {}

#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub fn request_mind(_id: &'static str) {}

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
pub const MINDS: [(&str, &str, [&str; 4]); 15] = [
    ("silk", "silk", ["depth", "grain", "hue", "drift"]),
    ("current", "current", ["flow", "eddies", "hue", "drift"]),
    ("petri", "petri", ["feed", "kill", "hue", "speed"]),
    ("wgsl", "wgsl", ["a", "b", "hue", "c"]),
    ("citadel", "citadel", ["scale", "warp", "hue", "dolly"]),
    ("gyroid", "gyroid", ["thickness", "twist", "hue", "speed"]),
    ("basilica", "basilica", ["scale", "fold", "hue", "orbit"]),
    ("gasket", "gasket", ["ratio", "zoom", "hue", "drift"]),
    ("cortex", "cortex", ["seed", "zoom", "hue", "flow"]),
    ("specter", "specter", ["folds", "paint", "hue", "zoom"]),
    ("maw", "maw", ["vault", "ceiling", "hue", "glide"]),
    ("bulb", "bulb", ["power", "detail", "hue", "orbit"]),
    ("indra", "indra", ["knot", "weave", "hue", "drift"]),
    ("hopf", "hopf", ["fibers", "precess", "hue", "orbit"]),
    ("lenia", "lenia", ["mood", "width", "hue", "speed"]),
];

/// The viewport + control panels, as one rack row.
#[component]
pub fn CitadelRack(
    /// Lifted so presets can rewrite the whole viewport state.
    params: RwSignal<CitadelParams>,
    /// Lifted so chrome (the topbar) can read AND light the active mind.
    mind: RwSignal<&'static str>,
) -> impl IntoView {
    let canvas_ref = NodeRef::<leptos::html::Canvas>::new();
    let error = RwSignal::new(None::<String>);
    let wgsl_status = RwSignal::new(None::<String>);
    let wgsl_source = RwSignal::new(
        crate::phazor_panel::wiring::load_state("phazor:wgsl")
            .unwrap_or_else(|| WGSL_STARTER.to_string()),
    );
    if let Some(saved) = crate::phazor_panel::wiring::load_state("phazor:mind").and_then(|m| {
        MINDS
            .iter()
            .find(|(id, _, _)| *id == m)
            .map(|(id, _, _)| *id)
    }) {
        mind.set(saved);
    }
    Effect::new(move |_| {
        crate::phazor_panel::wiring::save_state("phazor:mind", mind.get());
    });

    #[cfg(target_arch = "wasm32")]
    {
        use phunction_gfx::{
            wgpu, FeedbackPhunctor, FieldPhunctor, FrameInput, GfxContext, Phunctor as _,
            ShaderPhunctor,
        };
        use std::cell::Cell;
        use wgpu::CurrentSurfaceTexture as Cst;

        enum Vp {
            Shader(ShaderPhunctor),
            Field(FieldPhunctor),
            Feedback(FeedbackPhunctor),
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
                // sentinel: the first raf frame installs whatever mind the
                // signal restored, through the normal swap path
                let mut current = "";
                let mut vp = Vp::Shader(ShaderPhunctor::new(&ctx, phunction_gfx::SILK_WGSL));
                let t0 = web_time::Instant::now();
                let mut last_beat = 0u64;
                let mut pulse = 0.0f32;
                // the flow filter: one-pole slew on the whole mod bus, so
                // per-block spectrum jitter arrives as swells, not spasms
                let mut flow = [0.0f32; 8];
                let mut flow_primed = false;

                crate::raf::raf_loop(move || {
                    if !canvas.is_connected() {
                        return false;
                    }
                    if let Some(m) = REQUEST.with(std::cell::Cell::take) {
                        mind.set(m);
                    }
                    // a routed field owns the room while the cable holds:
                    // the patch is architecture, not decoration
                    let routed = ROUTED_FIELD.with(std::cell::Cell::get).is_some();
                    let want = if routed {
                        "specter"
                    } else {
                        mind.get_untracked()
                    };
                    // live wgsl: build candidates through a validation error
                    // scope; the running pipeline only swaps on approval, so
                    // broken code never blanks the room (GOAL III)
                    if want == "wgsl" && WGSL_DIRTY.with(std::cell::Cell::take) {
                        let src = WGSL_SRC.with(|s| {
                            let mut b = s.borrow_mut();
                            if b.is_empty() {
                                *b = crate::phazor_panel::wiring::load_state("phazor:wgsl")
                                    .unwrap_or_else(|| WGSL_STARTER.to_string());
                            }
                            b.clone()
                        });
                        let my_gen = WGSL_GEN.with(|g| {
                            let v = g.get().wrapping_add(1);
                            g.set(v);
                            v
                        });
                        // wgpu 30: push returns a guard; .pop() yields the future
                        let scope = ctx.device.push_error_scope(wgpu::ErrorFilter::Validation);
                        let candidate = ShaderPhunctor::new(&ctx, &src);
                        let fut = scope.pop();
                        WGSL_PENDING.with(|p| *p.borrow_mut() = Some(candidate));
                        leptos::task::spawn_local(async move {
                            match fut.await {
                                None => {
                                    if WGSL_GEN.with(std::cell::Cell::get) == my_gen {
                                        WGSL_ERR.with(|e| *e.borrow_mut() = None);
                                        WGSL_APPROVED.with(|a| a.set(true));
                                    }
                                }
                                Some(e) => {
                                    if WGSL_GEN.with(std::cell::Cell::get) == my_gen {
                                        WGSL_ERR.with(|er| *er.borrow_mut() = Some(format!("{e}")));
                                        WGSL_PENDING.with(|p| p.borrow_mut().take());
                                    }
                                }
                            }
                        });
                    }
                    if want == "wgsl" && WGSL_APPROVED.with(std::cell::Cell::take) {
                        if let Some(candidate) = WGSL_PENDING.with(|p| p.borrow_mut().take()) {
                            vp = Vp::Shader(candidate);
                            current = "wgsl";
                        }
                    }
                    wgsl_status.set(WGSL_ERR.with(|e| e.borrow().clone()));
                    if want != current {
                        current = want;
                        error.set(None);
                        if want == "wgsl" {
                            // guarded path below; the previous pipeline keeps
                            // rendering until the candidate approves
                            WGSL_DIRTY.with(|d| d.set(true));
                        } else {
                            vp = match want {
                                "gyroid" => Vp::Shader(ShaderPhunctor::new(
                                    &ctx,
                                    phunction_gfx::GYROID_WGSL,
                                )),
                                "silk" => {
                                    Vp::Shader(ShaderPhunctor::new(&ctx, phunction_gfx::SILK_WGSL))
                                }
                                "current" => Vp::Shader(ShaderPhunctor::new(
                                    &ctx,
                                    phunction_gfx::CURRENT_WGSL,
                                )),
                                "petri" => Vp::Feedback(FeedbackPhunctor::new(
                                    &ctx,
                                    phunction_gfx::PETRI_SIM_WGSL,
                                    phunction_gfx::PETRI_PRESENT_WGSL,
                                )),

                                "basilica" => Vp::Shader(ShaderPhunctor::new(
                                    &ctx,
                                    phunction_gfx::BASILICA_WGSL,
                                )),
                                "gasket" => Vp::Shader(ShaderPhunctor::new(
                                    &ctx,
                                    phunction_gfx::GASKET_WGSL,
                                )),
                                "maw" => {
                                    Vp::Shader(ShaderPhunctor::new(&ctx, phunction_gfx::MAW_WGSL))
                                }
                                "bulb" => {
                                    Vp::Shader(ShaderPhunctor::new(&ctx, phunction_gfx::BULB_WGSL))
                                }
                                "indra" => {
                                    Vp::Shader(ShaderPhunctor::new(&ctx, phunction_gfx::INDRA_WGSL))
                                }
                                "hopf" => {
                                    Vp::Shader(ShaderPhunctor::new(&ctx, phunction_gfx::HOPF_WGSL))
                                }
                                "lenia" => Vp::Feedback(FeedbackPhunctor::new(
                                    &ctx,
                                    phunction_gfx::LENIA_SIM_WGSL,
                                    phunction_gfx::LENIA_PRESENT_WGSL,
                                )),
                                "cortex" => Vp::Shader(ShaderPhunctor::new(
                                    &ctx,
                                    phunction_gfx::CORTEX_WGSL,
                                )),
                                "specter" => {
                                    if webgpu {
                                        crate::camera::request();
                                        Vp::Field(FieldPhunctor::new(
                                            &ctx,
                                            phunction_gfx::SPECTER_WGSL,
                                        ))
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
                                _ => Vp::Shader(ShaderPhunctor::new(
                                    &ctx,
                                    phunction_gfx::CITADEL_WGSL,
                                )),
                            };
                        }
                    }

                    // qualia lesson: cap effective DPR — fullscreen
                    // raymarchers render at ~half res and upscale; nobody
                    // can tell at motion, everybody can tell at 12 fps
                    let dpr = web_sys::window()
                        .map_or(1.0, |w| w.device_pixel_ratio())
                        .min(1.5);
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let size = (
                        (f64::from(canvas.client_width()) * dpr * 0.5).max(1.0) as u32,
                        (f64::from(canvas.client_height()) * dpr * 0.5).max(1.0) as u32,
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
                    pulse *= 0.97; // long decay: a swell, not a strobe

                    // slow value-noise walks: never periodic, never still —
                    // the drift itself evolves forever (glacial, not spazzy)
                    let wander = |speed: f32, seed: f32, amp: f32| -> f32 {
                        let phase = now * speed + seed;
                        let cell = phase.floor();
                        let frac = phase - cell;
                        let hash = |n: f32| ((n * 127.1 + seed * 311.7).sin() * 43758.55).fract();
                        let ease = frac * frac * (3.0 - 2.0 * frac);
                        amp * ((hash(cell) * (1.0 - ease) + hash(cell + 1.0) * ease) * 2.0 - 1.0)
                    };
                    let (ds, dw, dd) = if par.auto {
                        (
                            wander(0.011, 3.1, 0.12),
                            wander(0.009, 7.7, 0.14),
                            wander(0.007, 13.3, 0.1),
                        )
                    } else {
                        (0.0, 0.0, 0.0)
                    };
                    let rms = (met.rms_l + met.rms_r) * 0.8;
                    let bands = met.bands;
                    #[allow(clippy::cast_precision_loss)]
                    let coarse = |a: usize, z: usize| -> f32 {
                        (bands[a..z].iter().sum::<f32>() / (z - a) as f32).min(1.0)
                    };

                    let mut mods = [
                        (par.scale + ds).clamp(0.0, 1.0),
                        (par.warp + dw + rms).clamp(0.0, 1.0),
                        par.hue + pulse * 0.05 + now * 0.0015,
                        (par.dolly + dd).clamp(0.0, 1.0),
                        coarse(0, 4),
                        coarse(4, 8),
                        coarse(8, 12),
                        coarse(12, 16),
                    ];
                    // the patch speaks first, then the little language
                    let patch = crate::patchbay::mind_mods();
                    for (i, p) in patch.iter().enumerate() {
                        if i == 2 {
                            mods[2] += p; // hue is a phase: wrap, don't pin
                        } else {
                            mods[i] = (mods[i] + p).clamp(0.0, 1.0);
                        }
                    }
                    #[allow(clippy::cast_possible_truncation)]
                    crate::expr_slot::apply(
                        &mut mods,
                        &[
                            now,
                            met.beats as f32,
                            coarse(0, 24),
                            coarse(24, 48),
                            coarse(72, 96),
                            rms,
                        ],
                    );
                    if !flow_primed {
                        flow = mods;
                        flow_primed = true;
                    }
                    for (f, m) in flow.iter_mut().zip(mods) {
                        // ~0.5s to 95% at 60fps: musical, never twitchy
                        *f += (m - *f) * 0.06;
                    }
                    let input = FrameInput {
                        time: now,
                        aspect: size.0 as f32 / size.1 as f32,
                        mods: flow,
                    };
                    match &mut vp {
                        Vp::Shader(s) => s.frame(&ctx, &view, &input),
                        Vp::Field(f) => f.frame(&ctx, &view, &input),
                        Vp::Feedback(f) => f.frame(&ctx, &view, &input),
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
        // qualia move: the mind IS the room. Fixed fullscreen canvas under
        // the rack; every panel floats translucent above the visual field.
        <canvas
            node_ref=canvas_ref
            class="mind-field"
            aria-label="the mind field: fractal, gyroid, neural field, or your own kaleidoscoped camera, wall to wall"
        ></canvas>
        {move || error.get().map(|e| view! { <p class="gfx-error">"✗ " {e}</p> })}
        <RackPanel title="mind controls" class="span5" folded=true hue=235.0>
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
                    "explore"
                </button>
                <Led on={Signal::derive(move || params.get().auto)} hue=280.0 label="drift" />
            </div>
        </RackPanel>
        <RackPanel title="shader · live wgsl" class="span5" folded=true hue=190.0>
            <textarea
                class="pb-script wgsl-editor"
                spellcheck="false"
                rows="10"
                prop:value=wgsl_source
                on:input=move |ev| wgsl_source.set(event_target_value(&ev))
                aria-label="live wgsl fragment source"
            ></textarea>
            <div class="expr-row">
                <button
                    class="xport"
                    on:click=move |_| {
                        crate::fractal::submit_wgsl(&wgsl_source.get_untracked());
                        mind.set("wgsl");
                    }
                >"run shader"</button>
                <button
                    class="xport"
                    on:click=move |_| {
                        wgsl_source.set(WGSL_STARTER.to_string());
                        crate::fractal::submit_wgsl(WGSL_STARTER);
                        mind.set("wgsl");
                    }
                >"starter"</button>
            </div>
            <p class="expr-status" class:err=move || wgsl_status.get().is_some()>
                {move || wgsl_status.get().map_or_else(
                    || "⊢ prelude gives you: u.time · u.aspect · u.mod0..7 · palette(t, _) · TAU · hash21".to_string(),
                    |e| format!("✗ {}", e.lines().next().unwrap_or("compile error")),
                )}
            </p>
        </RackPanel>
    }
}
