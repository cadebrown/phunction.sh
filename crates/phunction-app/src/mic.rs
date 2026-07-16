//! Live microphone plumbing: getUserMedia(audio) → `AnalyserNode` on the
//! UI thread → one smoothed level the graph reads as `ctx.ext[0]`. A
//! first-class source, requested on first use (gesture-adjacent), silent
//! and graceful when refused.

#[cfg(target_arch = "wasm32")]
pub use imp::{level, request};

#[cfg(target_arch = "wasm32")]
mod imp {
    use std::cell::{Cell, RefCell};
    use wasm_bindgen::prelude::*;
    use wasm_bindgen_futures::JsFuture;

    thread_local! {
        static ANALYSER: RefCell<Option<web_sys::AnalyserNode>> = const { RefCell::new(None) };
        static PENDING: Cell<bool> = const { Cell::new(false) };
        static LEVEL: Cell<f32> = const { Cell::new(0.0) };
        static SCRATCH: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    }

    /// The current mic level in 0..≈1 (EMA-smoothed rms), 0 until granted.
    /// Also advances the analysis — call once per frame.
    pub fn level() -> f32 {
        ANALYSER.with(|a| {
            if let Some(an) = a.borrow().as_ref() {
                SCRATCH.with(|s| {
                    let mut buf = s.borrow_mut();
                    let n = an.fft_size() as usize;
                    if buf.len() != n {
                        buf.resize(n, 128);
                    }
                    an.get_byte_time_domain_data(&mut buf);
                    let mut acc = 0.0f32;
                    for &b in buf.iter() {
                        let v = (f32::from(b) - 128.0) / 128.0;
                        acc += v * v;
                    }
                    #[allow(clippy::cast_precision_loss)]
                    let rms = (acc / n as f32).sqrt() * 2.5;
                    LEVEL.with(|l| {
                        let prev = l.get();
                        l.set(prev + (rms.min(1.0) - prev) * 0.25);
                    });
                });
            }
        });
        LEVEL.with(Cell::get)
    }

    /// Ask for the microphone (idempotent).
    pub fn request() {
        let has = ANALYSER.with(|a| a.borrow().is_some());
        if has || PENDING.with(|p| p.replace(true)) {
            return;
        }
        leptos::task::spawn_local(async move {
            let result = open().await;
            PENDING.with(|p| p.set(false));
            match result {
                Ok(an) => ANALYSER.with(|slot| *slot.borrow_mut() = Some(an)),
                Err(e) => {
                    web_sys::console::warn_2(&"mic refused:".into(), &e);
                }
            }
        });
    }

    async fn open() -> Result<web_sys::AnalyserNode, JsValue> {
        let window = web_sys::window().ok_or("no window")?;
        let devices = window.navigator().media_devices()?;
        let constraints = web_sys::MediaStreamConstraints::new();
        constraints.set_audio(&JsValue::TRUE);
        constraints.set_video(&JsValue::FALSE);
        let stream: web_sys::MediaStream =
            JsFuture::from(devices.get_user_media_with_constraints(&constraints)?)
                .await?
                .dyn_into()?;
        // a UI-side context, separate from the engine's — analysis only
        let ctx = web_sys::AudioContext::new()?;
        let source = ctx.create_media_stream_source(&stream)?;
        let analyser = ctx.create_analyser()?;
        analyser.set_fft_size(512);
        source.connect_with_audio_node(&analyser)?;
        Ok(analyser)
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub fn level() -> f32 {
    0.0
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub fn request() {}
