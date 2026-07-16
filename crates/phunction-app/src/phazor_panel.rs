//! The phazor panel: transport, step grid, knobs, meters.
//!
//! UI-side rules: every engine interaction is a [`Command`] pushed onto the
//! ring (never shared state), and everything displayed comes back through
//! [`MeterFrame`] telemetry. The UI is a *client* of the engine, exactly as
//! a MIDI controller would be — which is what makes it replaceable, and
//! what will let external controllers drive the same surface later.

use crate::fractal::CitadelRack;
use crate::rack::{Jack, Knob, Led, LedMeter, RackPanel};
use leptos::prelude::*;
use phazor_core::{Command, ParamId, Step, StepSequencer};

/// Preset riff for v0 step toggles (A-minor pentatonic with octave drops).
/// Each step that's switched on plays its slot's note; a proper per-step
/// note editor replaces this soon.
const RIFF: [u8; 16] = [
    45, 57, 48, 57, 45, 55, 48, 60, 45, 57, 52, 57, 43, 55, 48, 62,
];

/// Live telemetry mirrored into signals for display.
#[derive(Clone, Copy, Default)]
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
pub fn PhazorPage() -> impl IntoView {
    let powered = RwSignal::new(false);
    let meters = RwSignal::new(Meters::default());
    let steps = RwSignal::new([false; 16]);
    let citadel = RwSignal::new(crate::fractal::CitadelParams::default());
    let tempo = RwSignal::new(120.0f32);

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
                            min=60.0 max=200.0 init=120.0 hue=190.0
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
                            min=20.0 max=18000.0 init=9000.0 log=true hue=235.0
                            fmt={|v| if v >= 1000.0 { format!("{:.1} kHz", v / 1000.0) } else { format!("{v:.0} Hz") }}
                            on_value=move |v: f32| wiring::send(Command::SetParam { id: ParamId::FilterCutoff, value: v })
                        />
                        <Knob
                            label="resonance"
                            min=0.5 max=10.0 init=0.707 hue=325.0
                            fmt=|v| format!("{v:.2}")
                            on_value=move |v: f32| wiring::send(Command::SetParam { id: ParamId::FilterQ, value: v })
                        />
                        <Knob
                            label="brightness"
                            min=0.0 max=1.0 init=0.35 hue=100.0
                            fmt=|v| format!("{v:.2}")
                            on_value=move |v: f32| wiring::send(Command::SetParam { id: ParamId::OscBrightness, value: v })
                        />
                    </RackPanel>

                    <RackPanel title="mix" class="span3">
                        <Knob
                            label="master"
                            min=0.0 max=1.2 init=0.8 hue=55.0
                            fmt=|v| format!("{v:.2}")
                            on_value=move |v: f32| wiring::send(Command::SetParam { id: ParamId::MasterGain, value: v })
                        />
                        <LedMeter label="L" level={Signal::derive(move || (meters.get().rms_l * 3.0).min(1.0))} />
                        <LedMeter label="R" level={Signal::derive(move || (meters.get().rms_r * 3.0).min(1.0))} />
                        <Led
                            on={Signal::derive(move || meters.get().peak_l.max(meters.get().peak_r) > 0.97)}
                            hue=10.0
                            label="clip"
                        />
                    </RackPanel>

                    <svg class="cable" viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden="true">
                        <path class="cable-shadow" d="M26.5 57 C 29 86, 40 74, 43.5 42"></path>
                        <path class="cable-core" d="M26.5 56 C 29 84, 40 72, 43.5 41"></path>
                        <path class="cable-sheen" d="M26.2 55.7 C 28.7 83.5, 39.7 71.5, 43.2 40.7"></path>
                    </svg>
                    <CitadelRack params=citadel />

                    <RackPanel title="presets · whole worlds" class="span12">
                        {PRESETS
                            .iter()
                            .map(|preset| {
                                let name = preset.name;
                                view! {
                                    <button
                                        class="xport preset"
                                        on:click=move |_| apply_preset(preset, steps, citadel, tempo)
                                    >
                                        {name}
                                    </button>
                                }
                            })
                            .collect_view()}
                        <span class="preset-hint">
                            "each button rewrites the whole machine: pattern, tempo, voice, folds"
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

                    <RackPanel title="sequence">
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
                    <span><kbd>"shift"</kbd>"+drag knobs for fine control · double-click resets"</span>
                    <span><kbd>"`"</kbd>" debug"</span>
                </div>
            </Show>
        </main>
    }
}

/// One whole-machine state: pattern, tempo, voice, folds. VISION calls
/// these worlds; the panel calls them presets.
pub struct Preset {
    name: &'static str,
    tempo: f64,
    pattern: [bool; 16],
    cutoff: f32,
    resonance: f32,
    brightness: f32,
    master: f32,
    citadel: crate::fractal::CitadelParams,
}

/// The shipped worlds.
pub static PRESETS: [Preset; 3] = [
    Preset {
        name: "⌬ acid citadel",
        tempo: 140.0,
        pattern: [
            true, false, true, false, true, true, false, true, true, false, true, false, true,
            true, true, false,
        ],
        cutoff: 900.0,
        resonance: 7.5,
        brightness: 0.85,
        master: 0.85,
        citadel: crate::fractal::CitadelParams {
            scale: 0.62,
            warp: 0.55,
            hue: 0.78,
            dolly: 0.68,
            auto: false,
            gen: 0,
        },
    },
    Preset {
        name: "∿ ambient drift",
        tempo: 72.0,
        pattern: [
            true, false, false, false, false, false, true, false, false, false, true, false, false,
            false, false, false,
        ],
        cutoff: 2400.0,
        resonance: 1.1,
        brightness: 0.22,
        master: 0.75,
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
        name: "◬ spectral storm",
        tempo: 128.0,
        pattern: [
            true, true, false, true, false, true, true, false, true, false, true, true, false,
            true, false, true,
        ],
        cutoff: 5200.0,
        resonance: 3.8,
        brightness: 1.0,
        master: 0.9,
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
/// + control remount (gen bump) so every cap and needle tells the truth.
fn apply_preset(
    p: &'static Preset,
    steps: RwSignal<[bool; 16]>,
    citadel: RwSignal<crate::fractal::CitadelParams>,
    tempo: RwSignal<f32>,
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
    wiring::send(Command::SetParam {
        id: ParamId::FilterCutoff,
        value: p.cutoff,
    });
    wiring::send(Command::SetParam {
        id: ParamId::FilterQ,
        value: p.resonance,
    });
    wiring::send(Command::SetParam {
        id: ParamId::OscBrightness,
        value: p.brightness,
    });
    wiring::send(Command::SetParam {
        id: ParamId::MasterGain,
        value: p.master,
    });
    wiring::send(Command::Play);
    citadel.update(|c| {
        let gen = c.gen + 1;
        *c = p.citadel;
        c.gen = gen;
    });
}
