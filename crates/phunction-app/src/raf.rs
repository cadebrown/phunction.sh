//! A tiny requestAnimationFrame loop helper.

#[cfg(target_arch = "wasm32")]
pub use imp::raf_loop;

#[cfg(target_arch = "wasm32")]
mod imp {
    use std::cell::RefCell;
    use std::rc::Rc;
    use wasm_bindgen::prelude::*;

    /// Run `tick` every animation frame until it returns `false`.
    pub fn raf_loop(mut tick: impl FnMut() -> bool + 'static) {
        // The classic self-referential closure knot: the closure needs its
        // own handle to reschedule itself.
        let slot: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
        let slot2 = Rc::clone(&slot);
        *slot.borrow_mut() = Some(Closure::new(move || {
            if tick() {
                if let Some(cb) = slot2.borrow().as_ref() {
                    request(cb);
                }
            } else {
                // Drop the closure; the loop ends.
                slot2.borrow_mut().take();
            }
        }));
        request(slot.borrow().as_ref().expect("closure just installed"));
    }

    fn request(cb: &Closure<dyn FnMut()>) {
        web_sys::window()
            .expect("no window")
            .request_animation_frame(cb.as_ref().unchecked_ref())
            .expect("requestAnimationFrame failed");
    }
}
