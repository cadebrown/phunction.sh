//! Tracing that ships to production: every event goes to the browser
//! console *and* into a bounded in-page ring the debug HUD renders.
//!
//! Hand-rolled `Subscriber` instead of `tracing-subscriber` on purpose:
//! the fmt stack drags std-heavy machinery into the bundle, and
//! `tracing-wasm` is abandoned. Events-only (spans are accepted and
//! ignored) — revisit if span timing ever earns its bytes.

#[cfg(target_arch = "wasm32")]
pub use imp::{drain_ring, init};

#[cfg(target_arch = "wasm32")]
mod imp {
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::fmt::Write as _;
    use tracing::field::{Field, Visit};
    use tracing::{Event, Level, Metadata, Subscriber};

    /// Retained log lines for the HUD (newest last).
    const RING_CAP: usize = 200;

    thread_local! {
        static RING: RefCell<VecDeque<String>> = RefCell::new(VecDeque::with_capacity(RING_CAP));
    }

    /// Copy the current ring contents (HUD polls this on open/refresh).
    pub fn drain_ring() -> Vec<String> {
        RING.with(|r| r.borrow().iter().cloned().collect())
    }

    struct ConsoleSubscriber;

    struct LineVisitor(String);

    impl Visit for LineVisitor {
        fn record_debug(&mut self, field: &Field, value: &dyn core::fmt::Debug) {
            if field.name() == "message" {
                let _ = write!(self.0, " {value:?}");
            } else {
                let _ = write!(self.0, " {}={value:?}", field.name());
            }
        }
    }

    impl Subscriber for ConsoleSubscriber {
        fn enabled(&self, _: &Metadata<'_>) -> bool {
            true
        }

        fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
            tracing::span::Id::from_u64(1)
        }

        fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
        fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
        fn enter(&self, _: &tracing::span::Id) {}
        fn exit(&self, _: &tracing::span::Id) {}

        fn event(&self, event: &Event<'_>) {
            let meta = event.metadata();
            let mut line = format!("[{}] {}", meta.level(), meta.target());
            let mut visitor = LineVisitor(String::new());
            event.record(&mut visitor);
            line.push_str(&visitor.0);

            let js: wasm_bindgen::JsValue = line.clone().into();
            match *meta.level() {
                Level::ERROR => web_sys::console::error_1(&js),
                Level::WARN => web_sys::console::warn_1(&js),
                Level::INFO => web_sys::console::info_1(&js),
                _ => web_sys::console::debug_1(&js),
            }

            RING.with(|r| {
                let mut r = r.borrow_mut();
                if r.len() == RING_CAP {
                    r.pop_front();
                }
                r.push_back(line);
            });
        }
    }

    /// Install the subscriber + panic hook. Call once at startup.
    pub fn init() {
        console_error_panic_hook::set_once();
        // Errors only if already set (hot reload) — fine to ignore.
        let _ = tracing::subscriber::set_global_default(ConsoleSubscriber);
    }
}
