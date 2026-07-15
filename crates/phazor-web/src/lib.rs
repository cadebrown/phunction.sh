//! # phazor-web
//!
//! Everything that makes [`phazor_core`] audible in a browser: spawning the
//! engine as a *thread* inside the `AudioWorklet` (shared wasm memory,
//! nightly atomics — see `docs/audio-engine.md`), the lock-free command and
//! telemetry rings between UI and audio threads, and `AudioContext` setup.
//!
//! Native builds of this crate compile to an empty shell so the workspace
//! checks/tests as one unit; all substance is `cfg(target_arch = "wasm32")`.

#[cfg(target_arch = "wasm32")]
mod worklet;

#[cfg(target_arch = "wasm32")]
pub use worklet::{start, Phazor, PhazorProcessor};

/// Capacity of the UI→engine command ring. Sized for a hail of UI events
/// (knob drags emit ~120 Hz × a few knobs); overflow drops newest commands,
/// which the UI surfaces in the debug HUD rather than blocking.
pub const COMMAND_RING_CAPACITY: usize = 1024;

/// Capacity of the engine→UI telemetry ring (~375 `MeterFrame`s/s at 48kHz;
/// 512 gives the UI over a second of slack before frames drop).
pub const METER_RING_CAPACITY: usize = 512;
