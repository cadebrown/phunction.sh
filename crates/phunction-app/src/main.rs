//! phunction.sh — the site shell.
//!
//! Routing + panels. Heavy machinery lives in the engine crates; this crate
//! is deliberately just wiring and view code.

mod camera;
mod design_lab;
mod fractal;
mod fun;
mod gfx_gate;
mod hud;
mod phasor_hero;
mod phazor_panel;
mod rack;
mod raf;
mod sigil;
mod studio;
mod substrate;
mod trace;

use leptos::prelude::*;
use leptos_router::components::{Route, Router, Routes, A};
use leptos_router::path;

fn main() {
    #[cfg(target_arch = "wasm32")]
    trace::init();
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "phunction booting");
    fun::mount(());
    leptos::mount::mount_to_body(App);
}

/// Root: router + persistent chrome.
#[component]
fn App() -> impl IntoView {
    view! {
        <Router>
            <substrate::Substrate />
            <nav class="topbar">
                <A href="/" attr:class="brand">
                    <sigil::Sigil size=26 />
                    <span class="brand-path">"phunction"</span>
                </A>
                <div class="links">
                    <A href="/phazor">"phazor"</A>
                    <A href="/studio">"studio"</A>
                    <a href="https://github.com/cadebrown/phunction.sh" target="_blank" rel="noopener">"src"</a>
                </div>
            </nav>
            <fun::Transmission />
            <Routes fallback=NotFound>
                <Route path=path!("/") view=Home />
                <Route path=path!("/phazor") view=phazor_panel::PhazorPage />
                <Route path=path!("/design") view=design_lab::DesignLab />
                <Route path=path!("/studio") view=studio::Studio />
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
            // the mark: sigil behind, playable name in front, .sh dangling
            <div class="mark-stack">
                <div class="mark-sigil">
                    <sigil::Sigil size=240 />
                </div>
                // the name is an instrument: each letter strikes a phazor note
                <h1 class="wordmark playable">
                {"phunction"
                    .chars()
                    .enumerate()
                    .map(|(i, ch)| {
                        const NOTES: [u8; 9] = [57, 60, 62, 64, 67, 69, 72, 74, 76];
                        let note = NOTES[i % NOTES.len()];
                        view! {
                            <button
                                class="key"
                                class:ph=i < 2
                                style=("--kh", format!("{}", 10 + i * 39))
                                on:click=move |_| phazor_panel::wiring::play_note(note)
                            >
                                {ch}
                            </button>
                        }
                    })
                    .collect_view()}
                    <span class="tld">".sh"</span>
                </h1>
            </div>
            <p class="theorem">
                <span class="thm-label">"Theorem "</span>
                "(phunction). "
                <em>"Any browser is a synthesizer; any screen, a canvas."</em>
                <br />
                <span class="thm-label">"Proof. "</span>
                "Press power. "
                <span class="qed">"∎"</span>
            </p>
            <phasor_hero::PhasorHero />
            <fun::Aphorism />
            <nav class="figs">
                <A href="/phazor" attr:class="fig hot">
                    <span class="fig-glyph">"∿"</span>
                    <span class="fig-label">"fig. 1"</span>
                    <span class="fig-name">"phazor"</span>
                    <span class="fig-desc">
                        "a DAW whose engine runs as a thread inside your audio driver. sixteen steps, sixteen phases."
                    </span>
                </A>
                <A href="/studio" attr:class="fig">
                    <span class="fig-glyph">"ℂ"</span>
                    <span class="fig-label">"fig. 2"</span>
                    <span class="fig-name">"the studio"</span>
                    <span class="fig-desc">
                        "the toolkit playground: faders, surfaces, neural toys — every control is a signal."
                    </span>
                </A>
            </nav>
            <footer class="colophon">
                "written in rust · compiled to wasm · served flat · MIT · "
                <a href="https://github.com/cadebrown/phunction.sh" target="_blank" rel="noopener">
                    "read the source"
                </a>
                " · "
                <fun::Fortune />
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
