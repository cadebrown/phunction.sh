//! The command ABI: everything the UI can say to the engine.
//!
//! Commands are small `Copy` values with no heap behind them, so they travel
//! through a lock-free ring across the UI→audio thread boundary without
//! allocation, locking, or (crucially, for the wasm worklet) string glue.

use crate::seq::Step;

/// A continuous engine parameter addressable from the UI.
///
/// Keep this exhaustive and flat: every parameter the UI can touch is listed
/// here, which is what makes the debug inspector able to enumerate the whole
/// surface of the instrument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ParamId {
    /// Master output gain, linear 0..=1.5.
    MasterGain,
    /// Filter cutoff in Hz, `20..=20_000` (mapped logarithmically by the UI).
    FilterCutoff,
    /// Filter resonance Q, 0.5..=12.
    FilterQ,
    /// Amp envelope attack in milliseconds.
    EnvAttackMs,
    /// Amp envelope decay in milliseconds.
    EnvDecayMs,
    /// Amp envelope sustain level 0..=1.
    EnvSustain,
    /// Amp envelope release in milliseconds.
    EnvReleaseMs,
    /// Oscillator brightness: blend from pure phasor sine (0) toward
    /// phase-stacked harmonics (1).
    OscBrightness,
    /// Ping-pong delay return level, 0..=1.
    DelayMix,
    /// Delay feedback, 0..=0.9 (engine clamps; runaway loops are not art).
    DelayFeedback,
    /// Reverb return level, 0..=1.
    ReverbMix,
    /// Reverb size (comb feedback), 0..=1.
    ReverbSize,
    /// Master saturation depth, 0..=1.
    Drive,
    /// Drone layer gain, 0..=1.
    DroneLevel,
    /// Arp/pattern layer gain, 0..=1.
    ArpLevel,
    /// Lead layer gain, 0..=1.
    LeadLevel,
    /// Probability that a lead slot fires, 0..=1.
    LeadDensity,
}

/// Number of `ParamId` variants (for tables indexed by parameter).
pub const PARAM_COUNT: usize = 17;

impl ParamId {
    /// All parameters, in declaration order. Enables the inspector to walk
    /// the full parameter surface without a registry.
    pub const ALL: [Self; PARAM_COUNT] = [
        Self::MasterGain,
        Self::FilterCutoff,
        Self::FilterQ,
        Self::EnvAttackMs,
        Self::EnvDecayMs,
        Self::EnvSustain,
        Self::EnvReleaseMs,
        Self::OscBrightness,
        Self::DelayMix,
        Self::DelayFeedback,
        Self::ReverbMix,
        Self::ReverbSize,
        Self::Drive,
        Self::DroneLevel,
        Self::ArpLevel,
        Self::LeadLevel,
        Self::LeadDensity,
    ];

    /// Stable index of this parameter (its discriminant).
    #[must_use]
    pub fn index(self) -> usize {
        self as usize
    }

    /// Default value on engine construction.
    #[must_use]
    pub fn default_value(self) -> f32 {
        match self {
            Self::MasterGain | Self::DroneLevel => 0.8,
            Self::FilterCutoff => 9_000.0,
            Self::FilterQ => 0.707,
            Self::EnvAttackMs => 4.0,
            Self::EnvDecayMs => 180.0,
            Self::EnvSustain | Self::LeadLevel => 0.6,
            Self::EnvReleaseMs => 220.0,
            Self::OscBrightness | Self::DelayMix => 0.35,
            Self::DelayFeedback => 0.45,
            Self::ReverbMix => 0.4,
            Self::ReverbSize | Self::ArpLevel => 0.7,
            Self::Drive => 0.25,
            Self::LeadDensity => 0.5,
        }
    }
}

/// One instruction from the UI to the engine, applied at block boundaries
/// (live input) or at its embedded musical time (future: clip events).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command {
    /// Start the transport.
    Play,
    /// Stop the transport and rewind to zero.
    Stop,
    /// Set tempo in BPM.
    SetTempo(f64),
    /// Begin a note (live input — quantization happens UI-side if wanted).
    NoteOn {
        /// MIDI note number.
        note: u8,
        /// Velocity 0..=127.
        vel: u8,
    },
    /// Release a note.
    NoteOff {
        /// MIDI note number.
        note: u8,
    },
    /// Set a continuous parameter (smoothed engine-side).
    SetParam {
        /// Which parameter.
        id: ParamId,
        /// New target value in the parameter's native unit.
        value: f32,
    },
    /// Write one sequencer step (`None` clears it).
    SetStep {
        /// Step index, `0..StepSequencer::LEN`.
        index: u8,
        /// New step contents.
        step: Option<Step>,
    },
    /// Panic: silence all voices immediately (the "oh no" button — always
    /// reachable, always instant; a live instrument earns trust this way).
    AllNotesOff,
    /// Reseed the generative score (drone progression stays; arp skips and
    /// lead choices rehash — a new weather system over the same terrain).
    SetSeed(u32),
    /// Select the score's scale (see [`crate::score::Scale`] discriminants).
    SetScale(u8),
    /// Jump the transport to a musical position (resume after reload).
    /// Ringing voices are released, not cut — the seek itself is silent.
    SeekBeats(f64),
}
