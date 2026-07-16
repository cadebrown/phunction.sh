//! Boot-order gate between the viewport's GPU init and the score's autoplay.
//!
//! Observed (Chrome 138/macOS, not theoretical): while the `AudioWorklet` is
//! rendering *audible* output, `navigator.gpu.requestAdapter()` can stall
//! indefinitely — even though the engine keeps perfect musical time. A
//! silent running context doesn't block it. So power-on boots the audio
//! stack quietly, the viewport claims its adapter/device, and only then
//! (or after a timeout fallback) does the opening world start playing.

#[cfg(target_arch = "wasm32")]
mod imp {
    use std::cell::{Cell, RefCell};

    type Waiter = Box<dyn FnOnce()>;

    thread_local! {
        static READY: Cell<bool> = const { Cell::new(false) };
        static WAITERS: RefCell<Vec<Waiter>> = const { RefCell::new(Vec::new()) };
    }

    /// The viewport finished (or conclusively failed) GPU init — release
    /// everything waiting on the gate. Idempotent.
    pub fn mark_ready() {
        READY.with(|r| r.set(true));
        let waiters = WAITERS.with(|w| std::mem::take(&mut *w.borrow_mut()));
        for w in waiters {
            w();
        }
    }

    /// Run `f` once the gate opens (immediately if it already has).
    pub fn on_ready(f: impl FnOnce() + 'static) {
        if READY.with(Cell::get) {
            f();
        } else {
            WAITERS.with(|w| w.borrow_mut().push(Box::new(f)));
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub use imp::{mark_ready, on_ready};

#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)] // called only from wasm render paths; stub keeps clippy on host
pub fn mark_ready() {}

#[cfg(not(target_arch = "wasm32"))]
pub fn on_ready(f: impl FnOnce() + 'static) {
    f();
}
