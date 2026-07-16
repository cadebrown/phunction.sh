//! # phazor-core
//!
//! The heart of **phazor**, phunction's browser DAW: a sample-accurate audio
//! engine written as pure Rust — no web APIs, no I/O, no allocation on the
//! audio path after construction.
//!
//! ## Design invariants (the non-negotiables)
//!
//! 1. **The engine is the clock.** [`Transport`] counts frames inside
//!    `process()`; everything musical (sequencer steps, note events) is
//!    derived from the frame counter, never from wall-clock time.
//! 2. **Sample-accurate events.** Events landing mid-buffer split the buffer;
//!    a note starting at frame 93 of a 128-frame quantum starts at frame 93.
//! 3. **Realtime-safe `process()`.** No allocation, no locking, no syscalls,
//!    no unbounded work. Command intake is a lock-free ring drained at block
//!    start (see [`engine::Engine::drain`]).
//! 4. **Bytes-only ABI.** [`Command`] and [`MeterFrame`] are `Copy` PODs so
//!    they cross thread (and wasm) boundaries without strings or heap — this
//!    is what lets the same engine run in an `AudioWorklet` thread.
//!
//! The crate is target-agnostic on purpose: everything here runs (and is
//! tested, and is benched) natively. The browser bootstrap lives in
//! `phazor-web`.

pub mod engine;
pub mod event;
pub mod fx;
pub mod math;
pub mod meter;
pub mod score;
pub mod seq;
pub mod spectrum;
pub mod transport;
pub mod voice;

pub use engine::Engine;
pub use event::{Command, ParamId};
pub use meter::MeterFrame;
pub use score::{Scale, Score};
pub use seq::{Step, StepSequencer};
pub use spectrum::{Spectrum, BANDS};
pub use transport::Transport;

/// The drone layer: chord roots in geological time, long envelopes, wide.
pub const LAYER_DRONE: u8 = 0;
/// The arp/pattern layer: the user's 16 pads + the score's arp, snappy.
pub const LAYER_ARP: u8 = 1;
/// The lead layer: semirandom pentatonic calls an octave or two up.
pub const LAYER_LEAD: u8 = 2;
/// Number of voice layers.
pub const LAYER_COUNT: usize = 3;

/// One audio sample. The engine is `f32` end to end, matching Web Audio.
pub type Sample = f32;

/// Frames per render quantum in every shipping Web Audio implementation
/// (July 2026: `renderSizeHint` is still an origin trial — treat 128 as law).
/// The engine handles arbitrary block sizes; this constant is for sizing
/// rings and tests to realistic conditions.
pub const QUANTUM: usize = 128;
