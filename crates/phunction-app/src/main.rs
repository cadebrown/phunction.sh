//! phunction.sh — the site shell.
//!
//! Routing + panels. Heavy machinery lives in the engine crates; this crate
//! is deliberately just wiring and view code.

mod camera;
mod design_lab;
mod expr_slot;
mod fractal;
mod fun;
mod gfx_gate;
mod hud;
mod mic;
mod midi;
mod pages;
mod patchbay;
mod phasor_hero;
mod phazor_panel;
mod presets;
mod rack;
mod raf;
mod sigil;
mod studio;
mod substrate;
mod topbar;
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
    // the grimoire twin: night (default) ↔ warm parchment. The stage
    // (/phazor) stays dark by design — ink is for the prose surfaces.
    let parchment = RwSignal::new(false);
    #[cfg(target_arch = "wasm32")]
    {
        let saved = phazor_panel::wiring::load_state("phunction:theme");
        if saved.as_deref() == Some("parchment") {
            parchment.set(true);
        }
        Effect::new(move |_| {
            let on = parchment.get();
            if let Some(root) = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.document_element())
            {
                if on {
                    let _ = root.set_attribute("data-theme", "parchment");
                } else {
                    let _ = root.remove_attribute("data-theme");
                }
            }
            phazor_panel::wiring::save_state(
                "phunction:theme",
                if on { "parchment" } else { "night" },
            );
        });
    }
    let flip_theme = move |_ev: leptos::ev::MouseEvent| parchment.update(|p| *p = !*p);

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
                    <button class="theme-flip" on:click=flip_theme aria-label="switch between night and parchment">
                        {move || if parchment.get() { "night" } else { "ink" }}
                    </button>
                </div>
            </nav>
            <fun::Transmission />
            <Routes fallback=NotFound>
                <Route path=path!("/") view=Home />
                <Route path=path!("/phazor") view=phazor_panel::PhazorPage />
                <Route path=path!("/design") view=design_lab::DesignLab />
                <Route path=path!("/studio") view=studio::Studio />
                <Route path=path!("/blog") view=pages::Blog />
                <Route path=path!("/info") view=pages::Info />
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
            // the founding identity — every variable wears its station hue
            // (canon: colorized math variables are sacred)
            <p class="identity" aria-label="the phasor identity: z of t equals A times e to the i omega t plus phi">
                <span class="mvar" style=("--mh", "325")>"z"</span>
                "("<span class="mvar" style=("--mh", "190")>"t"</span>") = "
                <span class="mvar" style=("--mh", "55")>"A"</span>
                " · e"
                <sup>
                    "i("
                    <span class="mvar" style=("--mh", "145")>"ω"</span>
                    <span class="mvar" style=("--mh", "190")>"t"</span>
                    " + "
                    <span class="mvar" style=("--mh", "280")>"φ"</span>
                    ")"
                </sup>
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
