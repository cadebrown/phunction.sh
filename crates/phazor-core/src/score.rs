//! The generative score: a weather system, not a piano roll.
//!
//! Three layers over one modal terrain — a drone that moves through a chord
//! progression in geological time, an arp that patterns the current chord in
//! sixteenths, and a lead that fires semirandomly from a pentatonic subset.
//! Everything derives from pure frame arithmetic + a seeded hash, exactly
//! like [`crate::seq::StepSequencer`]: replaying the same frame range always
//! yields the same music, so stop/rewind/seek are free and the audio path
//! never holds mutable musical state.

use crate::seq::{SeqEvent, SeqEventKind, MAX_EVENTS_PER_BLOCK};
use crate::transport::Transport;
use crate::{LAYER_ARP, LAYER_DRONE, LAYER_LEAD};

/// Modal scales, darkest first. Discriminants are the wire format for
/// [`crate::event::Command::SetScale`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum Scale {
    /// Phrygian: the flat second is the shadow over everything.
    #[default]
    Phrygian = 0,
    /// Aeolian (natural minor): dark, but it resolves.
    Aeolian = 1,
    /// Dorian: dark with one candle lit (the raised sixth).
    Dorian = 2,
}

impl Scale {
    /// Wire format → scale (out-of-range falls back to Phrygian).
    #[must_use]
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Aeolian,
            2 => Self::Dorian,
            _ => Self::Phrygian,
        }
    }

    /// Semitone offsets of the seven degrees.
    #[must_use]
    pub fn semis(self) -> [i16; 7] {
        match self {
            Self::Phrygian => [0, 1, 3, 5, 7, 8, 10],
            Self::Aeolian => [0, 2, 3, 5, 7, 8, 10],
            Self::Dorian => [0, 2, 3, 5, 7, 9, 10],
        }
    }
}

/// Deterministic 32-bit mix of (seed, index) — the score's only randomness.
/// Same seed + same musical position = same choice, forever.
#[must_use]
fn hash(seed: u32, k: u64) -> u32 {
    let mut x =
        seed ^ (k as u32).wrapping_mul(0x9E37_79B9) ^ ((k >> 32) as u32).wrapping_mul(0x85EB_CA6B);
    x ^= x >> 16;
    x = x.wrapping_mul(0x7FEB_352D);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846C_A68B);
    x ^= x >> 16;
    x
}

/// `hash` mapped to [0, 1).
#[must_use]
fn hash01(seed: u32, k: u64) -> f32 {
    (hash(seed, k) >> 8) as f32 / (1u32 << 24) as f32
}

/// The progression: chord-root scale degrees, two bars each. i → VI → iv → v
/// in minor-family modes — the dark-ambient standard for a reason.
const PROGRESSION: [usize; 4] = [0, 5, 3, 4];

/// Beats per chord (two bars of 4/4).
const CHORD_BEATS: f64 = 8.0;

/// Pentatonic degree subset the lead walks (indices into the scale).
const LEAD_DEGREES: [usize; 5] = [0, 2, 3, 4, 6];

/// Arp contour over the chord tones (indices into `CHORD_ARP_TONES`).
const ARP_CONTOUR: [usize; 6] = [0, 1, 2, 3, 2, 1];

/// Chord-relative semitone offsets the arp cycles through.
const CHORD_ARP_TONES: [i16; 4] = [0, 7, 12, 15];

/// Lead gate length in steps (sixteenths).
const LEAD_GATE_STEPS: f64 = 1.6;

/// The generative score. Cheap `Copy` state — all mutation is via commands.
#[derive(Debug, Clone, Copy)]
pub struct Score {
    /// Hash seed: a whole new weather system per value.
    pub seed: u32,
    /// The modal terrain.
    pub scale: Scale,
    /// MIDI root of the world (A2 by default — low enough to loom).
    pub root: u8,
}

impl Default for Score {
    fn default() -> Self {
        Self {
            seed: 0xC0FF_EE00,
            scale: Scale::Phrygian,
            root: 45,
        }
    }
}

impl Score {
    /// Chord index sounding at a given beat position.
    #[must_use]
    fn chord_at(beats: f64) -> usize {
        ((beats / CHORD_BEATS) as usize) % PROGRESSION.len()
    }

    /// The chord's tones as MIDI notes (low root, root, fifth, minor tenth).
    #[must_use]
    fn chord_tones(self, chord: usize) -> [u8; 4] {
        let semis = self.scale.semis()[PROGRESSION[chord] % 7];
        let base = i16::from(self.root) + semis;
        [
            clamp_midi(base - 12),
            clamp_midi(base),
            clamp_midi(base + 7),
            clamp_midi(base + 15),
        ]
    }

    /// Emit this block's generative events (drone boundaries, arp steps,
    /// lead slots) at exact frame offsets, appended to `out`.
    ///
    /// The caller sorts; we only guarantee correctness of offsets.
    pub fn events_for_block(
        self,
        transport: &Transport,
        block_len: usize,
        out: &mut heapless::Vec<SeqEvent, MAX_EVENTS_PER_BLOCK>,
        lead_density: f32,
    ) {
        if !transport.playing() {
            return;
        }
        let sr = transport.sample_rate();
        let start = transport.frame() as f64;
        let end = start + block_len as f64;
        let frames_per_beat = 60.0 / transport.bpm() * sr;

        self.drone_events(start, end, frames_per_beat, out);
        self.arp_events(start, end, frames_per_beat, out);
        self.lead_events(start, end, frames_per_beat, lead_density, out);
    }

    /// Drone: at every chord boundary, release the old tones and start the
    /// new ones. The voices' long envelopes do the crossfade.
    fn drone_events(
        self,
        start: f64,
        end: f64,
        frames_per_beat: f64,
        out: &mut heapless::Vec<SeqEvent, MAX_EVENTS_PER_BLOCK>,
    ) {
        let frames_per_chord = frames_per_beat * CHORD_BEATS;
        let first = (start / frames_per_chord).ceil() as u64;
        let last = (end / frames_per_chord).ceil() as u64;
        for k in first..last {
            let boundary = k as f64 * frames_per_chord;
            let offset = (boundary - start) as usize;
            let chord = (k as usize) % PROGRESSION.len();
            if k > 0 {
                let prev = (k as usize + PROGRESSION.len() - 1) % PROGRESSION.len();
                for note in self.chord_tones(prev) {
                    let _ = out.push(SeqEvent {
                        offset,
                        kind: SeqEventKind::NoteOff {
                            note,
                            layer: LAYER_DRONE,
                        },
                    });
                }
            }
            for (i, note) in self.chord_tones(chord).into_iter().enumerate() {
                let vel = if i == 0 { 96 } else { 72 };
                let _ = out.push(SeqEvent {
                    offset,
                    kind: SeqEventKind::NoteOn {
                        note,
                        vel,
                        layer: LAYER_DRONE,
                    },
                });
            }
        }
    }

    /// Arp: sixteenths cycling a contour over the current chord's tones,
    /// with hash-chosen rests for breath and a register lift every two bars.
    fn arp_events(
        self,
        start: f64,
        end: f64,
        frames_per_beat: f64,
        out: &mut heapless::Vec<SeqEvent, MAX_EVENTS_PER_BLOCK>,
    ) {
        let frames_per_step = frames_per_beat / 4.0;
        let first = (start / frames_per_step).ceil() as u64;
        let last = (end / frames_per_step).ceil() as u64;
        let arp_seed = self.seed ^ 0x00A4_9000;
        // ons
        for k in first..last {
            if hash(arp_seed, k) % 8 == 7 {
                continue; // rest — the pattern breathes
            }
            let boundary = k as f64 * frames_per_step;
            let beats = boundary / frames_per_beat;
            let chord = Self::chord_at(beats);
            let root = i16::from(self.chord_tones(chord)[1]);
            let tone = CHORD_ARP_TONES[ARP_CONTOUR[(k as usize) % ARP_CONTOUR.len()]];
            let lift = if (k / 32) % 2 == 1 { 12 } else { 0 };
            let note = clamp_midi(root + tone + lift);
            let vel = if k % 4 == 0 {
                96
            } else {
                60 + (hash(arp_seed, k) % 24) as u8
            };
            let _ = out.push(SeqEvent {
                offset: (boundary - start) as usize,
                kind: SeqEventKind::NoteOn {
                    note,
                    vel,
                    layer: LAYER_ARP,
                },
            });
        }
        // offs: gate = 0.55 steps; only the previous couple of steps can
        // still be ringing into this block
        for back in 1..=2u64 {
            let Some(k) = first.checked_sub(back) else {
                continue;
            };
            if hash(arp_seed, k) % 8 == 7 {
                continue;
            }
            let boundary = k as f64 * frames_per_step;
            let off_at = boundary + 0.55 * frames_per_step;
            if off_at >= start && off_at < end {
                let beats = boundary / frames_per_beat;
                let chord = Self::chord_at(beats);
                let root = i16::from(self.chord_tones(chord)[1]);
                let tone = CHORD_ARP_TONES[ARP_CONTOUR[(k as usize) % ARP_CONTOUR.len()]];
                let lift = if (k / 32) % 2 == 1 { 12 } else { 0 };
                let _ = out.push(SeqEvent {
                    offset: (off_at - start) as usize,
                    kind: SeqEventKind::NoteOff {
                        note: clamp_midi(root + tone + lift),
                        layer: LAYER_ARP,
                    },
                });
            }
        }
    }

    /// Lead: eighth-note slots that fire with `density` probability, notes
    /// hash-picked from a pentatonic subset an octave (sometimes two) up.
    /// Semirandom: random enough to surprise, seeded enough to replay.
    fn lead_events(
        self,
        start: f64,
        end: f64,
        frames_per_beat: f64,
        density: f32,
        out: &mut heapless::Vec<SeqEvent, MAX_EVENTS_PER_BLOCK>,
    ) {
        let frames_per_slot = frames_per_beat / 2.0;
        let lead_seed = self.seed ^ 0x001E_AD00;
        let semis = self.scale.semis();
        let note_of = |k: u64| -> u8 {
            let h = hash(lead_seed, k);
            let deg = LEAD_DEGREES[((h >> 8) % 5) as usize];
            let oct = if h & 1 == 1 { 24 } else { 12 };
            clamp_midi(i16::from(self.root) + semis[deg] + oct)
        };
        let fires = |k: u64| hash01(lead_seed, k) < density;

        let first = (start / frames_per_slot).ceil() as u64;
        let last = (end / frames_per_slot).ceil() as u64;
        for k in first..last {
            if !fires(k) {
                continue;
            }
            let boundary = k as f64 * frames_per_slot;
            let vel = 58 + (hash(lead_seed ^ 0x0000_00E1, k) % 40) as u8;
            let _ = out.push(SeqEvent {
                offset: (boundary - start) as usize,
                kind: SeqEventKind::NoteOn {
                    note: note_of(k),
                    vel,
                    layer: LAYER_LEAD,
                },
            });
        }
        // offs: gate is 1.6 slots, so scan a window of 2 slots back
        for back in 1..=2u64 {
            let Some(k) = first.checked_sub(back) else {
                continue;
            };
            if !fires(k) {
                continue;
            }
            let off_at = (k as f64 + LEAD_GATE_STEPS) * frames_per_slot;
            if off_at >= start && off_at < end {
                let _ = out.push(SeqEvent {
                    offset: (off_at - start) as usize,
                    kind: SeqEventKind::NoteOff {
                        note: note_of(k),
                        layer: LAYER_LEAD,
                    },
                });
            }
        }
    }
}

/// Clamp an i16 semitone computation into playable MIDI.
#[must_use]
fn clamp_midi(n: i16) -> u8 {
    n.clamp(12, 108) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect_events(score: Score, seconds: f64, density: f32) -> Vec<(u64, SeqEventKind)> {
        let mut t = Transport::new(48_000.0, 80.0);
        t.play();
        let mut all = Vec::new();
        let mut ev: heapless::Vec<SeqEvent, MAX_EVENTS_PER_BLOCK> = heapless::Vec::new();
        let blocks = (seconds * 48_000.0 / 128.0) as usize;
        for _ in 0..blocks {
            ev.clear();
            score.events_for_block(&t, 128, &mut ev, density);
            for e in &ev {
                all.push((t.frame() + e.offset as u64, e.kind));
            }
            t.advance(128);
        }
        all
    }

    #[test]
    fn drone_fires_chords_and_releases_them() {
        let s = Score::default();
        let events = collect_events(s, 20.0, 0.0);
        let ons = events
            .iter()
            .filter(
                |(_, k)| matches!(k, SeqEventKind::NoteOn { layer, .. } if *layer == LAYER_DRONE),
            )
            .count();
        let offs = events
            .iter()
            .filter(
                |(_, k)| matches!(k, SeqEventKind::NoteOff { layer, .. } if *layer == LAYER_DRONE),
            )
            .count();
        // 20s at 80 BPM ≈ 26.7 beats ≈ 3.3 chords → chord 0, 1, 2, 3 started
        assert!(ons >= 12, "chords must sound, got {ons} ons");
        assert!(offs >= 8, "old chords must release, got {offs} offs");
    }

    #[test]
    fn score_is_deterministic() {
        let s = Score::default();
        let a = collect_events(s, 10.0, 0.6);
        let b = collect_events(s, 10.0, 0.6);
        assert_eq!(a, b, "same seed + same frames = same music");
    }

    #[test]
    fn seed_changes_the_weather() {
        let a = collect_events(Score::default(), 10.0, 0.6);
        let b = collect_events(
            Score {
                seed: 12345,
                ..Score::default()
            },
            10.0,
            0.6,
        );
        assert_ne!(a, b, "a new seed must be a new weather system");
    }

    #[test]
    fn lead_density_zero_is_silent_density_one_is_every_slot() {
        let none = collect_events(Score::default(), 10.0, 0.0);
        let lead_none = none
            .iter()
            .filter(
                |(_, k)| matches!(k, SeqEventKind::NoteOn { layer, .. } if *layer == LAYER_LEAD),
            )
            .count();
        assert_eq!(lead_none, 0);
        let all = collect_events(Score::default(), 10.0, 1.0);
        let lead_all = all
            .iter()
            .filter(
                |(_, k)| matches!(k, SeqEventKind::NoteOn { layer, .. } if *layer == LAYER_LEAD),
            )
            .count();
        // 10s at 80 BPM = 13.3 beats ≈ 26 eighth slots
        assert!(lead_all >= 24, "density 1 fires every slot, got {lead_all}");
    }

    #[test]
    fn every_on_gets_an_off() {
        use std::collections::HashMap;
        let events = collect_events(Score::default(), 30.0, 0.7);
        let mut open: HashMap<(u8, u8), i32> = HashMap::new();
        for (_, k) in &events {
            match k {
                SeqEventKind::NoteOn { note, layer, .. } => {
                    *open.entry((*note, *layer)).or_default() += 1;
                }
                SeqEventKind::NoteOff { note, layer } => {
                    *open.entry((*note, *layer)).or_default() -= 1;
                }
            }
        }
        // Every (note, layer) should be nearly balanced — at most the final
        // chord + tail notes still open at the cut.
        let dangling: i32 = open.values().filter(|v| **v > 0).sum();
        assert!(dangling <= 8, "unbalanced note-ons: {open:?}");
    }

    #[test]
    fn notes_stay_in_midi_range() {
        let events = collect_events(Score::default(), 30.0, 1.0);
        for (_, k) in events {
            if let SeqEventKind::NoteOn { note, .. } = k {
                assert!((12..=108).contains(&note));
            }
        }
    }
}
