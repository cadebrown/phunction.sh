//! The expression slot: one live [`phunction_graph::expr::Program`] patched
//! onto the viewport's modulation bus. The panel writes it (on every valid
//! keystroke), the render loop reads it (every frame) — a text field as a
//! patch cable.

/// Variable names the expression sees, in evaluation order. The env is
/// `[t, beat, bass, mid, air, rms]` — time, musical time, three spectrum
/// coarse bands, and loudness.
pub const VARS: &[&str] = &["t", "beat", "bass", "mid", "air", "rms"];

/// Names of the mod-bus targets an expression can drive (fader indices).
pub const TARGETS: [&str; 4] = ["scale", "warp", "hue", "dolly"];

#[cfg(target_arch = "wasm32")]
mod imp {
    use phunction_graph::expr::Program;
    use std::cell::{Cell, RefCell};

    thread_local! {
        static SLOT: RefCell<Option<(Program, usize)>> = const { RefCell::new(None) };
        static LAST: Cell<f32> = const { Cell::new(0.0) };
    }

    /// Install (or clear) the live program and its target fader index.
    pub fn set(program: Option<(Program, usize)>) {
        SLOT.with(|s| *s.borrow_mut() = program);
    }

    /// Evaluate against `env` (see [`super::VARS`]) and add the result onto
    /// its target in `mods`, clamped to the bus range. No program, no-op.
    pub fn apply(mods: &mut [f32; 8], env: &[f32]) {
        SLOT.with(|s| {
            if let Some((program, target)) = s.borrow().as_ref() {
                let v = program.eval(env).clamp(-1.0, 1.0);
                LAST.with(|l| l.set(v));
                let t = *target % 4;
                if t == 2 {
                    // hue is a phase: let it wrap instead of pinning
                    mods[2] += v;
                } else {
                    mods[t] = (mods[t] + v).clamp(0.0, 1.0);
                }
            }
        });
    }

    /// The most recent evaluated value (for the panel's needle).
    pub fn last() -> f32 {
        LAST.with(Cell::get)
    }
}

#[cfg(target_arch = "wasm32")]
pub use imp::{apply, last, set};

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use phunction_graph::expr::Program;

    #[allow(dead_code)] // wasm render paths only; stubs keep host clippy honest
    pub fn set(_program: Option<(Program, usize)>) {}
    #[allow(dead_code)]
    pub fn apply(_mods: &mut [f32; 8], _env: &[f32]) {}
    #[allow(dead_code)]
    pub fn last() -> f32 {
        0.0
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(unused_imports)]
pub use imp::{apply, last, set};
