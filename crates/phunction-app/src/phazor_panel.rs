//! The phazor panel: transport, step grid, knobs, meters, FX, weather.
//!
//! UI-side rules: every engine interaction is a [`Command`] pushed onto the
//! ring (never shared state), and everything displayed comes back through
//! [`MeterFrame`] telemetry. The UI is a *client* of the engine, exactly as
//! a MIDI controller would be — which is what makes it replaceable, and
//! what will let external controllers drive the same surface later.

use crate::fractal::CitadelRack;
use crate::rack::{Jack, Knob, Led, LedMeter, RackPanel};
use leptos::prelude::*;
use phazor_core::meter::SCOPE;
use phazor_core::{Command, ParamId, Step, StepSequencer};

/// Pad notes for the user pattern (A-minor pentatonic with octave drops).
/// The pads ride the arp layer *on top of* the generative score.
const RIFF: [u8; 16] = [
    45, 57, 48, 57, 45, 55, 48, 60, 45, 57, 52, 57, 43, 55, 48, 62,
];

/// Live telemetry mirrored into signals for display.
#[derive(Clone, Copy)]
pub(crate) struct Meters {
    peak_l: f32,
    peak_r: f32,
    rms_l: f32,
    rms_r: f32,
    beats: f64,
    voices: u8,
    playing: bool,
    /// Commands dropped because the ring was full (debug HUD surfaces this).
    dropped: u32,
    /// 16-band spectrum mirror.
    bands: [f32; phazor_core::BANDS],
    /// Oscilloscope trace mirror.
    scope: [f32; SCOPE],
}

// Hand-written: `Default` for arrays stops at 32 elements (the scope is 64).
impl Default for Meters {
    fn default() -> Self {
        Self {
            peak_l: 0.0,
            peak_r: 0.0,
            rms_l: 0.0,
            rms_r: 0.0,
            beats: 0.0,
            voices: 0,
            playing: false,
            dropped: 0,
            bands: [0.0; phazor_core::BANDS],
            scope: [0.0; SCOPE],
        }
    }
}

/// Every continuous control's UI-side truth, so presets can rewrite the
/// whole surface and every cap still points where the engine actually is.
#[derive(Clone, Copy)]
struct Cv {
    cutoff: RwSignal<f32>,
    resonance: RwSignal<f32>,
    brightness: RwSignal<f32>,
    master: RwSignal<f32>,
    delay_mix: RwSignal<f32>,
    delay_fb: RwSignal<f32>,
    verb_mix: RwSignal<f32>,
    verb_size: RwSignal<f32>,
    drive: RwSignal<f32>,
    drone: RwSignal<f32>,
    arps: RwSignal<f32>,
    lead: RwSignal<f32>,
    density: RwSignal<f32>,
    seed: RwSignal<u32>,
    scale: RwSignal<u8>,
}

impl Cv {
    fn new() -> Self {
        let d = ParamId::default_value;
        Self {
            cutoff: RwSignal::new(d(ParamId::FilterCutoff)),
            resonance: RwSignal::new(d(ParamId::FilterQ)),
            brightness: RwSignal::new(d(ParamId::OscBrightness)),
            master: RwSignal::new(d(ParamId::MasterGain)),
            delay_mix: RwSignal::new(d(ParamId::DelayMix)),
            delay_fb: RwSignal::new(d(ParamId::DelayFeedback)),
            verb_mix: RwSignal::new(d(ParamId::ReverbMix)),
            verb_size: RwSignal::new(d(ParamId::ReverbSize)),
            drive: RwSignal::new(d(ParamId::Drive)),
            drone: RwSignal::new(d(ParamId::DroneLevel)),
            arps: RwSignal::new(d(ParamId::ArpLevel)),
            lead: RwSignal::new(d(ParamId::LeadLevel)),
            density: RwSignal::new(d(ParamId::LeadDensity)),
            seed: RwSignal::new(0xC0FF_EE00),
            scale: RwSignal::new(0),
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) mod wiring {
    use super::Meters;
    use leptos::prelude::*;
    use phazor_core::Command;
    use std::cell::RefCell;

    thread_local! {
        static PHAZOR: RefCell<Option<phazor_web::Phazor>> = const { RefCell::new(None) };
        static DROPPED: RefCell<u32> = const { RefCell::new(0) };
        static LAST: std::cell::Cell<phazor_core::MeterFrame> =
            std::cell::Cell::new(phazor_core::MeterFrame::default());
    }

    /// Latest engine telemetry, for anything outside the panel (fractal
    /// sync, patchbay sources). Zero-cost snapshot of the drain loop.
    pub fn last_meter() -> phazor_core::MeterFrame {
        LAST.with(std::cell::Cell::get)
    }

    /// Boot the engine (must be a user gesture) and start the telemetry loop.
    pub fn power_on(meters: RwSignal<Meters>, powered: RwSignal<bool>) {
        leptos::task::spawn_local(async move {
            match phazor_web::start().await {
                Ok(p) => {
                    PHAZOR.with(|slot| *slot.borrow_mut() = Some(p));
                    powered.set(true);
                    ignite();
                    crate::raf::raf_loop(move || {
                        PHAZOR.with(|slot| {
                            if let Some(p) = slot.borrow_mut().as_mut() {
                                let mut latest = None;
                                while let Ok(frame) = p.meters.pop() {
                                    latest = Some(frame);
                                }
                                if let Some(f) = latest {
                                    LAST.with(|l| l.set(f));
                                    meters.update(|m| {
                                        m.peak_l = f.peak_l;
                                        m.peak_r = f.peak_r;
                                        m.rms_l = f.rms_l;
                                        m.rms_r = f.rms_r;
                                        m.beats = f.beats;
                                        m.voices = f.voices;
                                        m.playing = f.playing;
                                        m.dropped = DROPPED.with(|d| *d.borrow());
                                        m.bands = f.bands;
                                        m.scope = f.scope;
                                    });
                                }
                            }
                        });
                        true // run for the lifetime of the page
                    });
                }
                Err(e) => {
                    web_sys::console::error_2(&"phazor failed to start:".into(), &e);
                }
            }
        });
    }

    /// The ignition flash: a moment of stage light when the engine wakes.
    fn ignite() {
        use wasm_bindgen::prelude::*;
        use wasm_bindgen::JsCast;
        let Some(root) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.document_element())
        else {
            return;
        };
        let _ = root.class_list().add_1("ignite");
        let root2 = root.clone();
        let end = Closure::once_into_js(move || {
            let _ = root2.class_list().remove_1("ignite");
        });
        let _ = web_sys::window()
            .expect("window")
            .set_timeout_with_callback_and_timeout_and_arguments_0(end.unchecked_ref(), 1400);
    }

    /// Play one short note from anywhere (the wordmark keys): boots the
    /// engine on first use — caller must be inside a user gesture.
    pub fn play_note(note: u8) {
        use wasm_bindgen::prelude::*;
        use wasm_bindgen::JsCast;
        let already = PHAZOR.with(|slot| slot.borrow().is_some());
        let strike = move || {
            send(Command::NoteOn { note, vel: 100 });
            let off = Closure::once_into_js(move || send(Command::NoteOff { note }));
            let _ = web_sys::window()
                .expect("window")
                .set_timeout_with_callback_and_timeout_and_arguments_0(off.unchecked_ref(), 220);
        };
        if already {
            strike();
        } else {
            leptos::task::spawn_local(async move {
                if let Ok(p) = phazor_web::start().await {
                    PHAZOR.with(|slot| *slot.borrow_mut() = Some(p));
                    strike();
                }
            });
        }
    }

    /// Push a command; counts (rather than blocks on) overflow.
    pub fn send(cmd: Command) {
        PHAZOR.with(|slot| {
            if let Some(p) = slot.borrow_mut().as_mut() {
                if p.commands.push(cmd).is_err() {
                    DROPPED.with(|d| *d.borrow_mut() += 1);
                }
            }
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod wiring {
    //! Native stub so `cargo clippy --workspace` (host target) checks the
    //! view code; the panel is browser-only at runtime.
    use super::Meters;
    use leptos::prelude::*;
    use phazor_core::Command;

    pub fn power_on(_meters: RwSignal<Meters>, _powered: RwSignal<bool>) {}
    pub fn send(_cmd: Command) {}
    pub fn play_note(_note: u8) {}
    #[allow(dead_code)] // consumed only by wasm render loops (fractal sync)
    pub fn last_meter() -> phazor_core::MeterFrame {
        phazor_core::MeterFrame::default()
    }
}

/// The `/phazor` route.
#[component]
#[allow(clippy::too_many_lines)]
pub fn PhazorPage() -> impl IntoView {
    let powered = RwSignal::new(false);
    let meters = RwSignal::new(Meters::default());
    let steps = RwSignal::new([false; 16]);
    let citadel = RwSignal::new(crate::fractal::CitadelParams::default());
    let tempo = RwSignal::new(120.0f32);
    let cv = Cv::new();

    // zen mode: `z` (outside inputs) or the floating button drops every
    // panel away and leaves the mind field wall to wall
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::prelude::*;
        use wasm_bindgen::JsCast;
        let on_key =
            Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |ev: web_sys::KeyboardEvent| {
                let tag = ev
                    .target()
                    .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                    .map(|e| e.tag_name())
                    .unwrap_or_default();
                if tag == "INPUT" || tag == "TEXTAREA" {
                    return;
                }
                if ev.key() == "z" {
                    if let Some(root) = web_sys::window()
                        .and_then(|w| w.document())
                        .and_then(|d| d.document_element())
                    {
                        let _ = root.class_list().toggle("zen");
                    }
                }
            });
        if let Some(w) = web_sys::window() {
            let _ = w.add_event_listener_with_callback("keydown", on_key.as_ref().unchecked_ref());
        }
        on_key.forget();
    }

    #[cfg(target_arch = "wasm32")]
    let toggle_zen = move |_ev: web_sys::MouseEvent| {
        if let Some(root) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.document_element())
        {
            let _ = root.class_list().toggle("zen");
        }
    };
    #[cfg(not(target_arch = "wasm32"))]
    let toggle_zen = move |_ev: leptos::ev::MouseEvent| {};

    // Power-on is ignition into a *world*, not silence — but the world
    // waits for the viewport to claim its GPU device first (see gfx_gate:
    // audible playback can stall requestAdapter). A fallback timer starts
    // the music anyway if the viewport never reports (headless, WebGL2).
    let booted = StoredValue::new(false);
    let start_world = move || {
        if !booted.get_value() {
            booted.set_value(true);
            apply_preset(&PRESETS[0], steps, citadel, tempo, cv);
        }
    };
    Effect::new(move |_| {
        if powered.get() {
            crate::gfx_gate::on_ready(start_world);
            #[cfg(target_arch = "wasm32")]
            {
                use wasm_bindgen::prelude::*;
                use wasm_bindgen::JsCast;
                let fallback = Closure::once_into_js(start_world);
                let _ = web_sys::window()
                    .expect("window")
                    .set_timeout_with_callback_and_timeout_and_arguments_0(
                        fallback.unchecked_ref(),
                        6000,
                    );
            }
        }
    });

    let toggle_step = move |i: usize| {
        steps.update(|s| s[i] = !s[i]);
        let step = steps.get()[i].then(|| Step {
            note: RIFF[i],
            vel: 108,
            gate: 0.55,
        });
        wiring::send(Command::SetStep {
            index: i as u8,
            step,
        });
    };

    let playhead = move || {
        let m = meters.get();
        if m.playing {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            Some(((m.beats * 4.0) as usize) % StepSequencer::LEN)
        } else {
            None
        }
    };

    // Oscilloscope trace: 64 points, engine-truth, redrawn per meter frame.
    let scope_points = move || {
        let s = meters.get().scope;
        let mut pts = String::with_capacity(SCOPE * 8);
        for (i, v) in s.iter().enumerate() {
            use core::fmt::Write;
            let _ = write!(pts, "{},{:.1} ", i, 16.0 - v.clamp(-1.0, 1.0) * 14.0);
        }
        pts
    };

    // A knob that owns a Cv signal and a ParamId, so presets stay truthful.
    macro_rules! cv_knob {
        ($label:literal, $sig:expr, $id:expr, $min:literal, $max:literal, $hue:literal) => {{
            let sig = $sig;
            view! {
                <Knob
                    label=$label
                    min=$min max=$max init={sig.get_untracked()} hue=$hue
                    fmt=|v| format!("{v:.2}")
                    sync=Signal::derive(move || sig.get())
                    on_value=move |v: f32| {
                        sig.set(v);
                        wiring::send(Command::SetParam { id: $id, value: v });
                    }
                />
            }
        }};
    }

    view! {
        <main class="panel">
            <header class="panel-head">
                <h1>"phazor"</h1>
                <span class="sub">"∿ a phase-locked instrument · the engine is a thread in your audio driver"</span>
            </header>

            <Show
                when=move || powered.get()
                fallback=move || view! {
                    <button class="power" on:click=move |_| wiring::power_on(meters, powered)>
                        "⏻ power on"
                    </button>
                }
            >
                <div class="rack">
                    <RackPanel title="transport" class="span4">
                        <button
                            class="xport"
                            class:lit=move || meters.get().playing
                            on:click=move |_| wiring::send(Command::Play)
                        >"▶ play"</button>
                        <button class="xport" on:click=move |_| wiring::send(Command::Stop)>"■ stop"</button>
                        <button class="xport panic" on:click=move |_| wiring::send(Command::AllNotesOff)>"✕ panic"</button>
                        <Led on=Signal::derive(move || meters.get().playing) hue=145.0 label="run" />
                        <Knob
                            label="tempo"
                            min=50.0 max=200.0 init=120.0 hue=190.0
                            fmt=|v| format!("{v:.0} bpm")
                            sync=Signal::derive(move || tempo.get())
                            on_value=move |v: f32| {
                                tempo.set(v);
                                wiring::send(Command::SetTempo(f64::from(v)));
                            }
                        />
                        <Jack label="clk" />
                        <div class="lcd">
                            <span>{move || format!("beat {:>6.2}", meters.get().beats)}</span>
                            <span>{move || format!("vox {}", meters.get().voices)}</span>
                            <span class:warn={move || meters.get().dropped > 0}>
                                {move || format!("drop {}", meters.get().dropped)}
                            </span>
                        </div>
                    </RackPanel>

                    <RackPanel title="voice" class="span5">
                        <Jack label="cv" />
                        <Knob
                            label="cutoff"
                            min=20.0 max=18000.0 init=3200.0 log=true hue=235.0
                            fmt={|v| if v >= 1000.0 { format!("{:.1} kHz", v / 1000.0) } else { format!("{v:.0} Hz") }}
                            sync=Signal::derive(move || cv.cutoff.get())
                            on_value=move |v: f32| {
                                cv.cutoff.set(v);
                                wiring::send(Command::SetParam { id: ParamId::FilterCutoff, value: v });
                            }
                        />
                        <Knob
                            label="resonance"
                            min=0.5 max=10.0 init=0.707 hue=325.0
                            fmt=|v| format!("{v:.2}")
                            sync=Signal::derive(move || cv.resonance.get())
                            on_value=move |v: f32| {
                                cv.resonance.set(v);
                                wiring::send(Command::SetParam { id: ParamId::FilterQ, value: v });
                            }
                        />
                        {cv_knob!("brightness", cv.brightness, ParamId::OscBrightness, 0.0, 1.0, 100.0)}
                    </RackPanel>

                    <RackPanel title="mix" class="span3">
                        {cv_knob!("master", cv.master, ParamId::MasterGain, 0.0, 1.2, 55.0)}
                        <LedMeter label="L" level={Signal::derive(move || (meters.get().rms_l * 3.0).min(1.0))} />
                        <LedMeter label="R" level={Signal::derive(move || (meters.get().rms_r * 3.0).min(1.0))} />
                        <Led
                            on={Signal::derive(move || meters.get().peak_l.max(meters.get().peak_r) > 0.97)}
                            hue=10.0
                            label="clip"
                        />
                    </RackPanel>

                    <RackPanel title="weather · the score writes itself" class="span5">
                        {cv_knob!("drone", cv.drone, ParamId::DroneLevel, 0.0, 1.0, 280.0)}
                        {cv_knob!("arps", cv.arps, ParamId::ArpLevel, 0.0, 1.0, 145.0)}
                        {cv_knob!("lead", cv.lead, ParamId::LeadLevel, 0.0, 1.0, 55.0)}
                        {cv_knob!("chance", cv.density, ParamId::LeadDensity, 0.0, 1.0, 10.0)}
                        <div class="fold-side">
                            <button
                                class="xport"
                                on:click=move |_| {
                                    let next = cv.seed.get().wrapping_mul(0x9E37_79B9).wrapping_add(0x7F4A_7C15);
                                    cv.seed.set(next);
                                    wiring::send(Command::SetSeed(next));
                                }
                            >"reseed"</button>
                            <div class="lcd"><span>{move || format!("wx {:08x}", cv.seed.get())}</span></div>
                        </div>
                        <div class="vp-select">
                            {[(0u8, "phrygian"), (1, "aeolian"), (2, "dorian")]
                                .map(|(id, name)| view! {
                                    <button
                                        class="xport vp"
                                        class:lit=move || cv.scale.get() == id
                                        on:click=move |_| {
                                            cv.scale.set(id);
                                            wiring::send(Command::SetScale(id));
                                        }
                                    >{name}</button>
                                })}
                        </div>
                    </RackPanel>

                    <RackPanel title="fx · space" class="span4">
                        {cv_knob!("echo", cv.delay_mix, ParamId::DelayMix, 0.0, 1.0, 190.0)}
                        {cv_knob!("regen", cv.delay_fb, ParamId::DelayFeedback, 0.0, 0.9, 235.0)}
                        {cv_knob!("wash", cv.verb_mix, ParamId::ReverbMix, 0.0, 1.0, 280.0)}
                        {cv_knob!("cavern", cv.verb_size, ParamId::ReverbSize, 0.0, 1.0, 325.0)}
                        {cv_knob!("drive", cv.drive, ParamId::Drive, 0.0, 1.0, 10.0)}
                    </RackPanel>

                    <RackPanel title="scope" class="span3">
                        <svg class="scope-lcd" viewBox="0 0 64 32" preserveAspectRatio="none" aria-label="oscilloscope: the master bus waveform">
                            <line class="scope-axis" x1="0" y1="16" x2="64" y2="16"></line>
                            <polyline class="scope-trace" points=scope_points></polyline>
                        </svg>
                        <Led on=Signal::derive(move || meters.get().rms_l + meters.get().rms_r > 0.005) hue=145.0 label="sig" />
                    </RackPanel>

                    <ExprRack meters=meters />

                    <crate::patchbay::Patchbay />

                    <svg class="cable" viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden="true">
                        <path class="cable-shadow" d="M26.5 57 C 29 86, 40 74, 43.5 42"></path>
                        <path class="cable-core" d="M26.5 56 C 29 84, 40 72, 43.5 41"></path>
                        <path class="cable-sheen" d="M26.2 55.7 C 28.7 83.5, 39.7 71.5, 43.2 40.7"></path>
                    </svg>
                    <CitadelRack params=citadel />

                    <RackPanel title="worlds · whole-machine presets" class="span12">
                        {PRESETS
                            .iter()
                            .map(|preset| {
                                let name = preset.name;
                                view! {
                                    <button
                                        class="xport preset"
                                        on:click=move |_| apply_preset(preset, steps, citadel, tempo, cv)
                                    >
                                        {name}
                                    </button>
                                }
                            })
                            .collect_view()}
                        <span class="preset-hint">
                            "each world rewrites the whole machine: score, tempo, voice, space, minds"
                        </span>
                    </RackPanel>

                    <RackPanel title="spectrum · 60 Hz → 12 kHz" class="span12">
                        <div class="spectrum-row">
                            {(0..phazor_core::BANDS)
                                .map(|i| {
                                    view! {
                                        <div class="spec-band" style=("--i", i.to_string())>
                                            <div
                                                class="spec-fill"
                                                style=("height", move || {
                                                    format!("{:.1}%", (meters.get().bands[i] * 130.0).min(100.0))
                                                })
                                            ></div>
                                        </div>
                                    }
                                })
                                .collect_view()}
                        </div>
                    </RackPanel>

                    <RackPanel title="sequence · your riff over the weather">
                        <section class="steps" style="width: 100%">
                            {(0..StepSequencer::LEN)
                                .map(|i| {
                                    view! {
                                        <button
                                            class="step"
                                            // Each step wears the hue of its phase angle (2πi/16).
                                            style=("--i", i.to_string())
                                            class:on=move || steps.get()[i]
                                            class:now=move || playhead() == Some(i)
                                            on:click=move |_| toggle_step(i)
                                        >
                                            <span class="note">{RIFF[i]}</span>
                                        </button>
                                    }
                                })
                                .collect_view()}
                        </section>
                    </RackPanel>
                </div>

                <div class="keyhints">
                    <span><kbd>"space"</kbd>" play/stop"</span>
                    <span><kbd>"esc"</kbd>" panic"</span>
                    <span><kbd>"z"</kbd>" zen"</span>
                    <span><kbd>"shift"</kbd>"+drag knobs for fine control · double-click resets"</span>
                    <span><kbd>"`"</kbd>" debug"</span>
                </div>
                <button class="zen-toggle" on:click=toggle_zen>"zen"</button>
            </Show>
        </main>
    }
}

/// One whole-machine state: score, tempo, voice, space, minds. VISION calls
/// these worlds; each is a different weather system over the same machine.
pub struct Preset {
    name: &'static str,
    tempo: f64,
    pattern: [bool; 16],
    cutoff: f32,
    resonance: f32,
    brightness: f32,
    master: f32,
    delay_mix: f32,
    delay_fb: f32,
    verb_mix: f32,
    verb_size: f32,
    drive: f32,
    drone: f32,
    arps: f32,
    lead: f32,
    density: f32,
    seed: u32,
    scale: u8,
    citadel: crate::fractal::CitadelParams,
}

/// The shipped worlds. Dark by default — the machine should loom, not chirp.
pub static PRESETS: [Preset; 3] = [
    Preset {
        name: "undervoid",
        tempo: 66.0,
        pattern: [false; 16],
        cutoff: 1800.0,
        resonance: 1.4,
        brightness: 0.25,
        master: 0.85,
        delay_mix: 0.3,
        delay_fb: 0.55,
        verb_mix: 0.55,
        verb_size: 0.9,
        drive: 0.3,
        drone: 0.95,
        arps: 0.5,
        lead: 0.55,
        density: 0.35,
        seed: 0xC0FF_EE00,
        scale: 0, // phrygian: the flat second looms
        citadel: crate::fractal::CitadelParams {
            scale: 0.38,
            warp: 0.42,
            hue: 0.82,
            dolly: 0.3,
            auto: true,
            gen: 0,
        },
    },
    Preset {
        name: "pale arps",
        tempo: 84.0,
        pattern: [
            true, false, false, false, false, false, true, false, false, false, true, false, false,
            false, false, false,
        ],
        cutoff: 3400.0,
        resonance: 2.2,
        brightness: 0.5,
        master: 0.8,
        delay_mix: 0.45,
        delay_fb: 0.5,
        verb_mix: 0.4,
        verb_size: 0.7,
        drive: 0.2,
        drone: 0.6,
        arps: 0.85,
        lead: 0.5,
        density: 0.45,
        seed: 0x0000_7331,
        scale: 1, // aeolian
        citadel: crate::fractal::CitadelParams {
            scale: 0.36,
            warp: 0.45,
            hue: 0.12,
            dolly: 0.28,
            auto: true,
            gen: 0,
        },
    },
    Preset {
        name: "black rain",
        tempo: 106.0,
        pattern: [
            true, false, true, false, false, true, false, false, true, false, true, false, false,
            true, false, false,
        ],
        cutoff: 5200.0,
        resonance: 3.8,
        brightness: 0.8,
        master: 0.9,
        delay_mix: 0.5,
        delay_fb: 0.6,
        verb_mix: 0.35,
        verb_size: 0.6,
        drive: 0.5,
        drone: 0.7,
        arps: 0.75,
        lead: 0.85,
        density: 0.8,
        seed: 0x0000_DEAD,
        scale: 2, // dorian: one candle lit
        citadel: crate::fractal::CitadelParams {
            scale: 0.5,
            warp: 0.72,
            hue: 0.5,
            dolly: 0.5,
            auto: true,
            gen: 0,
        },
    },
];

/// Rewrite the whole machine to a preset: engine commands + fractal state
/// + every Cv signal, so every cap and needle tells the truth.
fn apply_preset(
    p: &'static Preset,
    steps: RwSignal<[bool; 16]>,
    citadel: RwSignal<crate::fractal::CitadelParams>,
    tempo: RwSignal<f32>,
    cv: Cv,
) {
    #[allow(clippy::cast_possible_truncation)]
    tempo.set(p.tempo as f32);
    wiring::send(Command::SetTempo(p.tempo));
    steps.set(p.pattern);
    for (i, &on) in p.pattern.iter().enumerate() {
        #[allow(clippy::cast_possible_truncation)]
        wiring::send(Command::SetStep {
            index: i as u8,
            step: on.then(|| Step {
                note: RIFF[i],
                vel: 108,
                gate: 0.55,
            }),
        });
    }
    for (sig, id, value) in [
        (cv.cutoff, ParamId::FilterCutoff, p.cutoff),
        (cv.resonance, ParamId::FilterQ, p.resonance),
        (cv.brightness, ParamId::OscBrightness, p.brightness),
        (cv.master, ParamId::MasterGain, p.master),
        (cv.delay_mix, ParamId::DelayMix, p.delay_mix),
        (cv.delay_fb, ParamId::DelayFeedback, p.delay_fb),
        (cv.verb_mix, ParamId::ReverbMix, p.verb_mix),
        (cv.verb_size, ParamId::ReverbSize, p.verb_size),
        (cv.drive, ParamId::Drive, p.drive),
        (cv.drone, ParamId::DroneLevel, p.drone),
        (cv.arps, ParamId::ArpLevel, p.arps),
        (cv.lead, ParamId::LeadLevel, p.lead),
        (cv.density, ParamId::LeadDensity, p.density),
    ] {
        sig.set(value);
        wiring::send(Command::SetParam { id, value });
    }
    cv.seed.set(p.seed);
    wiring::send(Command::SetSeed(p.seed));
    cv.scale.set(p.scale);
    wiring::send(Command::SetScale(p.scale));
    wiring::send(Command::Play);
    citadel.update(|c| {
        let gen = c.gen + 1;
        *c = p.citadel;
        c.gen = gen;
    });
}

/// The expr rack: a text field that is a patch cable. Type a formula in
/// phunction's little language, pick a fader to possess, and the viewport
/// obeys — parse errors answer in theorem voice, addressed to the character.
#[component]
fn ExprRack(
    /// Telemetry signal (drives the live value needle at frame rate).
    meters: RwSignal<Meters>,
) -> impl IntoView {
    const DEFAULT_SRC: &str = "0.3*sin(t*0.1) + bass*0.5";
    let source = RwSignal::new(DEFAULT_SRC.to_string());
    let target = RwSignal::new(1usize); // warp, by default
    let error = RwSignal::new(None::<String>);

    let install =
        move || match phunction_graph::expr::parse(&source.get_untracked(), crate::expr_slot::VARS)
        {
            Ok(program) => {
                error.set(None);
                crate::expr_slot::set(Some((program, target.get_untracked())));
            }
            Err(e) => {
                error.set(Some(format!("✗ at char {}: {}", e.pos + 1, e.msg)));
                crate::expr_slot::set(None);
            }
        };
    // arm the default expression on mount
    install();

    view! {
        <RackPanel title="expr · a little language" class="span9">
            <div class="expr-row">
                <input
                    class="expr-input"
                    type="text"
                    spellcheck="false"
                    autocomplete="off"
                    prop:value=move || source.get()
                    on:input=move |ev| {
                        source.set(event_target_value(&ev));
                        install();
                    }
                    aria-label="modulation expression"
                />
                <div class="vp-select">
                    {crate::expr_slot::TARGETS
                        .into_iter()
                        .enumerate()
                        .map(|(i, name)| {
                            view! {
                                <button
                                    class="xport vp"
                                    class:lit=move || target.get() == i
                                    on:click=move |_| {
                                        target.set(i);
                                        install();
                                    }
                                >
                                    {"→ "}{name}
                                </button>
                            }
                        })
                        .collect_view()}
                </div>
            </div>
            <div class="expr-status" class:err=move || error.get().is_some()>
                {move || {
                    let _ = meters.get(); // frame clock for the live needle
                    error.get().unwrap_or_else(|| {
                        format!(
                            "⊢ vars: t · beat · bass · mid · air · rms   ∙   value {:+.3}",
                            crate::expr_slot::last()
                        )
                    })
                }}
            </div>
        </RackPanel>
    }
}
