//! The FX graph: ping-pong delay → Freeverb-style reverb → drive.
//!
//! All buffers are allocated once in the constructors (the realtime contract
//! covers everything *after* `new()`); processing is per-sample, branch-light,
//! and never allocates. The graph is a send architecture, not inserts: dry
//! layer buses feed the delay, the delay feeds the reverb (echoes bloom into
//! wash — the dark-ambient trick), and the master sums dry + returns.

use crate::math::flush_denormal;
use crate::Sample;

/// One-pole lowpass/highpass pair used inside feedback loops to darken
/// repeats (dark ambient: echoes decay into shadow, not sizzle).
#[derive(Debug, Clone, Copy, Default)]
struct LoopTone {
    lp: f32,
    hp: f32,
}

impl LoopTone {
    /// Darken: lowpass at ~5 kHz, highpass at ~120 Hz (coefficients baked
    /// for 48 kHz-ish rates; the loop tone is a color, not a spec).
    #[inline]
    fn tick(&mut self, x: f32) -> f32 {
        self.lp += 0.45 * (x - self.lp);
        self.hp += 0.015 * (self.lp - self.hp);
        self.lp - self.hp
    }

    fn flush(&mut self) {
        self.lp = flush_denormal(self.lp);
        self.hp = flush_denormal(self.hp);
    }
}

/// Tempo-synced ping-pong delay: two cross-fed lines, loop-darkened.
///
/// The delay time GLIDES (tape-style): a tempo change slews the read head
/// with linear interpolation instead of jumping it — retuning the delay
/// mid-performance pitch-bends the tail instead of clicking. This is the
/// glitch-free invariant applied to a delay line.
pub struct PingPong {
    bufs: [Box<[f32]>; 2],
    write: usize,
    /// Current (gliding, fractional) delay in frames.
    delay_f: f32,
    /// Where the delay is headed.
    target: f32,
    tones: [LoopTone; 2],
}

impl PingPong {
    /// Maximum delay time in seconds (sizing the lines).
    pub const MAX_SECONDS: f32 = 1.5;

    /// Lines sized for `sample_rate`; delay starts at a dotted eighth of
    /// 120 BPM until [`Self::set_delay_frames`] is called.
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        let len = (sample_rate * Self::MAX_SECONDS) as usize + 1;
        Self {
            bufs: [
                vec![0.0; len].into_boxed_slice(),
                vec![0.0; len].into_boxed_slice(),
            ],
            write: 0,
            delay_f: sample_rate * 0.375,
            target: sample_rate * 0.375,
            tones: [LoopTone::default(); 2],
        }
    }

    /// Set the delay time in frames (the engine derives it from tempo:
    /// a dotted eighth). The head GLIDES there; it never jumps.
    #[allow(clippy::cast_precision_loss)]
    pub fn set_delay_frames(&mut self, frames: usize) {
        self.target = frames.clamp(1, self.bufs[0].len() - 2) as f32;
    }

    /// Jump the head immediately (initialization/tests only — gliding is
    /// the performance behavior).
    pub fn snap_delay(&mut self, frames: usize) {
        self.set_delay_frames(frames);
        self.delay_f = self.target;
    }

    /// Advance one frame: feed the send, return the wet pair. `feedback` is
    /// clamped hard below 1 — a live instrument never runs away.
    #[inline]
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    pub fn tick(&mut self, send_l: f32, send_r: f32, feedback: f32) -> (f32, f32) {
        // exponential glide (τ ≈ 42ms at 48k): a tempo change bends the
        // tail like tape instead of jumping the read head
        self.delay_f += (self.target - self.delay_f) * 5e-4;
        let len = self.bufs[0].len();
        let d0 = self.delay_f as usize;
        let frac = self.delay_f - d0 as f32;
        let r0 = (self.write + len - d0) % len;
        let r1 = (self.write + len - (d0 + 1)) % len;
        let wet_l = self.bufs[0][r0] * (1.0 - frac) + self.bufs[0][r1] * frac;
        let wet_r = self.bufs[1][r0] * (1.0 - frac) + self.bufs[1][r1] * frac;
        let fb = feedback.clamp(0.0, 0.9);
        // the cross: left's tail returns on the right and vice versa
        self.bufs[0][self.write] = self.tones[0].tick(send_l + wet_r * fb);
        self.bufs[1][self.write] = self.tones[1].tick(send_r + wet_l * fb);
        self.write = (self.write + 1) % len;
        (wet_l, wet_r)
    }

    /// Block-boundary denormal hygiene.
    pub fn flush(&mut self) {
        for t in &mut self.tones {
            t.flush();
        }
    }
}

/// Freeverb comb: delay line with damped feedback.
struct Comb {
    buf: Box<[f32]>,
    ix: usize,
    store: f32,
}

impl Comb {
    fn new(len: usize) -> Self {
        Self {
            buf: vec![0.0; len.max(2)].into_boxed_slice(),
            ix: 0,
            store: 0.0,
        }
    }

    #[inline]
    fn tick(&mut self, x: f32, feedback: f32, damp: f32) -> f32 {
        let out = self.buf[self.ix];
        self.store = out * (1.0 - damp) + self.store * damp;
        self.buf[self.ix] = x + self.store * feedback;
        self.ix = (self.ix + 1) % self.buf.len();
        out
    }

    fn flush(&mut self) {
        self.store = flush_denormal(self.store);
    }
}

/// Freeverb allpass: smears comb resonances into diffusion.
struct Allpass {
    buf: Box<[f32]>,
    ix: usize,
}

impl Allpass {
    fn new(len: usize) -> Self {
        Self {
            buf: vec![0.0; len.max(2)].into_boxed_slice(),
            ix: 0,
        }
    }

    #[inline]
    fn tick(&mut self, x: f32) -> f32 {
        let bufout = self.buf[self.ix];
        let out = bufout - x;
        self.buf[self.ix] = x + bufout * 0.5;
        self.ix = (self.ix + 1) % self.buf.len();
        out
    }
}

/// Jezar's Freeverb tunings (samples at 44.1 kHz), the public-domain
/// standard: 8 parallel combs into 4 series allpasses per channel, the right
/// channel offset by a fixed spread for width.
const COMB_TUNING: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
const ALLPASS_TUNING: [usize; 4] = [556, 441, 341, 225];
const STEREO_SPREAD: usize = 23;

/// Stereo Freeverb. `size` maps to comb feedback, fixed damping keeps the
/// tail dark.
pub struct Reverb {
    combs_l: [Comb; 8],
    combs_r: [Comb; 8],
    aps_l: [Allpass; 4],
    aps_r: [Allpass; 4],
}

impl Reverb {
    /// Tunings scaled from 44.1 kHz to `sample_rate`.
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        let scale = sample_rate / 44_100.0;
        let sz = |n: usize| ((n as f32) * scale) as usize;
        Self {
            combs_l: core::array::from_fn(|i| Comb::new(sz(COMB_TUNING[i]))),
            combs_r: core::array::from_fn(|i| Comb::new(sz(COMB_TUNING[i] + STEREO_SPREAD))),
            aps_l: core::array::from_fn(|i| Allpass::new(sz(ALLPASS_TUNING[i]))),
            aps_r: core::array::from_fn(|i| Allpass::new(sz(ALLPASS_TUNING[i] + STEREO_SPREAD))),
        }
    }

    /// Advance one frame. `size` in 0..=1 (comb feedback 0.7..=0.98).
    #[inline]
    pub fn tick(&mut self, send_l: f32, send_r: f32, size: f32) -> (f32, f32) {
        let feedback = 0.7 + 0.28 * size.clamp(0.0, 1.0);
        let damp = 0.4;
        let input = f32::midpoint(send_l, send_r) * 0.015; // freeverb's fixed input gain
        let mut l = 0.0;
        let mut r = 0.0;
        for c in &mut self.combs_l {
            l += c.tick(input, feedback, damp);
        }
        for c in &mut self.combs_r {
            r += c.tick(input, feedback, damp);
        }
        for a in &mut self.aps_l {
            l = a.tick(l);
        }
        for a in &mut self.aps_r {
            r = a.tick(r);
        }
        (l, r)
    }

    /// Block-boundary denormal hygiene.
    pub fn flush(&mut self) {
        for c in &mut self.combs_l {
            c.flush();
        }
        for c in &mut self.combs_r {
            c.flush();
        }
    }
}

/// Soft saturation: gain into tanh, output compensated so `drive` changes
/// color more than loudness.
#[inline]
#[must_use]
pub fn drive(x: Sample, amount: f32) -> Sample {
    let g = 1.0 + amount * 3.0;
    (x * g).tanh() / (1.0 + amount * 1.2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pingpong_echo_arrives_after_delay_and_crosses() {
        let mut d = PingPong::new(48_000.0);
        d.snap_delay(100);
        // impulse on the left only
        let _ = d.tick(1.0, 0.0, 0.5);
        let mut first_echo = None;
        for i in 1..300 {
            let (l, r) = d.tick(0.0, 0.0, 0.5);
            if (l.abs() > 1e-4 || r.abs() > 1e-4) && first_echo.is_none() {
                first_echo = Some((i, l, r));
            }
        }
        let (at, l, r) = first_echo.expect("echo must arrive");
        assert_eq!(at, 100, "echo lands exactly at the delay time");
        assert!(l.abs() > 1e-4, "first tap returns on the fed channel");
        assert!(r.abs() < 1e-6, "cross-echo waits one round");
    }

    #[test]
    fn pingpong_feedback_decays() {
        let mut d = PingPong::new(48_000.0);
        d.snap_delay(50);
        let _ = d.tick(1.0, 1.0, 0.6);
        let mut peak_late = 0.0f32;
        for i in 0..48_000 {
            let (l, r) = d.tick(0.0, 0.0, 0.6);
            if i > 24_000 {
                peak_late = peak_late.max(l.abs()).max(r.abs());
            }
        }
        assert!(peak_late < 0.05, "echoes must die out, got {peak_late}");
    }

    #[test]
    fn retuning_the_delay_never_jumps_the_output() {
        let mut d = PingPong::new(48_000.0);
        d.snap_delay(2_000);
        // charge the line with a slow sine so any head jump would be audible
        let mut prev = 0.0f32;
        for i in 0..20_000 {
            let x = (i as f32 * 0.01).sin() * 0.5;
            if i == 6_000 {
                d.set_delay_frames(9_000); // a hard retune mid-flight
            }
            let (l, _) = d.tick(x, x, 0.4);
            if i > 2_100 {
                assert!(
                    (l - prev).abs() < 0.1,
                    "delay retune clicked at {i}: Δ={}",
                    (l - prev).abs()
                );
            }
            prev = l;
        }
    }

    #[test]
    fn reverb_rings_then_decays_and_stays_finite() {
        let mut v = Reverb::new(48_000.0);
        let _ = v.tick(1.0, 1.0, 0.8);
        let mut early = 0.0f32;
        let mut late = 0.0f32;
        for i in 0..(48_000 * 6) {
            let (l, r) = v.tick(0.0, 0.0, 0.8);
            assert!(l.is_finite() && r.is_finite());
            if i < 48_000 {
                early = early.max(l.abs());
            }
            if i > 48_000 * 5 {
                late = late.max(l.abs());
            }
        }
        assert!(early > 1e-6, "an impulse must ring");
        assert!(late < early, "the tail must decay");
    }

    #[test]
    fn drive_is_bounded_and_monotone_in_color() {
        for x in [-2.0f32, -0.5, 0.0, 0.5, 2.0] {
            for a in [0.0f32, 0.5, 1.0] {
                let y = drive(x, a);
                assert!(y.is_finite());
                assert!(y.abs() <= 1.0);
            }
        }
    }
}
