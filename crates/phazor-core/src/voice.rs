//! A phazor voice: quadrature oscillator → ADSR → state-variable filter.

use crate::math::{flush_denormal, midi_to_hz, Phasor};
use crate::Sample;

/// ADSR envelope stages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

/// Exponential-approach ADSR. Each stage is a one-pole glide toward a target,
/// which is inherently click-free and costs one multiply-add per sample.
#[derive(Debug, Clone, Copy)]
pub struct Adsr {
    stage: Stage,
    level: f32,
    attack_coeff: f32,
    decay_coeff: f32,
    sustain: f32,
    release_coeff: f32,
    /// Steal-release coefficient (~2ms), fixed at construction.
    fast_coeff: f32,
    /// Fast (steal) release engaged.
    fast: bool,
    sample_rate: f32,
}

/// Convert a time-to-63% in milliseconds into a one-pole coefficient.
fn ms_to_coeff(ms: f32, sample_rate: f32) -> f32 {
    1.0 - (-1.0 / (ms.max(0.1) * 1e-3 * sample_rate)).exp()
}

impl Adsr {
    /// A silent, idle envelope.
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        Self {
            stage: Stage::Idle,
            level: 0.0,
            attack_coeff: ms_to_coeff(4.0, sample_rate),
            decay_coeff: ms_to_coeff(180.0, sample_rate),
            sustain: 0.6,
            release_coeff: ms_to_coeff(220.0, sample_rate),
            fast_coeff: ms_to_coeff(2.0, sample_rate),
            fast: false,
            sample_rate,
        }
    }

    /// Update timing parameters (from the smoothed engine params).
    pub fn configure(&mut self, attack_ms: f32, decay_ms: f32, sustain: f32, release_ms: f32) {
        self.attack_coeff = ms_to_coeff(attack_ms, self.sample_rate);
        self.decay_coeff = ms_to_coeff(decay_ms, self.sample_rate);
        self.sustain = sustain.clamp(0.0, 1.0);
        self.release_coeff = ms_to_coeff(release_ms, self.sample_rate);
    }

    /// Enter the attack stage (retriggers from the current level — legato).
    pub fn gate_on(&mut self) {
        self.stage = Stage::Attack;
        self.fast = false;
    }

    /// Enter the release stage.
    pub fn gate_off(&mut self) {
        if self.stage != Stage::Idle {
            self.stage = Stage::Release;
            self.fast = false;
        }
    }

    /// Enter a ~2ms forced release (voice steal): fast enough to free the
    /// voice within a block, slow enough to never click.
    pub fn fast_release(&mut self) {
        if self.stage != Stage::Idle {
            self.stage = Stage::Release;
            self.fast = true;
        }
    }

    /// Hard-stop to silence (voice steal, panic).
    pub fn kill(&mut self) {
        self.stage = Stage::Idle;
        self.level = 0.0;
    }

    /// True once the envelope has fully died out.
    #[must_use]
    pub fn idle(&self) -> bool {
        self.stage == Stage::Idle
    }

    /// Current envelope level without advancing.
    #[must_use]
    pub fn level(&self) -> f32 {
        self.level
    }

    /// Advance one sample.
    #[inline]
    pub fn tick(&mut self) -> f32 {
        match self.stage {
            Stage::Idle => {}
            Stage::Attack => {
                // Overshoot target so the exponential actually reaches 1.0.
                self.level += self.attack_coeff * (1.6 - self.level);
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.stage = Stage::Decay;
                }
            }
            Stage::Decay => {
                self.level += self.decay_coeff * (self.sustain - self.level);
                if (self.level - self.sustain).abs() < 1e-4 {
                    self.stage = Stage::Sustain;
                }
            }
            Stage::Sustain => self.level = self.sustain,
            Stage::Release => {
                let coeff = if self.fast {
                    self.fast_coeff
                } else {
                    self.release_coeff
                };
                self.level += coeff * (0.0 - self.level);
                if self.level < 1e-5 {
                    self.level = 0.0;
                    self.stage = Stage::Idle;
                    self.fast = false;
                }
            }
        }
        self.level
    }
}

/// Andrew Simper's trapezoidal-integration state-variable filter (SVF), the
/// modern standard: stable under audio-rate modulation, cheap, and gives
/// low/band/high simultaneously. We use the lowpass tap.
#[derive(Debug, Clone, Copy, Default)]
pub struct Svf {
    g: f32,
    k: f32,
    a1: f32,
    a2: f32,
    a3: f32,
    ic1: f32,
    ic2: f32,
}

impl Svf {
    /// Configure cutoff/Q. Call at block rate; per-sample retuning is safe
    /// but wasted work at this altitude.
    pub fn set(&mut self, cutoff_hz: f32, q: f32, sample_rate: f32) {
        // Clamp below Nyquist; tan() blows up at π/2.
        let fc = cutoff_hz.clamp(10.0, 0.49 * sample_rate);
        self.g = (core::f32::consts::PI * fc / sample_rate).tan();
        self.k = 1.0 / q.max(0.25);
        self.a1 = 1.0 / (1.0 + self.g * (self.g + self.k));
        self.a2 = self.g * self.a1;
        self.a3 = self.g * self.a2;
    }

    /// Advance one sample; returns the lowpass output.
    #[inline]
    pub fn tick(&mut self, v0: Sample) -> Sample {
        let v3 = v0 - self.ic2;
        let v1 = self.a1 * self.ic1 + self.a2 * v3;
        let v2 = self.ic2 + self.a2 * self.ic1 + self.a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;
        v2
    }

    /// Flush denormal state at block boundaries.
    pub fn flush(&mut self) {
        self.ic1 = flush_denormal(self.ic1);
        self.ic2 = flush_denormal(self.ic2);
    }
}

/// One polyphonic voice.
///
/// Timbre: the fundamental phasor plus two phase-locked overtone phasors
/// (2× and 3×), blended by the `brightness` parameter, plus an optional
/// detuned unison phasor (`spread`) that beats slowly against the
/// fundamental — the drone layer's width and motion come from here.
#[derive(Debug, Clone, Copy)]
pub struct Voice {
    fundamental: Phasor,
    overtone2: Phasor,
    overtone3: Phasor,
    unison: Phasor,
    env: Adsr,
    filter: Svf,
    /// MIDI note this voice is sounding (`u8::MAX` = none).
    note: u8,
    /// Slewed velocity (retriggering a ringing note glides, never steps).
    velocity: f32,
    velocity_target: f32,
    /// Monotone stamp of the last gate-on, for oldest-voice stealing.
    age: u64,
    /// A steal in flight: the note that takes over once the fast release
    /// reaches silence. `(note, vel, age, layer, pan, spread)`.
    pending: Option<(u8, u8, u64, u8, f32, f32)>,
    /// Which engine layer owns this voice (see `crate::LAYER_*`).
    layer: u8,
    /// Unison detune mix, 0 (off) ..= 1.
    spread: f32,
    /// Equal-power pan gains, set at note-on.
    pan_l: f32,
    pan_r: f32,
    sample_rate: f32,
}

impl Voice {
    /// A silent voice.
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        Self {
            fundamental: Phasor::new(440.0, f64::from(sample_rate)),
            overtone2: Phasor::new(880.0, f64::from(sample_rate)),
            overtone3: Phasor::new(1320.0, f64::from(sample_rate)),
            unison: Phasor::new(441.5, f64::from(sample_rate)),
            env: Adsr::new(sample_rate),
            filter: Svf::default(),
            note: u8::MAX,
            velocity: 0.0,
            velocity_target: 0.0,
            age: 0,
            pending: None,
            layer: crate::LAYER_ARP,
            spread: 0.0,
            pan_l: core::f32::consts::FRAC_1_SQRT_2,
            pan_r: core::f32::consts::FRAC_1_SQRT_2,
            sample_rate,
        }
    }

    /// Take this voice for `note` — gracefully. A silent or same-note
    /// voice starts immediately; a voice ringing a different note gets a
    /// ~2ms fade first, then the new note takes over (steals never click:
    /// the live-performance invariant).
    pub fn steal_to(&mut self, note: u8, vel: u8, age: u64, layer: u8, pan: f32, spread: f32) {
        if self.sounding().is_none() || (self.note == note && self.layer == layer) {
            self.note_on(note, vel, age, layer, pan, spread);
        } else {
            self.pending = Some((note, vel, age, layer, pan, spread));
            self.age = age; // claim steal order immediately
            self.env.fast_release();
        }
    }

    /// Begin sounding `note` at `vel`, stamped with `age` for steal ordering.
    /// `pan` is -1 (left) ..= 1 (right); `spread` mixes in a +9-cent unison.
    pub fn note_on(&mut self, note: u8, vel: u8, age: u64, layer: u8, pan: f32, spread: f32) {
        let hz = f64::from(midi_to_hz(f32::from(note)));
        let sr = f64::from(self.sample_rate);
        self.fundamental.set_freq(hz, sr);
        self.overtone2.set_freq(hz * 2.0, sr);
        self.overtone3.set_freq(hz * 3.0, sr);
        self.unison.set_freq(hz * 1.0052, sr);
        self.note = note;
        self.velocity_target = f32::from(vel) / 127.0;
        if self.env.idle() {
            self.velocity = self.velocity_target; // fresh voice: no history to honor
        }
        self.age = age;
        self.layer = layer;
        self.spread = spread.clamp(0.0, 1.0);
        let theta = f32::midpoint(pan.clamp(-1.0, 1.0), 1.0) * core::f32::consts::FRAC_PI_2;
        self.pan_l = theta.cos();
        self.pan_r = theta.sin();
        self.env.gate_on();
    }

    /// The layer that owns this voice.
    #[must_use]
    pub fn layer(&self) -> u8 {
        self.layer
    }

    /// Equal-power pan gains (left, right).
    #[must_use]
    pub fn pan(&self) -> (f32, f32) {
        (self.pan_l, self.pan_r)
    }

    /// Release the voice (envelope enters release; voice frees itself when
    /// the tail dies).
    pub fn note_off(&mut self) {
        self.env.gate_off();
    }

    /// Immediate silence.
    pub fn kill(&mut self) {
        self.env.kill();
        self.note = u8::MAX;
    }

    /// The MIDI note this voice is sounding, if any.
    #[must_use]
    pub fn sounding(&self) -> Option<u8> {
        (!self.env.idle())
            .then_some(self.note)
            .filter(|&n| n != u8::MAX)
    }

    /// Gate-on stamp (for steal ordering).
    #[must_use]
    pub fn age(&self) -> u64 {
        self.age
    }

    /// Envelope level (for steal ordering: prefer the quietest).
    #[must_use]
    pub fn level(&self) -> f32 {
        self.env.level()
    }

    /// Reconfigure envelope + filter from block-rate parameter values.
    pub fn configure(
        &mut self,
        attack_ms: f32,
        decay_ms: f32,
        sustain: f32,
        release_ms: f32,
        cutoff_hz: f32,
        q: f32,
    ) {
        self.env.configure(attack_ms, decay_ms, sustain, release_ms);
        self.filter.set(cutoff_hz, q, self.sample_rate);
    }

    /// Render one sample (mono; the engine applies this voice's pan).
    #[inline]
    pub fn tick(&mut self, brightness: f32) -> Sample {
        if self.env.idle() {
            if let Some((note, vel, age, layer, pan, spread)) = self.pending.take() {
                self.note_on(note, vel, age, layer, pan, spread);
            } else {
                return 0.0;
            }
        }
        let f = self.fundamental.tick();
        let o2 = self.overtone2.tick();
        let o3 = self.overtone3.tick();
        let u = self.unison.tick();
        // Normalized harmonic blend; brightness fades overtones in, spread
        // beats the detuned unison against the fundamental.
        let raw = f + self.spread * 0.8 * u + brightness * (0.5 * o2 + 0.33 * o3);
        let norm = 1.0 / (1.0 + self.spread * 0.8 + brightness * 0.83);
        let env = self.env.tick();
        // ~2ms velocity glide: accents on ringing notes swell, never step
        self.velocity += (self.velocity_target - self.velocity) * 0.01;
        self.filter.tick(raw * norm) * env * self.velocity
    }

    /// Block-boundary denormal hygiene.
    pub fn flush(&mut self) {
        self.filter.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adsr_reaches_sustain_then_dies() {
        let sr = 48_000.0;
        let mut env = Adsr::new(sr);
        env.configure(1.0, 10.0, 0.5, 20.0);
        env.gate_on();
        for _ in 0..48_000 {
            env.tick();
        }
        assert!((env.level() - 0.5).abs() < 1e-3, "should sit at sustain");
        env.gate_off();
        for _ in 0..48_000 {
            env.tick();
        }
        assert!(env.idle(), "release must terminate");
        assert_eq!(env.level(), 0.0);
    }

    #[test]
    fn voice_is_silent_when_idle_and_bounded_when_sounding() {
        let mut v = Voice::new(48_000.0);
        assert_eq!(v.tick(1.0), 0.0);
        v.configure(1.0, 50.0, 0.7, 100.0, 12_000.0, 0.707);
        v.note_on(60, 127, 1, crate::LAYER_ARP, 0.0, 0.5);
        let mut peak = 0.0f32;
        for _ in 0..48_000 {
            peak = peak.max(v.tick(1.0).abs());
        }
        assert!(peak > 0.05, "voice should sound");
        assert!(peak <= 1.5, "voice must stay bounded, got {peak}");
    }

    #[test]
    fn svf_dc_passes_and_is_stable() {
        let mut f = Svf::default();
        f.set(1_000.0, 0.707, 48_000.0);
        let mut y = 0.0;
        for _ in 0..10_000 {
            y = f.tick(1.0);
            assert!(y.is_finite());
        }
        assert!((y - 1.0).abs() < 1e-3, "lowpass must pass DC, got {y}");
    }
}
