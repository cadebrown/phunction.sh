//! Live camera plumbing: getUserMedia → hidden `<video>` → per-frame GPU
//! copy (WebGPU external image path). The camera is a first-class light
//! source for the viewport (VISION §II: video as clay).

#[cfg(target_arch = "wasm32")]
pub use imp::{request, video};

#[cfg(target_arch = "wasm32")]
mod imp {
    use std::cell::RefCell;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen_futures::JsFuture;

    thread_local! {
        static VIDEO: RefCell<Option<web_sys::HtmlVideoElement>> = const { RefCell::new(None) };
        static PENDING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }

    /// The live camera element, once granted and playing.
    pub fn video() -> Option<web_sys::HtmlVideoElement> {
        VIDEO.with(|v| v.borrow().clone())
    }

    /// Ask for the camera (idempotent; call from a gesture-adjacent path).
    /// On success the hidden `<video>` starts feeding [`video`].
    pub fn request() {
        if video().is_some() || PENDING.with(|p| p.replace(true)) {
            return;
        }
        leptos::task::spawn_local(async move {
            let result = open().await;
            PENDING.with(|p| p.set(false));
            match result {
                Ok(v) => VIDEO.with(|slot| *slot.borrow_mut() = Some(v)),
                Err(e) => {
                    tracing::warn!("camera refused: {e:?}");
                    web_sys::console::warn_2(&"camera refused:".into(), &e);
                }
            }
        });
    }

    async fn open() -> Result<web_sys::HtmlVideoElement, JsValue> {
        let window = web_sys::window().ok_or("no window")?;
        let devices = window.navigator().media_devices()?;
        let constraints = web_sys::MediaStreamConstraints::new();
        constraints.set_video(&JsValue::TRUE);
        constraints.set_audio(&JsValue::FALSE);
        let stream: web_sys::MediaStream =
            JsFuture::from(devices.get_user_media_with_constraints(&constraints)?)
                .await?
                .dyn_into()?;

        let document = window.document().ok_or("no document")?;
        let video: web_sys::HtmlVideoElement = document.create_element("video")?.dyn_into()?;
        video.set_muted(true);
        video.set_autoplay(true);
        let _ = video.set_attribute("playsinline", "");
        let _ = video.set_attribute("style", "display:none");
        video.set_src_object(Some(&stream));
        document.body().ok_or("no body")?.append_child(&video)?;
        let _ = JsFuture::from(video.play()?).await;
        Ok(video)
    }
}
