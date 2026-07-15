//! The debug HUD: a backtick-toggled overlay that ships to production.
//! Radical art software gives you the wiring diagram — build info, the
//! tracing ring, and (as subsystems register them) live stats.

use leptos::prelude::*;

/// Global overlay, mounted once in `App`. Toggle with `` ` ``.
#[component]
pub fn DebugHud() -> impl IntoView {
    let open = RwSignal::new(false);
    let lines = RwSignal::new(Vec::<String>::new());

    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::prelude::*;
        use wasm_bindgen::JsCast;
        let on_key =
            Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |ev: web_sys::KeyboardEvent| {
                if ev.key() == "`" {
                    open.update(|o| *o = !*o);
                    if open.get() {
                        lines.set(crate::trace::drain_ring());
                    }
                }
            });
        web_sys::window()
            .expect("window")
            .add_event_listener_with_callback("keydown", on_key.as_ref().unchecked_ref())
            .expect("hud keydown listener");
        on_key.forget();
    }

    view! {
        <Show when=move || open.get()>
            <aside class="hud">
                <header>
                    <span>"phunction debug"</span>
                    <span class="hud-build">{concat!("v", env!("CARGO_PKG_VERSION"))}</span>
                </header>
                <pre class="hud-log">
                    {move || {
                        let l = lines.get();
                        if l.is_empty() {
                            "· no trace events yet ·".to_string()
                        } else {
                            l.join("\n")
                        }
                    }}
                </pre>
                <footer>"` to close · logs mirror the browser console"</footer>
            </aside>
        </Show>
    }
}
