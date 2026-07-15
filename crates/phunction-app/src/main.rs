//! phunction.sh — entry point. The real shell lands next; this proves the
//! toolchain end to end (threaded wasm, Leptos mount, deploy).

use leptos::prelude::*;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}

/// Root component.
#[component]
fn App() -> impl IntoView {
    view! {
        <main style="font-family: monospace; padding: 2rem;">
            <h1>"phunction"</h1>
            <p>"⟨ signal acquired — the lab is assembling itself ⟩"</p>
        </main>
    }
}
