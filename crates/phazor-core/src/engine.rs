//! The engine: commands in, stereo audio + telemetry out.

use crate::event::{Command, ParamId, PARAM_COUNT};
use crate::math::Smoothed;
use crate::meter::{BlockMeter, MeterFrame};
use crate::seq::{SeqEvent, SeqEventKind, StepSequencer, MAX_EVENTS_PER_BLOCK};
use crate::transport::Transport;
use crate::voice::Voice;
use crate::Sample;

/// Fixed polyphony. Sixteen voices of three phasors each is nothing on any
/// hardware from the last decade; raise when a real limit is hit, with a
/// bench to justify it.
pub const VOICES: usize = 16;

/// The phazor engine. Construct once on the audio thread; call
/// [`Engine::apply`] with drained commands and [`Engine::process`] per block.
///
/// Realtime contract: after `new()`, no method on this type allocates,
/// locks, or performs unbounded work.
pub struct Engine {
    transport: Transport,
    seq: StepSequencer,
    voices: [Voice; VOICES],
    params: [Smoothed; PARAM_COUNT],
    meter: BlockMeter,
    /// Monotone counter stamping gate-ons for voice-steal ordering.
    note_counter: u64,
    /// Scratch for the sequencer's per-block events (audio path: no alloc).
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
            voices: [Voice::new(sample_rate); VOICES],
            params,
            meter: BlockMeter::default(),
            note_counter: 0,
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
            Command::NoteOn { note, vel } => self.note_on(note, vel),
            Command::NoteOff { note } => self.note_off(note),
            Command::SetParam { id, value } => self.params[id.index()].set(value),
            Command::SetStep { index, step } => self.seq.set_step(usize::from(index), step),
            Command::AllNotesOff => {
                for v in &mut self.voices {
                    v.kill();
                }
            }
        }
    }

    /// Render one block into planar stereo buffers (Web Audio's layout).
    /// Both slices must be the same length. Returns the block's telemetry.
    pub fn process(&mut self, left: &mut [Sample], right: &mut [Sample]) -> MeterFrame {
        debug_assert_eq!(left.len(), right.len());
        let block_len = left.len().min(right.len());

        // 1. Sequencer events for this frame range, offset-sorted.
        self.events.clear();
        self.seq
            .events_for_block(&self.transport, block_len, &mut self.events);

        // 2. Block-rate parameter update: smoothers tick once per block
        //    (constructed at block rate — see new()). Far cheaper than
        //    per-sample smoothing of 8 params × 16 voices, and a 15ms glide
        //    sampled every 2.7ms is indistinguishable by ear.
        let cutoff = self.smooth_block(ParamId::FilterCutoff);
        let q = self.smooth_block(ParamId::FilterQ);
        let att = self.smooth_block(ParamId::EnvAttackMs);
        let dec = self.smooth_block(ParamId::EnvDecayMs);
        let sus = self.smooth_block(ParamId::EnvSustain);
        let rel = self.smooth_block(ParamId::EnvReleaseMs);
        let brightness = self.smooth_block(ParamId::OscBrightness);
        let master = self.smooth_block(ParamId::MasterGain);
        for v in &mut self.voices {
            v.configure(att, dec, sus, rel, cutoff, q);
        }

        // 3. Render, splitting the buffer at each event offset so note
        //    starts/ends land on their exact frame.
        let mut cursor = 0usize;
        let mut ev_ix = 0usize;
        while cursor < block_len {
            while ev_ix < self.events.len() && self.events[ev_ix].offset == cursor {
                match self.events[ev_ix].kind {
                    SeqEventKind::NoteOn { note, vel } => self.note_on(note, vel),
                    SeqEventKind::NoteOff { note } => self.note_off(note),
                }
                ev_ix += 1;
            }
            let until = self
                .events
                .get(ev_ix)
                .map_or(block_len, |e| e.offset.min(block_len));
            for i in cursor..until {
                let mut mono = 0.0f32;
                for v in &mut self.voices {
                    mono += v.tick(brightness);
                }
                // Gentle tanh ceiling: a live instrument must never hand the
                // browser (or the PA) a runaway sample.
                let out = (mono * master * 0.25).tanh();
                left[i] = out;
                right[i] = out;
                self.meter.tick(out, out);
            }
            cursor = until;
        }

        // 4. Advance musical time by what we actually rendered.
        self.transport.advance(block_len);

        // 5. Hygiene + telemetry.
        let mut sounding = 0u8;
        for v in &mut self.voices {
            v.flush();
            sounding += u8::from(v.sounding().is_some());
        }
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
        }
    }

    /// Advance a parameter's block-rate smoother one step.
    fn smooth_block(&mut self, id: ParamId) -> f32 {
        self.params[id.index()].tick()
    }

    fn note_on(&mut self, note: u8, vel: u8) {
        self.note_counter += 1;
        // Reuse a voice already sounding this note (retrigger), else the
        // first idle voice, else steal the quietest-then-oldest.
        let slot = self
            .voices
            .iter()
            .position(|v| v.sounding() == Some(note))
            .or_else(|| self.voices.iter().position(|v| v.sounding().is_none()))
            .unwrap_or_else(|| {
                self.voices
                    .iter()
                    .enumerate()
                    .min_by(|(_, a), (_, b)| {
                        (a.level(), a.age())
                            .partial_cmp(&(b.level(), b.age()))
                            .unwrap_or(core::cmp::Ordering::Equal)
                    })
                    .map_or(0, |(i, _)| i)
            });
        self.voices[slot].note_on(note, vel, self.note_counter);
    }

    fn note_off(&mut self, note: u8) {
        for v in &mut self.voices {
            if v.sounding() == Some(note) {
                v.note_off();
            }
        }
    }
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
            peak = peak.max(m.peak_l);
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
        render_seconds(&mut e, 2.0); // let release die
        let (tail, _) = render_seconds(&mut e, 0.2);
        assert!(tail < 1e-4, "silent after release, got {tail}");
    }

    #[test]
    fn sequencer_drives_voices_when_playing() {
        let mut e = Engine::new(48_000.0);
        e.apply(Command::SetStep {
            index: 0,
            step: Some(Step {
                note: 48,
                vel: 120,
                gate: 0.9,
            }),
        });
        e.apply(Command::SetStep {
            index: 8,
            step: Some(Step {
                note: 55,
                vel: 120,
                gate: 0.9,
            }),
        });
        // Not playing yet: silence.
        let (peak, _) = render_seconds(&mut e, 0.2);
        assert_eq!(peak, 0.0);
        e.apply(Command::Play);
        let (peak, frames) = render_seconds(&mut e, 1.0);
        assert!(peak > 0.01, "sequencer should sound while playing");
        assert!(frames > 0, "transport should advance");
    }

    #[test]
    fn output_is_always_bounded_and_finite() {
        let mut e = Engine::new(48_000.0);
        // Worst case: every voice fortissimo, master high.
        e.apply(Command::SetParam {
            id: ParamId::MasterGain,
            value: 1.5,
        });
        for n in 0..VOICES as u8 {
            e.apply(Command::NoteOn {
                note: 36 + n * 3,
                vel: 127,
            });
        }
        let mut l = [0.0f32; QUANTUM];
        let mut r = [0.0f32; QUANTUM];
        for _ in 0..1000 {
            e.process(&mut l, &mut r);
            for s in l.iter().chain(r.iter()) {
                assert!(s.is_finite());
                assert!(s.abs() <= 1.0, "tanh ceiling breached: {s}");
            }
        }
    }

    #[test]
    fn stop_rewinds_and_silences() {
        let mut e = Engine::new(48_000.0);
        e.apply(Command::SetStep {
            index: 0,
            step: Some(Step {
                note: 60,
                vel: 100,
                gate: 0.5,
            }),
        });
        e.apply(Command::Play);
        render_seconds(&mut e, 0.5);
        e.apply(Command::Stop);
        render_seconds(&mut e, 2.0);
        let (peak, frame) = render_seconds(&mut e, 0.1);
        assert!(peak < 1e-4);
        assert_eq!(frame, 0, "stop must rewind to zero");
    }
}
