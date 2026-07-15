//! phunction.sh — the site shell.
//!
//! Routing + panels. Heavy machinery lives in the engine crates; this crate
//! is deliberately just wiring and view code.

mod lab;
mod phazor_panel;
mod raf;

use leptos::prelude::*;
use leptos_router::components::{Route, Router, Routes, A};
use leptos_router::path;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}

/// Root: router + persistent chrome.
#[component]
fn App() -> impl IntoView {
    view! {
        <Router>
            <nav class="topbar">
                <A href="/" attr:class="brand">"phunction"</A>
                <div class="links">
                    <A href="/phazor">"phazor"</A>
                    <A href="/lab">"lab"</A>
                    <a href="https://github.com/cadebrown/phunction.sh" target="_blank" rel="noopener">"src"</a>
                </div>
            </nav>
            <Routes fallback=NotFound>
                <Route path=path!("/") view=Home />
                <Route path=path!("/phazor") view=phazor_panel::PhazorPage />
                <Route path=path!("/lab") view=lab::LabIndex />
                <Route path=path!("/lab/:id") view=lab::LabView />
            </Routes>
        </Router>
    }
}

/// Landing page.
#[component]
fn Home() -> impl IntoView {
    view! {
        <main class="hero">
            <h1 class="glitch">"phunction"</h1>
            <p class="tagline">"⟨ psychedelic code futurism · math you can hear · shaders you can play ⟩"</p>
            <div class="portals">
                <A href="/phazor" attr:class="portal">
                    <span class="portal-glyph">"∿"</span>
                    <span class="portal-name">"phazor"</span>
                    <span class="portal-desc">"the browser DAW — a Rust engine threaded into your AudioWorklet"</span>
                </A>
                <A href="/lab" attr:class="portal">
                    <span class="portal-glyph">"⌬"</span>
                    <span class="portal-name">"the lab"</span>
                    <span class="portal-desc">"fullscreen shader experiments — plug into a projector and go"</span>
                </A>
            </div>
            <p class="fine">"all Rust · all open · runs on anything with a browser"</p>
        </main>
    }
}

/// 404.
#[component]
fn NotFound() -> impl IntoView {
    view! {
        <main class="hero">
            <h1>"∅"</h1>
            <p class="tagline">"this address does not converge"</p>
        </main>
    }
}
