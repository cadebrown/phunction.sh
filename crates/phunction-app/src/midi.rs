//! Web MIDI plumbing: hardware controllers into the graph. The last
//! note/velocity/mod-wheel land in cells the patch clock reads as
//! `ctx.ext[4..7]` — a keyboard on the desk is a patchable source like
//! everything else.

#[cfg(target_arch = "wasm32")]
pub use imp::{request, snapshot};

#[cfg(target_arch = "wasm32")]
mod imp {
    use std::cell::Cell;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen_futures::JsFuture;

    thread_local! {
        static NOTE: Cell<f32> = const { Cell::new(0.0) };
        static VEL: Cell<f32> = const { Cell::new(0.0) };
        static CC1: Cell<f32> = const { Cell::new(0.0) };
        static PENDING: Cell<bool> = const { Cell::new(false) };
        static WIRED: Cell<bool> = const { Cell::new(false) };
    }

    /// Latest (note, velocity, mod-wheel), each normalized 0..=1.
    pub fn snapshot() -> (f32, f32, f32) {
        (
            NOTE.with(Cell::get),
            VEL.with(Cell::get),
            CC1.with(Cell::get),
        )
    }

    /// Ask for MIDI access (idempotent; no sysex, so no scary prompt).
    pub fn request() {
        if WIRED.with(Cell::get) || PENDING.with(|p| p.replace(true)) {
            return;
        }
        leptos::task::spawn_local(async move {
            let result = open().await;
            PENDING.with(|p| p.set(false));
            match result {
                Ok(()) => WIRED.with(|w| w.set(true)),
                Err(e) => {
                    web_sys::console::warn_2(&"midi refused:".into(), &e);
                }
            }
        });
    }

    async fn open() -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("no window")?;
        let access: web_sys::MidiAccess = JsFuture::from(window.navigator().request_midi_access()?)
            .await?
            .dyn_into()?;
        let on_msg = Closure::<dyn FnMut(web_sys::MidiMessageEvent)>::new(
            move |ev: web_sys::MidiMessageEvent| {
                if let Ok(data) = ev.data() {
                    if data.len() >= 3 {
                        let status = data[0] & 0xF0;
                        match status {
                            0x90 if data[2] > 0 => {
                                NOTE.with(|n| n.set(f32::from(data[1]) / 127.0));
                                VEL.with(|v| v.set(f32::from(data[2]) / 127.0));
                            }
                            0x80 | 0x90 => VEL.with(|v| v.set(0.0)),
                            0xB0 if data[1] == 1 => {
                                CC1.with(|c| c.set(f32::from(data[2]) / 127.0));
                            }
                            _ => {}
                        }
                    }
                }
            },
        );
        // wire every input, present and future-agnostic (re-request re-wires)
        let inputs = access.inputs();
        let entries = js_sys::try_iter(&inputs)?.ok_or("inputs not iterable")?;
        for entry in entries {
            let entry = entry?;
            // Map iterator yields [key, value] pairs
            let pair = js_sys::Array::from(&entry);
            let input: web_sys::MidiInput = pair.get(1).dyn_into()?;
            input.set_onmidimessage(Some(on_msg.as_ref().unchecked_ref()));
        }
        on_msg.forget();
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub fn snapshot() -> (f32, f32, f32) {
    (0.0, 0.0, 0.0)
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub fn request() {}
