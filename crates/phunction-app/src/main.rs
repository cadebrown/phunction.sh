//! phunction.sh — the site shell.
//!
//! Routing + panels. Heavy machinery lives in the engine crates; this crate
//! is deliberately just wiring and view code.

mod design_lab;
mod hud;
mod lab;
mod phasor_hero;
mod phazor_panel;
mod raf;
mod sigil;
mod trace;

use leptos::prelude::*;
use leptos_router::components::{Route, Router, Routes, A};
use leptos_router::path;

fn main() {
    #[cfg(target_arch = "wasm32")]
    trace::init();
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "phunction booting");
    leptos::mount::mount_to_body(App);
}

/// Root: router + persistent chrome.
#[component]
fn App() -> impl IntoView {
    view! {
        <Router>
            <nav class="topbar">
                <A href="/" attr:class="brand">
                    <sigil::Sigil size=26 />
                    <span class="brand-path">"phunction"</span>
                </A>
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
                <Route path=path!("/design") view=design_lab::DesignLab />
            </Routes>
            <hud::DebugHud />
        </Router>
    }
}

/// Landing page: the thesis, drawn then stated.
#[component]
fn Home() -> impl IntoView {
    view! {
        <main class="hero">
            <phasor_hero::PhasorHero />
            <h1 class="wordmark">"phunction"</h1>
            <p class="theorem">
                <span class="thm-label">"Theorem "</span>
                "(phunction). "
                <em>"Any browser is a synthesizer; any screen, a canvas."</em>
                <br />
                <span class="thm-label">"Proof. "</span>
                "Press power. "
                <span class="qed">"∎"</span>
            </p>
            <nav class="figs">
                <A href="/phazor" attr:class="fig hot">
                    <span class="fig-glyph">"∿"</span>
                    <span class="fig-label">"fig. 1"</span>
                    <span class="fig-name">"phazor"</span>
                    <span class="fig-desc">
                        "a DAW whose engine runs as a thread inside your audio driver. sixteen steps, sixteen phases."
                    </span>
                </A>
                <A href="/lab" attr:class="fig">
                    <span class="fig-glyph">"ℂ"</span>
                    <span class="fig-label">"fig. 2"</span>
                    <span class="fig-name">"the lab"</span>
                    <span class="fig-desc">
                        "shader experiments on the complex plane. one URL each — feed them a projector."
                    </span>
                </A>
            </nav>
            <footer class="colophon">
                "written in rust · compiled to wasm · served flat · MIT · "
                <a href="https://github.com/cadebrown/phunction.sh" target="_blank" rel="noopener">
                    "read the source"
                </a>
                " — it's part of the art"
            </footer>
        </main>
    }
}

/// 404.
#[component]
fn NotFound() -> impl IntoView {
    view! {
        <main class="nothing">
            <div><sigil::Sigil size=110 /></div>
            <h1>"∄"</h1>
            <p>"no route satisfies this address"</p>
        </main>
    }
}
