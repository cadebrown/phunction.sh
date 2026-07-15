//! The phazor panel: transport, step grid, knobs, meters.
//!
//! UI-side rules: every engine interaction is a [`Command`] pushed onto the
//! ring (never shared state), and everything displayed comes back through
//! [`MeterFrame`] telemetry. The UI is a *client* of the engine, exactly as
//! a MIDI controller would be — which is what makes it replaceable, and
//! what will let external controllers drive the same surface later.

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
struct Meters {
    peak_l: f32,
    peak_r: f32,
    rms_l: f32,
    rms_r: f32,
    beats: f64,
    voices: u8,
    playing: bool,
    /// Commands dropped because the ring was full (debug HUD surfaces this).
    dropped: u32,
}

#[cfg(target_arch = "wasm32")]
mod wiring {
    use super::Meters;
    use leptos::prelude::*;
    use phazor_core::Command;
    use std::cell::RefCell;

    thread_local! {
        static PHAZOR: RefCell<Option<phazor_web::Phazor>> = const { RefCell::new(None) };
        static DROPPED: RefCell<u32> = const { RefCell::new(0) };
    }

    /// Boot the engine (must be a user gesture) and start the telemetry loop.
    pub fn power_on(meters: RwSignal<Meters>, powered: RwSignal<bool>) {
        leptos::task::spawn_local(async move {
            match phazor_web::start().await {
                Ok(p) => {
                    PHAZOR.with(|slot| *slot.borrow_mut() = Some(p));
                    powered.set(true);
                    crate::raf::raf_loop(move || {
                        PHAZOR.with(|slot| {
                            if let Some(p) = slot.borrow_mut().as_mut() {
                                let mut latest = None;
                                while let Ok(frame) = p.meters.pop() {
                                    latest = Some(frame);
                                }
                                if let Some(f) = latest {
                                    meters.update(|m| {
                                        m.peak_l = f.peak_l;
                                        m.peak_r = f.peak_r;
                                        m.rms_l = f.rms_l;
                                        m.rms_r = f.rms_r;
                                        m.beats = f.beats;
                                        m.voices = f.voices;
                                        m.playing = f.playing;
                                        m.dropped = DROPPED.with(|d| *d.borrow());
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
mod wiring {
    //! Native stub so `cargo clippy --workspace` (host target) checks the
    //! view code; the panel is browser-only at runtime.
    use super::Meters;
    use leptos::prelude::*;
    use phazor_core::Command;

    pub fn power_on(_meters: RwSignal<Meters>, _powered: RwSignal<bool>) {}
    pub fn send(_cmd: Command) {}
}

/// The `/phazor` route.
#[component]
pub fn PhazorPage() -> impl IntoView {
    let powered = RwSignal::new(false);
    let meters = RwSignal::new(Meters::default());
    let steps = RwSignal::new([false; 16]);
    let tempo = RwSignal::new(120.0f64);

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
                <section class="transport">
                    <button
                        class="xport"
                        class:lit=move || meters.get().playing
                        on:click=move |_| wiring::send(Command::Play)
                    >"▶ play"</button>
                    <button class="xport" on:click=move |_| wiring::send(Command::Stop)>"■ stop"</button>
                    <button class="xport panic" on:click=move |_| wiring::send(Command::AllNotesOff)>"✕ panic"</button>
                    <label class="tempo">
                        "tempo "
                        <input
                            type="range" min="60" max="200" step="1"
                            prop:value=move || tempo.get().to_string()
                            on:input=move |ev| {
                                let bpm: f64 = event_target_value(&ev).parse().unwrap_or(120.0);
                                tempo.set(bpm);
                                wiring::send(Command::SetTempo(bpm));
                            }
                        />
                        <span class="val">{move || format!("{:.0}", tempo.get())}</span>
                    </label>
                </section>

                <section class="steps">
                    {(0..StepSequencer::LEN)
                        .map(|i| {
                            view! {
                                <button
                                    class="step"
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

                <section class="knobs">
                    <Param id=ParamId::FilterCutoff label="cutoff" min=0.0 max=1.0 step=0.001
                        to_value=|v| 20.0 * 10.0f32.powf(3.0 * v)
                        show=|v| format!("{:.0} Hz", 20.0 * 10.0f32.powf(3.0 * v))
                        init=0.885 />
                    <Param id=ParamId::FilterQ label="resonance" min=0.5 max=10.0 step=0.01
                        to_value=|v| v show=|v| format!("{v:.2}") init=0.707 />
                    <Param id=ParamId::OscBrightness label="brightness" min=0.0 max=1.0 step=0.01
                        to_value=|v| v show=|v| format!("{v:.2}") init=0.35 />
                    <Param id=ParamId::MasterGain label="master" min=0.0 max=1.2 step=0.01
                        to_value=|v| v show=|v| format!("{v:.2}") init=0.8 />
                </section>

                <section class="readout">
                    <Meter label="L" peak=move || meters.get().peak_l rms=move || meters.get().rms_l />
                    <Meter label="R" peak=move || meters.get().peak_r rms=move || meters.get().rms_r />
                    <div class="stats">
                        <span>{move || format!("beat {:.2}", meters.get().beats)}</span>
                        <span>{move || format!("voices {}", meters.get().voices)}</span>
                        <span class:warn={move || meters.get().dropped > 0}>
                            {move || format!("dropped {}", meters.get().dropped)}
                        </span>
                    </div>
                </section>
            </Show>
        </main>
    }
}

/// One labeled parameter slider bound to an engine [`ParamId`].
#[component]
fn Param(
    /// Engine parameter this slider drives.
    id: ParamId,
    /// Display label.
    label: &'static str,
    /// Slider minimum (UI units).
    min: f32,
    /// Slider maximum (UI units).
    max: f32,
    /// Slider step (UI units).
    step: f32,
    /// Map slider position → engine value (e.g. log cutoff).
    to_value: fn(f32) -> f32,
    /// Map slider position → display string.
    show: fn(f32) -> String,
    /// Initial slider position (UI units).
    init: f32,
) -> impl IntoView {
    let pos = RwSignal::new(init);
    view! {
        <label class="param">
            <span class="name">{label}</span>
            <input
                type="range"
                min=min max=max step=step
                prop:value=move || pos.get().to_string()
                on:input=move |ev| {
                    let v: f32 = event_target_value(&ev).parse().unwrap_or(init);
                    pos.set(v);
                    wiring::send(Command::SetParam { id, value: to_value(v) });
                }
            />
            <span class="val">{move || show(pos.get())}</span>
        </label>
    }
}

/// A peak+RMS bar meter.
#[component]
fn Meter(
    /// Channel label.
    label: &'static str,
    /// Peak level 0..=1 (post-limiter, so it genuinely can't exceed 1).
    peak: impl Fn() -> f32 + Send + Sync + 'static,
    /// RMS level 0..=1.
    rms: impl Fn() -> f32 + Send + Sync + 'static,
) -> impl IntoView {
    view! {
        <div class="meter">
            <span class="ch">{label}</span>
            <div class="bar">
                <div class="rms" style:width=move || format!("{:.1}%", rms() * 100.0)></div>
                <div class="peak" style:left=move || format!("{:.1}%", (peak() * 100.0).min(99.5))></div>
            </div>
        </div>
    }
}
