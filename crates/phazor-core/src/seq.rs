//! The step sequencer: a 16-step loop locked to the transport.
//!
//! The sequencer never keeps its own clock. Each block, the engine hands it
//! the frame range being rendered; the sequencer converts step boundaries to
//! frame offsets inside that range and emits events at exact offsets. This
//! is what "cycle-locked, sample-accurate" means concretely.

use crate::transport::Transport;

/// One sequencer step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Step {
    /// MIDI note number.
    pub note: u8,
    /// Velocity 1..=127.
    pub vel: u8,
    /// Gate length as a fraction of the step duration, (0, 1].
    pub gate: f32,
}

/// A timed note event within one render block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeqEvent {
    /// Frame offset from the start of the block, `0 <= offset < block_len`.
    pub offset: usize,
    /// What happens at that frame.
    pub kind: SeqEventKind,
}

/// Kind of sequencer event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeqEventKind {
    /// Start a note.
    NoteOn {
        /// MIDI note number.
        note: u8,
        /// Velocity.
        vel: u8,
        /// Which engine layer sounds it (see `crate::LAYER_*`).
        layer: u8,
    },
    /// End a note.
    NoteOff {
        /// MIDI note number.
        note: u8,
        /// Which engine layer releases it.
        layer: u8,
    },
}

/// Sixteen sixteenth-note steps = one bar of 4/4.
#[derive(Debug, Clone, Copy)]
pub struct StepSequencer {
    steps: [Option<Step>; Self::LEN],
    /// Steps per beat: 4 → sixteenth notes.
    steps_per_beat: f64,
}

/// Maximum events a single block can produce across ALL generators (the
/// user pattern plus the score's drone/arp/lead layers). Chord boundaries
/// are the worst case: four offs + four ons in one block, on top of a full
/// pattern edge — 64 leaves comfortable headroom.
pub const MAX_EVENTS_PER_BLOCK: usize = 64;

impl Default for StepSequencer {
    fn default() -> Self {
        Self {
            steps: [None; Self::LEN],
            steps_per_beat: 4.0,
        }
    }
}

impl StepSequencer {
    /// Number of steps in the loop.
    pub const LEN: usize = 16;

    /// Write (or clear) a step.
    pub fn set_step(&mut self, index: usize, step: Option<Step>) {
        if let Some(slot) = self.steps.get_mut(index) {
            *slot = step;
        }
    }

    /// Read a step.
    #[must_use]
    pub fn step(&self, index: usize) -> Option<Step> {
        self.steps.get(index).copied().flatten()
    }

    /// Length of the loop in beats.
    #[must_use]
    pub fn loop_beats(&self) -> f64 {
        Self::LEN as f64 / self.steps_per_beat
    }

    /// Which step index a beat position falls in (for UI playhead display).
    #[must_use]
    pub fn step_at_beats(&self, beats: f64) -> usize {
        let step_pos = beats * self.steps_per_beat;
        (step_pos.rem_euclid(Self::LEN as f64)) as usize % Self::LEN
    }

    /// Collect the events for a block spanning `block_len` frames starting at
    /// the transport's current frame. Events are pushed onto `out` sorted by
    /// offset. `out` is caller-owned so the audio path never allocates.
    ///
    /// Both note-ons (step boundaries inside the block) and the matching
    /// note-offs (gate ends, possibly from steps begun in earlier blocks) are
    /// derived purely from frame arithmetic — replaying the same frame range
    /// always yields the same events.
    pub fn events_for_block(
        &self,
        transport: &Transport,
        block_len: usize,
        out: &mut heapless::Vec<SeqEvent, MAX_EVENTS_PER_BLOCK>,
    ) {
        if !transport.playing() {
            return;
        }
        let sr = transport.sample_rate();
        let frames_per_step = 60.0 / transport.bpm() / self.steps_per_beat * sr;
        let start = transport.frame() as f64;
        let end = start + block_len as f64;

        // Any step boundary k·frames_per_step in [start, end) fires here.
        let first_k = (start / frames_per_step).ceil() as u64;
        let last_k = (end / frames_per_step).ceil() as u64;
        for k in first_k..last_k {
            let boundary = k as f64 * frames_per_step;
            let index = (k as usize) % Self::LEN;
            if let Some(step) = self.steps[index] {
                let on_offset = (boundary - start) as usize;
                let _ = out.push(SeqEvent {
                    offset: on_offset.min(block_len - 1),
                    kind: SeqEventKind::NoteOn {
                        note: step.note,
                        vel: step.vel,
                        layer: crate::LAYER_ARP,
                    },
                });
            }
        }

        // Gate ends: for each step whose gate interval ends inside this
        // block, emit the off. Scan the steps whose note-on could still be
        // ringing (gate ≤ 1 step, so only the current and previous step).
        for k_back in 0..2u64 {
            let Some(k) = first_k.checked_sub(k_back + 1) else {
                continue;
            };
            let index = (k as usize) % Self::LEN;
            if let Some(step) = self.steps[index] {
                let off_at = k as f64 * frames_per_step
                    + f64::from(step.gate.clamp(0.05, 1.0)) * frames_per_step;
                if off_at >= start && off_at < end {
                    let _ = out.push(SeqEvent {
                        offset: (off_at - start) as usize,
                        kind: SeqEventKind::NoteOff {
                            note: step.note,
                            layer: crate::LAYER_ARP,
                        },
                    });
                }
            }
        }

        // Keep events offset-sorted so the engine can split buffers linearly.
        out.sort_unstable_by_key(|e| e.offset);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seq_with_step0() -> StepSequencer {
        let mut s = StepSequencer::default();
        s.set_step(
            0,
            Some(Step {
                note: 60,
                vel: 100,
                gate: 0.5,
            }),
        );
        s
    }

    #[test]
    fn step_zero_fires_at_frame_zero() {
        let seq = seq_with_step0();
        let mut t = Transport::new(48_000.0, 120.0);
        t.play();
        let mut ev = heapless::Vec::new();
        seq.events_for_block(&t, 128, &mut ev);
        assert_eq!(ev.len(), 1);
        assert_eq!(ev[0].offset, 0);
        assert!(matches!(
            ev[0].kind,
            SeqEventKind::NoteOn {
                note: 60,
                vel: 100,
                layer: crate::LAYER_ARP,
            }
        ));
    }

    #[test]
    fn events_are_sample_accurate_across_blocks() {
        // At 120 BPM / 48kHz, a 16th note = 6000 frames. Step 1's boundary at
        // frame 6000 must land at offset 6000 - 5888 = 112 in its block.
        let mut seq = seq_with_step0();
        seq.set_step(
            1,
            Some(Step {
                note: 62,
                vel: 90,
                gate: 0.5,
            }),
        );
        let mut t = Transport::new(48_000.0, 120.0);
        t.play();
        let mut found = None;
        let mut ev: heapless::Vec<SeqEvent, MAX_EVENTS_PER_BLOCK> = heapless::Vec::new();
        for _block in 0..100 {
            ev.clear();
            seq.events_for_block(&t, 128, &mut ev);
            for e in &ev {
                if let SeqEventKind::NoteOn { note: 62, .. } = e.kind {
                    found = Some(t.frame() as usize + e.offset);
                }
            }
            t.advance(128);
        }
        assert_eq!(found, Some(6000), "step 1 must fire at exactly frame 6000");
    }

    #[test]
    fn gate_off_follows_note_on() {
        let seq = seq_with_step0();
        let mut t = Transport::new(48_000.0, 120.0);
        t.play();
        // gate 0.5 × 6000 frames = off at frame 3000 → block starting 2944.
        let mut off_frame = None;
        let mut ev: heapless::Vec<SeqEvent, MAX_EVENTS_PER_BLOCK> = heapless::Vec::new();
        for _ in 0..50 {
            ev.clear();
            seq.events_for_block(&t, 128, &mut ev);
            for e in &ev {
                if let SeqEventKind::NoteOff { note: 60, .. } = e.kind {
                    off_frame = Some(t.frame() as usize + e.offset);
                }
            }
            t.advance(128);
        }
        assert_eq!(off_frame, Some(3000));
    }

    #[test]
    fn stopped_transport_emits_nothing() {
        let seq = seq_with_step0();
        let t = Transport::new(48_000.0, 120.0);
        let mut ev = heapless::Vec::new();
        seq.events_for_block(&t, 128, &mut ev);
        assert!(ev.is_empty());
    }
}
