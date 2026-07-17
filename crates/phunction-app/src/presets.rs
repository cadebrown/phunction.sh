//! The worlds: whole-machine presets. One press rewrites score, voice,
//! space, sequence, mind, and patch — every cap and needle tells the
//! truth afterwards because the preset writes the SIGNALS, not just the
//! engine.

use crate::phazor_panel::{wiring, Cv};
use leptos::prelude::*;
use phazor_core::{Command, ParamId, Step};

/// Pad notes for the user pattern (A-minor pentatonic with octave drops).
/// The pads ride the arp layer *on top of* the generative score.
pub(crate) const RIFF: [u8; 16] = [
    45, 57, 48, 57, 45, 55, 48, 60, 45, 57, 52, 57, 43, 55, 48, 62,
];

/// One whole-machine state: score, tempo, voice, space, minds. VISION calls
/// these worlds; each is a different weather system over the same machine.
pub struct Preset {
    pub(crate) name: &'static str,
    pub(crate) tempo: f64,
    pub(crate) pattern: [bool; 16],
    pub(crate) cutoff: f32,
    pub(crate) resonance: f32,
    pub(crate) brightness: f32,
    pub(crate) master: f32,
    pub(crate) delay_mix: f32,
    pub(crate) delay_fb: f32,
    pub(crate) verb_mix: f32,
    pub(crate) verb_size: f32,
    pub(crate) drive: f32,
    pub(crate) drone: f32,
    pub(crate) arps: f32,
    pub(crate) lead: f32,
    pub(crate) density: f32,
    pub(crate) seed: u32,
    pub(crate) scale: u8,
    pub(crate) citadel: crate::fractal::CitadelParams,
    pub(crate) mind: &'static str,
    pub(crate) patch: &'static str,
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
        mind: "silk",
        patch: "k = knob 0.3\nl = lfo rate=k depth=0.35\nl -> mind.hue\nb = beat\ns = slew in=b amount=0.95\ns -> mind.warp",
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
        mind: "gyroid",
        patch: "a = audio-in\ns = slew in=a amount=0.9\ns -> mind.warp",
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
        mind: "gasket",
        patch: "a = audio-in\ne = expr \"a*0.6 + 0.2*tri(t*0.05)\" a=a\ne -> mind.dolly",
    },
];

/// Rewrite the whole machine to a preset: engine commands + fractal state
/// + every Cv signal, so every cap and needle tells the truth.
pub(crate) fn apply_preset(
    p: &'static Preset,
    steps: RwSignal<[Option<(u8, u8)>; 16]>,
    citadel: RwSignal<crate::fractal::CitadelParams>,
    tempo: RwSignal<f32>,
    cv: Cv,
) {
    #[allow(clippy::cast_possible_truncation)]
    tempo.set(p.tempo as f32);
    wiring::send(Command::SetTempo(p.tempo));
    let mut pat = [None; 16];
    for (i, &on) in p.pattern.iter().enumerate() {
        pat[i] = on.then_some((RIFF[i], 108));
    }
    steps.set(pat);
    for (i, st) in pat.iter().enumerate() {
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
    // a world rewrites the WHOLE machine — the mind and the patch too
    crate::fractal::request_mind(p.mind);
    crate::patchbay::request_patch(p.patch);
}
