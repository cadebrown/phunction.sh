//! `AudioWorklet` thread bootstrap (in progress).
//!
//! The plan (wasm-bindgen `wasm-audio-worklet` pattern, verified against
//! firewheel-web-audio): the main thread creates the `AudioContext`, adds a
//! tiny processor module, and spawns the engine as a thread whose stack
//! lives in this module's shared memory. `process()` then pulls commands
//! from the rtrb ring and renders directly into the worklet's output.

/// Placeholder export until the bootstrap lands (next commit).
pub const PENDING: &str = "worklet bootstrap lands with the audio milestone";
