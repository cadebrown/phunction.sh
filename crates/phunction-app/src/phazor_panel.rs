//! The phazor panel: transport, step grid, knobs, meters, FX, weather.
//!
//! UI-side rules: every engine interaction is a [`Command`] pushed onto the
//! ring (never shared state), and everything displayed comes back through
//! [`MeterFrame`] telemetry. The UI is a *client* of the engine, exactly as
//! a MIDI controller would be — which is what makes it replaceable, and
//! what will let external controllers drive the same surface later.

use crate::fractal::CitadelRack;
use crate::presets::{apply_preset, PRESETS, RIFF};
use crate::rack::{Knob, Led, LedMeter, RackPanel};
use leptos::prelude::*;
use phazor_core::meter::SCOPE;
use phazor_core::{Command, ParamId, Scale, Step, StepSequencer};

/// MIDI note → display name ("a2", "c#3") — lowercase, the machine's voice.
fn note_name(n: u8) -> String {
    const NAMES: [&str; 12] = [
        "c", "c#", "d", "d#", "e", "f", "f#", "g", "g#", "a", "a#", "b",
    ];
    format!("{}{}", NAMES[usize::from(n % 12)], i16::from(n / 12) - 1)
}

/// Live telemetry mirrored into signals for display.
#[derive(Clone, Copy)]
pub(crate) struct Meters {
    pub(crate) peak_l: f32,
    pub(crate) peak_r: f32,
    pub(crate) rms_l: f32,
    pub(crate) rms_r: f32,
    pub(crate) beats: f64,
    pub(crate) voices: u8,
    pub(crate) playing: bool,
    /// Commands dropped because the ring was full (debug HUD surfaces this).
    pub(crate) dropped: u32,
    /// 16-band spectrum mirror.
    pub(crate) bands: [f32; phazor_core::BANDS],
    /// Oscilloscope trace mirror.
    pub(crate) scope: [f32; SCOPE],
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
pub(crate) struct Cv {
    pub(crate) cutoff: RwSignal<f32>,
    pub(crate) resonance: RwSignal<f32>,
    pub(crate) brightness: RwSignal<f32>,
    pub(crate) master: RwSignal<f32>,
    pub(crate) delay_mix: RwSignal<f32>,
    pub(crate) delay_fb: RwSignal<f32>,
    pub(crate) verb_mix: RwSignal<f32>,
    pub(crate) verb_size: RwSignal<f32>,
    pub(crate) drive: RwSignal<f32>,
    pub(crate) drone: RwSignal<f32>,
    pub(crate) arps: RwSignal<f32>,
    pub(crate) lead: RwSignal<f32>,
    pub(crate) density: RwSignal<f32>,
    pub(crate) seed: RwSignal<u32>,
    pub(crate) scale: RwSignal<u8>,
}

impl Cv {
    pub(crate) fn new() -> Self {
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
thread_local! {
    /// Latest transport state, mirrored for the space-bar toggle.
    static LAST_PLAYING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
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
                                    super::LAST_PLAYING.with(|p| p.set(f.playing));
                                    // the resume checkpoint: beats + playing,
                                    // stamped every ~2s — only while running
                                    // (a stopped engine must not clobber the
                                    // set's saved position with 0)
                                    if f.playing && f.frame % (48_000 * 2) < 4096 {
                                        save_state(
                                            "phazor:clock",
                                            &format!("{};{}", f.beats, u8::from(f.playing)),
                                        );
                                    }
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

    /// Persist a machine-state string (the live set survives reloads).
    pub fn save_state(key: &str, value: &str) {
        if let Some(s) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
            let _ = s.set_item(key, value);
        }
    }

    /// Read back a persisted machine-state string.
    pub fn load_state(key: &str) -> Option<String> {
        web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
            .and_then(|s| s.get_item(key).ok().flatten())
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
    #[allow(dead_code)]
    pub fn save_state(_key: &str, _value: &str) {}
    #[allow(dead_code)]
    pub fn load_state(_key: &str) -> Option<String> {
        None
    }
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
    let steps = RwSignal::new([None::<(u8, u8)>; 16]);
    let citadel = RwSignal::new(crate::fractal::CitadelParams::default());
    let mind = RwSignal::new("silk");
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
                match ev.key().as_str() {
                    "z" => crate::topbar::toggle_zen(),
                    // layouts name PANELS; transport/minds/worlds live in
                    // the topbar now and need no slot here
                    "1" => crate::rack::reorder::apply_layout(&[
                        "weather · the score writes itself",
                        "mind controls",
                    ]),
                    "2" => crate::rack::reorder::apply_layout(&[
                        "patchbay · the constructive graph",
                        "expr · a little language",
                        "shader · live wgsl",
                    ]),
                    "3" => crate::rack::reorder::apply_layout(&[
                        "voice",
                        "mix",
                        "weather · the score writes itself",
                        "fx · space",
                        "spectrum · 50 Hz → 14 kHz · 96 bands",
                        "scope",
                    ]),
                    " " => {
                        // the keyhint is a promise: space toggles transport
                        ev.prevent_default();
                        if LAST_PLAYING.with(std::cell::Cell::get) {
                            wiring::send(Command::Stop);
                        } else {
                            wiring::send(Command::Play);
                        }
                    }
                    "Escape" => {
                        crate::patchbay::cancel_gestures();
                        wiring::send(Command::AllNotesOff);
                    }
                    _ => {}
                }
            });
        if let Some(w) = web_sys::window() {
            let _ = w.add_event_listener_with_callback("keydown", on_key.as_ref().unchecked_ref());
        }
        on_key.forget();
    }

    let toggle_zen = move |_ev: leptos::ev::MouseEvent| crate::topbar::toggle_zen();

    // Every control change writes the machine state down; a reload is a
    // set change, not a reset (the live-performance invariant, UI side).
    // Gated on `hydrated`: the effect's first run at mount carries default
    // values and must not clobber the state we're about to restore.
    let hydrated = StoredValue::new(false);
    Effect::new(move |_| {
        let bits: String = steps
            .get()
            .iter()
            .map(|st| st.map_or("-".to_string(), |(n, v)| format!("{n}:{v}")))
            .collect::<Vec<_>>()
            .join(",");
        let c = citadel.get();
        let state = format!(
            "v3;{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{}",
            tempo.get(),
            cv.cutoff.get(),
            cv.resonance.get(),
            cv.brightness.get(),
            cv.master.get(),
            cv.delay_mix.get(),
            cv.delay_fb.get(),
            cv.verb_mix.get(),
            cv.verb_size.get(),
            cv.drive.get(),
            cv.drone.get(),
            cv.arps.get(),
            cv.lead.get(),
            cv.density.get(),
            cv.seed.get(),
            cv.scale.get(),
            bits,
            c.scale,
            c.warp,
            c.hue,
            c.dolly,
            u8::from(c.auto),
        );
        if hydrated.get_value() {
            wiring::save_state("phazor:state", &state);
        }
    });

    // Power-on is ignition into a *world*, not silence — but the world
    // waits for the viewport to claim its GPU device first (see gfx_gate:
    // audible playback can stall requestAdapter). A fallback timer starts
    // the music anyway if the viewport never reports (headless, WebGL2).
    let booted = StoredValue::new(false);
    let start_world = move || {
        if !booted.get_value() {
            booted.set_value(true);
            if !restore_session(steps, tempo, cv, citadel) {
                apply_preset(&PRESETS[0], steps, citadel, tempo, cv);
            }
            hydrated.set_value(true);
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

    // The step editor: tap toggles, vertical drag walks scale degrees
    // (7px = one degree, snapped to the world's scale so edits can't leave
    // the harmony), shift-drag writes velocity. Every edit lands live via
    // SetStep — storm-tested, so mid-performance rewrites never click.
    let send_step = move |i: usize| {
        let step = steps.get_untracked()[i].map(|(note, vel)| Step {
            note,
            vel,
            gate: 0.55,
        });
        wiring::send(Command::SetStep {
            index: i as u8,
            step,
        });
    };
    // (index, y0, note0, vel0, moved)
    let drag = StoredValue::new(None::<(usize, f64, u8, u8, bool)>);
    let step_down = move |i: usize, ev: web_sys::PointerEvent| {
        use wasm_bindgen::JsCast;
        if let Some(t) = ev
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        {
            let _ = t.set_pointer_capture(ev.pointer_id());
        }
        let (n0, v0) = steps.get_untracked()[i].unwrap_or((RIFF[i], 108));
        drag.set_value(Some((i, f64::from(ev.client_y()), n0, v0, false)));
    };
    let step_move = move |i: usize, ev: web_sys::PointerEvent| {
        let Some((idx, y0, n0, v0, moved)) = drag.get_value() else {
            return;
        };
        if idx != i {
            return;
        }
        let dy = y0 - f64::from(ev.client_y());
        if !moved && dy.abs() < 5.0 {
            return;
        }
        drag.set_value(Some((idx, y0, n0, v0, true)));
        if ev.shift_key() {
            let vel = (i32::from(v0) + (dy * 0.8) as i32).clamp(1, 127) as u8;
            steps.update(|s| s[i] = Some((n0, vel)));
        } else {
            let scale = Scale::from_u8(cv.scale.get_untracked());
            let degrees = (dy / 7.0).round() as i16;
            let note = scale.degree_step(n0, degrees, 45);
            steps.update(|s| s[i] = Some((note, v0)));
        }
        send_step(i);
    };
    let step_up = move |i: usize, _ev: web_sys::PointerEvent| {
        let Some((idx, _, n0, _, moved)) = drag.get_value() else {
            return;
        };
        if idx != i {
            return;
        }
        drag.set_value(None);
        if !moved {
            steps.update(|s| {
                s[i] = if s[i].is_some() {
                    None
                } else {
                    Some((n0, 108))
                };
            });
            send_step(i);
        }
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

    // Full-res analyzer: 96 log-spaced bands drawn as one smooth curve
    // (area + line), not a bar chart — insanely detailed, zero DOM churn.
    let analyzer_line = move || {
        let b = meters.get().bands;
        let mut d = String::with_capacity(b.len() * 12);
        for (i, v) in b.iter().enumerate() {
            use core::fmt::Write;
            let y = 32.0 - (v * 40.0).clamp(0.0, 31.0);
            let _ = write!(d, "{}{i},{y:.1} ", if i == 0 { "M" } else { "L" });
        }
        d
    };
    let analyzer_fill = move || {
        let b = meters.get().bands;
        let mut d = String::with_capacity(b.len() * 12 + 24);
        d.push_str("M0,32 ");
        for (i, v) in b.iter().enumerate() {
            use core::fmt::Write;
            let y = 32.0 - (v * 40.0).clamp(0.0, 31.0);
            let _ = write!(d, "L{i},{y:.1} ");
        }
        d.push_str("L95,32 Z");
        d
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
                <crate::topbar::Topbar meters=meters tempo=tempo mind=mind steps=steps citadel=citadel cv=cv />
                <div class="rack">
                    <div class="ws-col">
                    <RackPanel title="voice" class="span5" folded=true hue=145.0>
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

                    <RackPanel title="mix" class="span3" folded=true hue=55.0>
                        {cv_knob!("master", cv.master, ParamId::MasterGain, 0.0, 1.2, 55.0)}
                        <LedMeter label="L" level={Signal::derive(move || (meters.get().rms_l * 3.0).min(1.0))} />
                        <LedMeter label="R" level={Signal::derive(move || (meters.get().rms_r * 3.0).min(1.0))} />
                        <Led
                            on={Signal::derive(move || meters.get().peak_l.max(meters.get().peak_r) > 0.97)}
                            hue=10.0
                            label="clip"
                        />
                    </RackPanel>

                    <RackPanel title="weather · the score writes itself" class="span5" folded=true hue=280.0>
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

                    <RackPanel title="fx · space" class="span4" folded=true hue=190.0>
                        {cv_knob!("echo", cv.delay_mix, ParamId::DelayMix, 0.0, 1.0, 190.0)}
                        {cv_knob!("regen", cv.delay_fb, ParamId::DelayFeedback, 0.0, 0.9, 235.0)}
                        {cv_knob!("wash", cv.verb_mix, ParamId::ReverbMix, 0.0, 1.0, 280.0)}
                        {cv_knob!("cavern", cv.verb_size, ParamId::ReverbSize, 0.0, 1.0, 325.0)}
                        {cv_knob!("drive", cv.drive, ParamId::Drive, 0.0, 1.0, 10.0)}
                    </RackPanel>
                    </div>
                    <div class="ws-mid">
                        <div class="ws-mid-spacer"></div>
                    <RackPanel title="sequence · your riff over the weather" folded=true hue=100.0>
                        <section class="steps" style="width: 100%">
                            {(0..StepSequencer::LEN)
                                .map(|i| {
                                    view! {
                                        <button
                                            class="step"
                                            // Each step wears the hue of its phase angle (2πi/16).
                                            style=("--i", i.to_string())
                                            style=("--vel", move || {
                                                steps.get()[i].map_or("0".into(), |(_, v)| {
                                                    format!("{:.2}", f32::from(v) / 127.0)
                                                })
                                            })
                                            class:on=move || steps.get()[i].is_some()
                                            class:now=move || playhead() == Some(i)
                                            on:pointerdown=move |ev| step_down(i, ev)
                                            on:pointermove=move |ev| step_move(i, ev)
                                            on:pointerup=move |ev| step_up(i, ev)
                                            on:pointercancel=move |_| drag.set_value(None)
                                            aria-label=move || format!("step {}", i + 1)
                                        >
                                            <span class="note">
                                                {move || steps.get()[i].map_or_else(
                                                    || "·".to_string(),
                                                    |(n, _)| note_name(n),
                                                )}
                                            </span>
                                            <span class="vel-bar" aria-hidden="true"></span>
                                        </button>
                                    }
                                })
                                .collect_view()}
                        </section>
                    </RackPanel>

                    <crate::patchbay::Patchbay />
                    </div>
                    <div class="ws-col">
                    <CitadelRack params=citadel mind=mind />

                    <RackPanel title="scope" class="span3" folded=true hue=145.0>
                        <svg class="scope-lcd" viewBox="0 0 64 32" preserveAspectRatio="none" aria-label="oscilloscope: the master bus waveform">
                            <line class="scope-axis" x1="0" y1="16" x2="64" y2="16"></line>
                            <polyline class="scope-trace" points=scope_points></polyline>
                        </svg>
                        <Led on=Signal::derive(move || meters.get().rms_l + meters.get().rms_r > 0.005) hue=145.0 label="sig" />
                    </RackPanel>

                    <RackPanel title="spectrum · 50 Hz → 14 kHz · 96 bands" class="span12" folded=true hue=325.0>
                        <svg
                            class="analyzer"
                            viewBox="0 0 95 32"
                            preserveAspectRatio="none"
                            aria-label="full-resolution spectrum analyzer"
                        >
                            <defs>
                                <linearGradient id="specgrad" x1="0" y1="0" x2="1" y2="0">
                                    <stop offset="0%" stop-color="oklch(0.55 0.13 300)"></stop>
                                    <stop offset="40%" stop-color="oklch(0.6 0.14 280)"></stop>
                                    <stop offset="75%" stop-color="oklch(0.62 0.11 220)"></stop>
                                    <stop offset="100%" stop-color="oklch(0.65 0.1 190)"></stop>
                                </linearGradient>
                            </defs>
                            <path class="an-fill" d=analyzer_fill></path>
                            <path class="an-line" d=analyzer_line></path>
                        </svg>
                    </RackPanel>

                    <ExprRack meters=meters />
                    </div>
                </div>

                <div class="keyhints">
                    <span><kbd>"space"</kbd>" play/stop"</span>
                    <span><kbd>"esc"</kbd>" panic"</span>
                    <span><kbd>"z"</kbd>" zen"</span>
                    <span><kbd>"1/2/3"</kbd>" perform · patch · mix"</span>
                    <span><kbd>"shift"</kbd>"+drag knobs for fine control · double-click resets"</span>
                    <span>"steps: tap toggles · drag = pitch · "<kbd>"shift"</kbd>"+drag = velocity"</span>
                    <span><kbd>"`"</kbd>" debug"</span>
                </div>
                <button class="zen-toggle" on:click=toggle_zen>"zen"</button>
            </Show>
        </main>
    }
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
        <RackPanel title="expr · a little language" class="span9" folded=true hue=145.0>
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

/// Rebuild the whole machine from the persisted state; `false` if there is
/// none (first visit) and the opening world should play instead. The clock
/// checkpoint seeks the transport, so a reload resumes mid-set.
fn restore_session(
    steps: RwSignal<[Option<(u8, u8)>; 16]>,
    tempo: RwSignal<f32>,
    cv: Cv,
    citadel: RwSignal<crate::fractal::CitadelParams>,
) -> bool {
    let Some(state) = wiring::load_state("phazor:state") else {
        return false;
    };
    let parts: Vec<&str> = state.split(';').collect();
    if parts.len() != 23 || !matches!(parts[0], "v2" | "v3") {
        return false;
    }
    let f = |i: usize| parts[i].parse::<f32>().unwrap_or(0.5);
    let t = f(1);
    tempo.set(t);
    wiring::send(Command::SetTempo(f64::from(t)));
    for (sig, id, ix) in [
        (cv.cutoff, ParamId::FilterCutoff, 2),
        (cv.resonance, ParamId::FilterQ, 3),
        (cv.brightness, ParamId::OscBrightness, 4),
        (cv.master, ParamId::MasterGain, 5),
        (cv.delay_mix, ParamId::DelayMix, 6),
        (cv.delay_fb, ParamId::DelayFeedback, 7),
        (cv.verb_mix, ParamId::ReverbMix, 8),
        (cv.verb_size, ParamId::ReverbSize, 9),
        (cv.drive, ParamId::Drive, 10),
        (cv.drone, ParamId::DroneLevel, 11),
        (cv.arps, ParamId::ArpLevel, 12),
        (cv.lead, ParamId::LeadLevel, 13),
        (cv.density, ParamId::LeadDensity, 14),
    ] {
        let value = f(ix);
        sig.set(value);
        wiring::send(Command::SetParam { id, value });
    }
    let seed = parts[15].parse::<u32>().unwrap_or(0xC0FF_EE00);
    cv.seed.set(seed);
    wiring::send(Command::SetSeed(seed));
    let scale = parts[16].parse::<u8>().unwrap_or(0);
    cv.scale.set(scale);
    wiring::send(Command::SetScale(scale));
    let mut pattern = [None::<(u8, u8)>; 16];
    if parts[0] == "v3" {
        // v3: comma-joined `note:vel` per step, `-` for rests
        for (i, cell) in parts[17].split(',').take(16).enumerate() {
            pattern[i] = cell
                .split_once(':')
                .and_then(|(n, v)| Some((n.parse::<u8>().ok()?, v.parse::<u8>().ok()?)));
        }
    } else {
        // v2 carried on/off bits over the fixed riff
        for (i, c) in parts[17].chars().take(16).enumerate() {
            pattern[i] = (c == '1').then_some((RIFF[i], 108));
        }
    }
    steps.set(pattern);
    for (i, st) in pattern.iter().enumerate() {
        #[allow(clippy::cast_possible_truncation)]
        wiring::send(Command::SetStep {
            index: i as u8,
            step: st.map(|(note, vel)| Step {
                note,
                vel,
                gate: 0.55,
            }),
        });
    }
    citadel.update(|c| {
        let generation = c.gen + 1;
        *c = crate::fractal::CitadelParams {
            scale: f(18),
            warp: f(19),
            hue: f(20),
            dolly: f(21),
            auto: parts[22] != "0",
            gen: generation,
        };
    });
    // the clock checkpoint: land where the set left off
    if let Some(clock) = wiring::load_state("phazor:clock") {
        if let Some((beats, playing)) = clock.split_once(';') {
            if let Ok(b) = beats.parse::<f64>() {
                wiring::send(Command::SeekBeats(b));
            }
            if playing != "0" {
                wiring::send(Command::Play);
            }
        }
    } else {
        wiring::send(Command::Play);
    }
    true
}
