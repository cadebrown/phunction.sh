//! The engine: commands in, stereo audio + telemetry out.
//!
//! Signal flow (per sample):
//!
//! ```text
//! score ──┐                       ┌─ drone bus ─┐        ┌→ ping-pong delay ─┐
//! pattern ┼→ voices (pan/layer) ──┼─ arp bus  ──┼─ sends ┤                   ├→ drive → out
//! live in ┘                       └─ lead bus ──┘        └→ reverb  ←────────┘
//! ```
//!
//! The delay feeds the reverb: echoes bloom into wash, which is most of what
//! "dark ambient" means as a signal chain.

use crate::event::{Command, ParamId, PARAM_COUNT};
use crate::fx::{drive, PingPong, Reverb};
use crate::math::Smoothed;
use crate::meter::{BlockMeter, MeterFrame, SCOPE};
use crate::score::{Scale, Score};
use crate::seq::{SeqEvent, SeqEventKind, StepSequencer, MAX_EVENTS_PER_BLOCK};
use crate::spectrum::Spectrum;
use crate::transport::Transport;
use crate::voice::Voice;
use crate::{Sample, LAYER_ARP, LAYER_COUNT, LAYER_DRONE, LAYER_LEAD};

/// Fixed polyphony. Sixteen voices of four phasors each is nothing on any
/// hardware from the last decade; raise when a real limit is hit, with a
/// bench to justify it.
pub const VOICES: usize = 16;

/// Per-layer voice configuration derived each block from the user params.
#[derive(Clone, Copy)]
struct LayerCfg {
    attack: f32,
    decay: f32,
    sustain: f32,
    release: f32,
    cutoff: f32,
    q: f32,
    brightness: f32,
}

/// The phazor engine. Construct once on the audio thread; call
/// [`Engine::apply`] with drained commands and [`Engine::process`] per block.
///
/// Realtime contract: `new()` allocates the delay/reverb lines; after that,
/// no method on this type allocates, locks, or performs unbounded work.
pub struct Engine {
    transport: Transport,
    seq: StepSequencer,
    score: Score,
    voices: [Voice; VOICES],
    params: [Smoothed; PARAM_COUNT],
    delay: PingPong,
    reverb: Reverb,
    meter: BlockMeter,
    /// The on-thread analyzer feeding MeterFrame.bands.
    spectrum: Spectrum,
    /// Monotone counter stamping gate-ons for voice-steal ordering.
    note_counter: u64,
    /// Last 64-beat era the score evolved in (see `process`).
    era: u64,
    /// Slewed weather biases (brightness add, cutoff/verb/feedback muls) —
    /// the engine glides toward each era's [`crate::score::EraWeather`]
    /// targets so character changes are swells, never steps.
    wx_bright: f32,
    wx_cutoff: f32,
    wx_verb: f32,
    wx_fb: f32,
    /// Scratch for the per-block events (audio path: no alloc).
    events: heapless::Vec<SeqEvent, MAX_EVENTS_PER_BLOCK>,
}

impl Engine {
    /// A silent engine at 120 BPM.
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        let params = core::array::from_fn(|i| {
            let id = ParamId::ALL[i];
            // 15ms glide on every parameter: fast enough to feel immediate,
            // slow enough to never zipper. Smoothers run at *block* rate
            // (ticked once per process() call), hence the divided rate.
            Smoothed::new(
                id.default_value(),
                15.0,
                sample_rate / crate::QUANTUM as f32,
            )
        });
        Self {
            transport: Transport::new(f64::from(sample_rate), 120.0),
            seq: StepSequencer::default(),
            score: Score::default(),
            voices: [Voice::new(sample_rate); VOICES],
            params,
            delay: PingPong::new(sample_rate),
            reverb: Reverb::new(sample_rate),
            meter: BlockMeter::default(),
            spectrum: Spectrum::new(sample_rate),
            note_counter: 0,
            era: 0,
            wx_bright: 0.0,
            wx_cutoff: 1.0,
            wx_verb: 1.0,
            wx_fb: 1.0,
            events: heapless::Vec::new(),
        }
    }

    /// Read access for the UI/debug side (via snapshots, not shared refs).
    #[must_use]
    pub fn transport(&self) -> &Transport {
        &self.transport
    }

    /// The step sequencer (UI edits go through [`Command::SetStep`]; this
    /// accessor is for tests and native tooling).
    #[must_use]
    pub fn sequencer(&self) -> &StepSequencer {
        &self.seq
    }

    /// The generative score's current settings.
    #[must_use]
    pub fn score(&self) -> &Score {
        &self.score
    }

    /// Current smoothed value of a parameter.
    #[must_use]
    pub fn param(&self, id: ParamId) -> f32 {
        self.params[id.index()].get()
    }

    /// Apply one command. Called at block start for every drained command.
    pub fn apply(&mut self, cmd: Command) {
        match cmd {
            Command::Play => self.transport.play(),
            Command::Stop => {
                self.transport.stop();
                for v in &mut self.voices {
                    v.note_off();
                }
            }
            Command::SetTempo(bpm) => self.transport.set_bpm(bpm),
            Command::NoteOn { note, vel } => self.note_on(note, vel, LAYER_ARP),
            Command::NoteOff { note } => self.note_off(note, LAYER_ARP),
            Command::SetParam { id, value } => self.params[id.index()].set(value),
            Command::SetStep { index, step } => {
                // release whatever the old step is ringing — editing a
                // step must not orphan its note-off
                if let Some(old) = self.seq.step(usize::from(index)) {
                    self.note_off(old.note, LAYER_ARP);
                }
                self.seq.set_step(usize::from(index), step);
            }
            Command::AllNotesOff => {
                for v in &mut self.voices {
                    v.kill();
                }
            }
            Command::SetSeed(seed) => self.score.seed = seed,
            Command::SetScale(scale) => self.score.scale = Scale::from_u8(scale),
            Command::SeekBeats(beats) => {
                // release (not kill): tails ride across the jump gracefully
                for v in &mut self.voices {
                    v.note_off();
                }
                self.transport.seek_beats(beats);
            }
        }
    }

    /// Render one block into planar stereo buffers (Web Audio's layout).
    /// Both slices must be the same length. Returns the block's telemetry.
    #[allow(clippy::too_many_lines)]
    pub fn process(&mut self, left: &mut [Sample], right: &mut [Sample]) -> MeterFrame {
        debug_assert_eq!(left.len(), right.len());
        let block_len = left.len().min(right.len());

        // 1. Block-rate parameter update (see new() for why block rate).
        let cutoff =
            (self.smooth_block(ParamId::FilterCutoff) * self.wx_cutoff).clamp(80.0, 12_000.0);
        let q = self.smooth_block(ParamId::FilterQ);
        let att = self.smooth_block(ParamId::EnvAttackMs);
        let dec = self.smooth_block(ParamId::EnvDecayMs);
        let sus = self.smooth_block(ParamId::EnvSustain);
        let rel = self.smooth_block(ParamId::EnvReleaseMs);
        // the era's weather leans on the user's timbre without owning it:
        // brightness bias and cutoff multiplier ride ON the CV values
        let user_brightness =
            (self.smooth_block(ParamId::OscBrightness) + self.wx_bright).clamp(0.0, 1.0);
        let master = self.smooth_block(ParamId::MasterGain);
        let delay_mix = self.smooth_block(ParamId::DelayMix);
        let delay_fb = (self.smooth_block(ParamId::DelayFeedback) * self.wx_fb).clamp(0.0, 0.95);
        let reverb_mix = self.smooth_block(ParamId::ReverbMix);
        let reverb_size = (self.smooth_block(ParamId::ReverbSize) * self.wx_verb).clamp(0.0, 1.0);
        let drv = self.smooth_block(ParamId::Drive);
        let layer_gain = [
            self.smooth_block(ParamId::DroneLevel),
            self.smooth_block(ParamId::ArpLevel),
            self.smooth_block(ParamId::LeadLevel),
        ];
        let lead_density = self.smooth_block(ParamId::LeadDensity);

        // Per-layer voice character, derived from the one set of user CVs:
        // the drone is the user's sound slowed to geological time, the lead
        // is the same sound sharpened and lifted.
        let cfgs = [
            // NB: envelope times are one-pole time constants (τ, to 63%),
            // not total lengths — a voice frees itself ~11τ after gate-off,
            // so τ=800ms already means an eight-second audible tail.
            LayerCfg {
                attack: 900.0,
                decay: 900.0,
                sustain: 0.85,
                release: 800.0,
                cutoff: (240.0 + cutoff * 0.06).clamp(200.0, 1400.0),
                q: 0.8,
                brightness: 0.12,
            },
            LayerCfg {
                attack: att,
                decay: dec,
                sustain: sus,
                release: rel,
                cutoff,
                q,
                brightness: user_brightness,
            },
            LayerCfg {
                attack: 8.0,
                decay: 260.0,
                sustain: 0.45,
                release: 700.0,
                cutoff: (cutoff * 1.2).min(9_000.0),
                q: (q * 1.15).min(10.0),
                brightness: (user_brightness * 1.4).min(1.0),
            },
        ];
        let brightness: [f32; LAYER_COUNT] =
            [cfgs[0].brightness, cfgs[1].brightness, cfgs[2].brightness];
        for v in &mut self.voices {
            let c = cfgs[usize::from(v.layer()) % LAYER_COUNT];
            v.configure(c.attack, c.decay, c.sustain, c.release, c.cutoff, c.q);
        }

        // Tempo-synced delay: a dotted eighth behind the beat.
        let frames_per_beat = 60.0 / self.transport.bpm() * self.transport.sample_rate();
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        self.delay
            .set_delay_frames((frames_per_beat * 0.75) as usize);

        // The weather never sits still: every 64 beats the seed takes one
        // deterministic hash step, so the arp skips and lead choices evolve
        // forever without ever repeating — and a resumed set evolves the
        // same way it would have (the walk is a function of position).
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let era = (self.transport.beats() / 64.0) as u64;
        if self.transport.playing() && era != self.era {
            self.era = era;
            self.score.seed = self.score.seed.wrapping_mul(0x9E37_79B9).wrapping_add(1);
            // the progression may change with the seed: release the old
            // chord gracefully so no drone voice is orphaned mid-era
            for v in &mut self.voices {
                if v.layer() == LAYER_DRONE {
                    v.note_off();
                }
            }
            // tempo weather steps once per era (the delay's tape-glide
            // absorbs the retune; sparse set_drift calls keep the
            // beat-rebase rounding negligible)
            self.transport.set_drift(self.score.weather().tempo_mul);
        }
        // timbre and space glide toward this era's weather (~4s swells)
        let wx = self.score.weather();
        let k = 0.0007;
        self.wx_bright += (wx.bright - self.wx_bright) * k;
        self.wx_cutoff += (wx.cutoff_mul - self.wx_cutoff) * k;
        self.wx_verb += (wx.verb_mul - self.wx_verb) * k;
        self.wx_fb += (wx.feedback_mul - self.wx_fb) * k;

        // 2. This block's events: the user pattern + the generative score,
        //    offset-sorted, note-offs before note-ons at equal offsets so a
        //    retriggered tone re-attacks legato instead of double-voicing.
        self.events.clear();
        self.seq
            .events_for_block(&self.transport, block_len, &mut self.events);
        self.score
            .events_for_block(&self.transport, block_len, &mut self.events, lead_density);
        self.events
            .sort_unstable_by_key(|e| (e.offset, matches!(e.kind, SeqEventKind::NoteOn { .. })));

        // 3. Render, splitting the buffer at each event offset so note
        //    starts/ends land on their exact frame.
        let mut scope = [0.0f32; SCOPE];
        let mut cursor = 0usize;
        let mut ev_ix = 0usize;
        while cursor < block_len {
            while ev_ix < self.events.len() && self.events[ev_ix].offset == cursor {
                match self.events[ev_ix].kind {
                    SeqEventKind::NoteOn { note, vel, layer } => self.note_on(note, vel, layer),
                    SeqEventKind::NoteOff { note, layer } => self.note_off(note, layer),
                }
                ev_ix += 1;
            }
            let until = self
                .events
                .get(ev_ix)
                .map_or(block_len, |e| e.offset.min(block_len));
            for i in cursor..until {
                // per-layer stereo buses
                let mut bus = [(0.0f32, 0.0f32); LAYER_COUNT];
                for v in &mut self.voices {
                    let layer = usize::from(v.layer()) % LAYER_COUNT;
                    let s = v.tick(brightness[layer]);
                    if s != 0.0 {
                        let (pl, pr) = v.pan();
                        bus[layer].0 += s * pl;
                        bus[layer].1 += s * pr;
                    }
                }
                for (b, g) in bus.iter_mut().zip(layer_gain) {
                    b.0 *= g;
                    b.1 *= g;
                }
                let dry_l = bus[0].0 + bus[1].0 + bus[2].0;
                let dry_r = bus[0].1 + bus[1].1 + bus[2].1;

                // sends: arps and leads echo, the drone mostly washes
                let send_d_l = bus[0].0 * 0.12 + bus[1].0 * 0.55 + bus[2].0 * 0.65;
                let send_d_r = bus[0].1 * 0.12 + bus[1].1 * 0.55 + bus[2].1 * 0.65;
                let (dl, dr) = self.delay.tick(send_d_l, send_d_r, delay_fb);

                let verb_in_l = bus[0].0 * 0.6 + bus[1].0 * 0.3 + bus[2].0 * 0.45 + dl * 0.5;
                let verb_in_r = bus[0].1 * 0.6 + bus[1].1 * 0.3 + bus[2].1 * 0.45 + dr * 0.5;
                let (vl, vr) = self.reverb.tick(verb_in_l, verb_in_r, reverb_size);

                // master: sum, soft-knee, saturate. The rational knee
                // (x / (1 + k|x|)) bounds the slope BEFORE the tanh — a
                // slammed bus rounds off instead of squaring up (the storm
                // test hears squared-up saturation as a click; a limiter
                // is gain staging, not a distortion effect).
                let pre_l = (dry_l * 0.35 + dl * delay_mix + vl * reverb_mix) * master;
                let pre_r = (dry_r * 0.35 + dr * delay_mix + vr * reverb_mix) * master;
                let knee_l = pre_l / (1.0 + 0.45 * pre_l.abs());
                let knee_r = pre_r / (1.0 + 0.45 * pre_r.abs());
                let out_l = drive(knee_l, drv);
                let out_r = drive(knee_r, drv);
                left[i] = out_l;
                right[i] = out_r;
                self.meter.tick(out_l, out_r);
                scope[i * SCOPE / block_len.max(1)] = f32::midpoint(out_l, out_r);
            }
            cursor = until;
        }

        // 4. Advance musical time by what we actually rendered.
        self.transport.advance(block_len);

        // 5. Hygiene + telemetry. The orphan watchdog releases any short-
        // note voice (arp/lead) still holding past four beats: reseeds and
        // step edits can strand a note-off, and stranded sustains stack
        // until the master saturates — the storm test found this as a
        // near-square tanh edge, not a control click.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let max_hold = (frames_per_beat * 4.0) as u64;
        let now_frame = self.transport.frame();
        let mut sounding = 0u8;
        for v in &mut self.voices {
            v.flush();
            if v.sounding().is_some()
                && v.layer() != LAYER_DRONE
                && now_frame.saturating_sub(v.born()) > max_hold
            {
                v.note_off();
            }
            sounding += u8::from(v.sounding().is_some());
        }
        self.delay.flush();
        self.reverb.flush();
        self.spectrum.analyze(&left[..block_len]);
        let (peak_l, peak_r, rms_l, rms_r) = self.meter.finish();
        MeterFrame {
            frame: self.transport.frame(),
            beats: self.transport.beats(),
            peak_l,
            peak_r,
            rms_l,
            rms_r,
            voices: sounding,
            playing: self.transport.playing(),
            bands: self.spectrum.levels(),
            scope,
        }
    }

    /// Advance a parameter's block-rate smoother one step.
    fn smooth_block(&mut self, id: ParamId) -> f32 {
        self.params[id.index()].tick()
    }

    fn note_on(&mut self, note: u8, vel: u8, layer: u8) {
        self.note_counter += 1;
        let layer_ix = usize::from(layer) % LAYER_COUNT;
        // Placement is a musical decision the engine owns: the drone is
        // wide, the arp ping-pongs, the lead wanders near center.
        #[allow(clippy::cast_precision_loss)]
        let pan = match layer {
            LAYER_DRONE => {
                if note.is_multiple_of(2) {
                    -0.7
                } else {
                    0.7
                }
            }
            LAYER_LEAD => ((self.note_counter % 7) as f32 / 3.0 - 1.0) * 0.3,
            _ => {
                if self.note_counter.is_multiple_of(2) {
                    -0.55
                } else {
                    0.55
                }
            }
        };
        let spread = [0.85, 0.15, 0.35][layer_ix];
        // Reuse a voice already sounding this note in this layer
        // (retrigger), else an idle voice, else steal the quietest of the
        // same layer, else the globally quietest.
        let slot = self
            .voices
            .iter()
            .position(|v| v.sounding() == Some(note) && v.layer() == layer)
            .or_else(|| self.voices.iter().position(|v| v.sounding().is_none()))
            .or_else(|| quietest(&self.voices, Some(layer)))
            .or_else(|| quietest(&self.voices, None))
            .unwrap_or(0);
        self.voices[slot].steal_to(
            note,
            vel,
            self.note_counter,
            layer,
            pan,
            spread,
            self.transport.frame(),
        );
    }

    fn note_off(&mut self, note: u8, layer: u8) {
        for v in &mut self.voices {
            if v.sounding() == Some(note) && v.layer() == layer {
                v.note_off();
            }
        }
    }
}

/// Index of the quietest-then-oldest voice, optionally restricted to one
/// layer. Enumerates before filtering so indices stay valid.
fn quietest(voices: &[Voice], layer: Option<u8>) -> Option<usize> {
    voices
        .iter()
        .enumerate()
        .filter(|(_, v)| layer.is_none_or(|l| v.layer() == l))
        .min_by(|(_, a), (_, b)| {
            (a.level(), a.age())
                .partial_cmp(&(b.level(), b.age()))
                .unwrap_or(core::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seq::Step;
    use crate::QUANTUM;

    fn render_seconds(engine: &mut Engine, secs: f32) -> (f32, u64) {
        let mut l = [0.0f32; QUANTUM];
        let mut r = [0.0f32; QUANTUM];
        let blocks = (secs * 48_000.0 / QUANTUM as f32) as usize;
        let mut peak = 0.0f32;
        let mut last_frame = 0;
        for _ in 0..blocks {
            let m = engine.process(&mut l, &mut r);
            peak = peak.max(m.peak_l).max(m.peak_r);
            last_frame = m.frame;
        }
        (peak, last_frame)
    }

    #[test]
    fn silent_by_default() {
        let mut e = Engine::new(48_000.0);
        let (peak, _) = render_seconds(&mut e, 0.2);
        assert_eq!(peak, 0.0);
    }

    #[test]
    fn note_on_makes_sound_and_note_off_decays() {
        let mut e = Engine::new(48_000.0);
        e.apply(Command::NoteOn { note: 60, vel: 110 });
        let (peak, _) = render_seconds(&mut e, 0.3);
        assert!(peak > 0.01, "audible after note on, got {peak}");
        e.apply(Command::NoteOff { note: 60 });
        render_seconds(&mut e, 6.0); // release + delay echoes + reverb wash
        let (tail, _) = render_seconds(&mut e, 0.2);
        assert!(tail < 1e-3, "silent after tails die, got {tail}");
    }

    #[test]
    fn eras_breathe_the_tempo() {
        // run the clock across several eras: the effective bpm must drift
        // off the user's 120 (weather) yet never leave the ±6% band
        let mut e = Engine::new(48_000.0);
        e.apply(Command::Play);
        let mut l = [0.0f32; 128];
        let mut r = [0.0f32; 128];
        let mut bpms = std::collections::BTreeSet::new();
        // 3 eras at 64 beats each @120bpm ≈ 96s ≈ 36k blocks
        for _ in 0..36_000 {
            e.process(&mut l, &mut r);
            bpms.insert((e.transport.bpm() * 10.0) as i64);
        }
        assert!(
            bpms.iter().all(|b| (1128..=1272).contains(b)),
            "tempo left the band: {bpms:?}"
        );
        assert!(bpms.len() >= 2, "tempo never drifted: {bpms:?}");
    }

    #[test]
    fn playing_generates_the_score() {
        let mut e = Engine::new(48_000.0);
        // No user steps at all: the generative score alone must sound.
        e.apply(Command::Play);
        let (peak, frames) = render_seconds(&mut e, 2.0);
        assert!(peak > 0.01, "the weather must sound on its own");
        assert!(frames > 0);
    }

    #[test]
    fn sequencer_steps_still_fire() {
        let mut e = Engine::new(48_000.0);
        // Silence the generative layers; only the user pattern remains.
        for (id, v) in [
            (ParamId::DroneLevel, 0.0),
            (ParamId::LeadLevel, 0.0),
            (ParamId::LeadDensity, 0.0),
        ] {
            e.apply(Command::SetParam { id, value: v });
        }
        e.apply(Command::SetStep {
            index: 0,
            step: Some(Step {
                note: 48,
                vel: 120,
                gate: 0.9,
            }),
        });
        e.apply(Command::Play);
        let (peak, _) = render_seconds(&mut e, 1.0);
        assert!(peak > 0.01, "pattern steps must sound");
    }

    #[test]
    fn output_is_always_bounded_and_finite() {
        let mut e = Engine::new(48_000.0);
        // Worst case: everything loud, all layers, playing, saturated.
        e.apply(Command::SetParam {
            id: ParamId::MasterGain,
            value: 1.5,
        });
        e.apply(Command::SetParam {
            id: ParamId::Drive,
            value: 1.0,
        });
        e.apply(Command::Play);
        for n in 0..VOICES as u8 {
            e.apply(Command::NoteOn {
                note: 36 + n * 3,
                vel: 127,
            });
        }
        let mut l = [0.0f32; QUANTUM];
        let mut r = [0.0f32; QUANTUM];
        for _ in 0..2000 {
            e.process(&mut l, &mut r);
            for s in l.iter().chain(r.iter()) {
                assert!(s.is_finite());
                assert!(s.abs() <= 1.0, "output ceiling breached: {s}");
            }
        }
    }

    #[test]
    fn stop_rewinds_and_silences() {
        let mut e = Engine::new(48_000.0);
        e.apply(Command::Play);
        render_seconds(&mut e, 1.0);
        e.apply(Command::Stop);
        render_seconds(&mut e, 8.0); // drone release + wash
        let (peak, frame) = render_seconds(&mut e, 0.1);
        assert!(peak < 1e-3, "tails must die after stop, got {peak}");
        assert_eq!(frame, 0, "stop must rewind to zero");
    }

    /// THE LIVE-PERFORMANCE INVARIANT: rerouting, retuning, reseeding,
    /// editing steps, seeking, starting — none of it may click. Parameters
    /// glide (block-rate smoothers), commands land at block boundaries,
    /// voices release instead of cutting. This test storms the command
    /// surface mid-render and fails on any sample-to-sample jump that a
    /// human would hear as a pop. Keep it passing; it is a design standard,
    /// not a unit test.
    #[test]
    fn command_storms_never_click() {
        let mut e = Engine::new(48_000.0);
        e.apply(Command::Play);
        let mut l = [0.0f32; QUANTUM];
        let mut r = [0.0f32; QUANTUM];
        // let the world come up first
        for _ in 0..400 {
            e.process(&mut l, &mut r);
        }
        let mut hash = 0x1234_5678u32;
        let mut rng = move || {
            hash ^= hash << 13;
            hash ^= hash >> 17;
            hash ^= hash << 5;
            hash
        };
        let mut prev = (l[QUANTUM - 1], r[QUANTUM - 1]);
        let mut worst = 0.0f32;
        for block in 0..2000u32 {
            // a storm: several commands every block, all surfaces
            for _ in 0..3 {
                let v = (rng() % 1000) as f32 / 1000.0;
                match rng() % 10 {
                    // NB: content stays band-limited (≤1.2 kHz) because the
                    // click detector is a slope bound — a legitimate 6 kHz
                    // partial swings 0.39/sample and would false-positive.
                    // The CONTROL surface is still fully stormed; that is
                    // what the invariant is about.
                    0 => e.apply(Command::SetParam {
                        id: ParamId::FilterCutoff,
                        value: 200.0 + v * 1_000.0,
                    }),
                    1 => e.apply(Command::SetParam {
                        id: ParamId::MasterGain,
                        value: 0.2 + v,
                    }),
                    2 => e.apply(Command::SetParam {
                        id: ParamId::DelayMix,
                        value: v,
                    }),
                    3 => e.apply(Command::SetParam {
                        id: ParamId::ReverbMix,
                        value: v,
                    }),
                    4 => e.apply(Command::SetParam {
                        id: ParamId::DroneLevel,
                        value: v,
                    }),
                    5 => e.apply(Command::SetSeed(rng())),
                    6 => e.apply(Command::SetScale((rng() % 3) as u8)),
                    7 => e.apply(Command::SetStep {
                        index: (rng() % 16) as u8,
                        step: (rng() % 2 == 0).then(|| Step {
                            note: 40 + (rng() % 24) as u8,
                            vel: 100,
                            gate: 0.5,
                        }),
                    }),
                    8 => e.apply(Command::SetParam {
                        id: ParamId::LeadDensity,
                        value: v,
                    }),
                    _ => e.apply(Command::SetTempo(60.0 + f64::from(v) * 80.0)),
                }
            }
            e.process(&mut l, &mut r);
            for i in 0..QUANTUM {
                let dl = (l[i] - prev.0).abs();
                let dr = (r[i] - prev.1).abs();
                worst = worst.max(dl).max(dr);
                assert!(
                    dl < 0.25 && dr < 0.25,
                    "audible click at block {block} sample {i}: Δl={dl:.3} Δr={dr:.3}"
                );
                prev = (l[i], r[i]);
            }
        }
        assert!(worst > 0.0, "the storm must actually produce audio");
    }

    #[test]
    fn seek_is_silent_and_lands_on_the_beat() {
        let mut e = Engine::new(48_000.0);
        e.apply(Command::Play);
        let mut l = [0.0f32; QUANTUM];
        let mut r = [0.0f32; QUANTUM];
        for _ in 0..400 {
            e.process(&mut l, &mut r);
        }
        let mut prev = (l[QUANTUM - 1], r[QUANTUM - 1]);
        e.apply(Command::SeekBeats(64.0));
        let m = e.process(&mut l, &mut r);
        for i in 0..QUANTUM {
            let dl = (l[i] - prev.0).abs();
            let dr = (r[i] - prev.1).abs();
            assert!(dl < 0.25 && dr < 0.25, "seek clicked at sample {i}");
            prev = (l[i], r[i]);
        }
        assert!(
            m.beats >= 64.0,
            "must land at the seek target, got {}",
            m.beats
        );
    }

    #[test]
    fn reseed_changes_the_render() {
        let capture = |seed: Option<u32>| {
            let mut e = Engine::new(48_000.0);
            if let Some(s) = seed {
                e.apply(Command::SetSeed(s));
            }
            e.apply(Command::Play);
            let mut l = [0.0f32; QUANTUM];
            let mut r = [0.0f32; QUANTUM];
            let mut sig = 0.0f64;
            for _ in 0..2000 {
                e.process(&mut l, &mut r);
                sig += l.iter().map(|s| f64::from(s.abs())).sum::<f64>();
            }
            sig
        };
        let a = capture(None);
        let b = capture(Some(99));
        assert!((a - b).abs() > 1e-6, "reseeding must change the music");
    }

    #[test]
    fn stereo_field_is_actually_stereo() {
        let mut e = Engine::new(48_000.0);
        e.apply(Command::Play);
        let mut l = [0.0f32; QUANTUM];
        let mut r = [0.0f32; QUANTUM];
        let mut diff = 0.0f64;
        for _ in 0..4000 {
            e.process(&mut l, &mut r);
            for (a, b) in l.iter().zip(r.iter()) {
                diff += f64::from((a - b).abs());
            }
        }
        assert!(diff > 1.0, "left and right must differ (pan/ping-pong)");
    }
}
