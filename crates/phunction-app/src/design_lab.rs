//! `/design` — the design lab, and it ships to production on purpose:
//! open source radical art software shows its palette deliberations.
//!
//! Round 2 of the aesthetic process (see docs/aesthetic.md history): three
//! complete palette candidates on the round-1 midnight-purple ground, with
//! live texture toggles and a whiteboard-twin preview. Each candidate is a
//! CSS token block in `styles/design-lab.css`; this page just switches
//! data-attributes and renders one homepage in miniature under them.

use leptos::prelude::*;

/// Palette candidates under consideration.
const PALETTES: [(&str, &str, &str); 3] = [
    ("roots", "A · three roots", "cube roots of unity: amber 85° · cyan 215° · magenta 345°"),
    (
        "stations",
        "B · eight stations",
        "the grimoire wheel: 8th roots of unity +10° — blood·rust·phosphor·ichor·verdigris·aether·sigil·philtre",
    ),
    ("golden", "C · golden continuum", "hue_k = k·137.508° — the colorize-katex rule as the whole palette"),
];

/// Named swatches shown per palette (name, css var, angle note).
fn swatch_rows(palette: &str) -> Vec<(&'static str, &'static str, &'static str)> {
    match palette {
        "stations" => vec![
            ("blood", "--a1", "10°"),
            ("rust", "--a2", "55°"),
            ("phosphor", "--a3", "100°"),
            ("ichor", "--a4", "145°"),
            ("verdigris", "--a5", "190°"),
            ("aether", "--a6", "235°"),
            ("sigil", "--a7", "280°"),
            ("philtre", "--a8", "325°"),
        ],
        "golden" => vec![
            ("k = 1", "--a1", "137.5°"),
            ("k = 2", "--a2", "275.0°"),
            ("k = 3", "--a3", "52.5°"),
            ("k = 4", "--a4", "190.0°"),
            ("k = 5", "--a5", "327.5°"),
            ("k = 6", "--a6", "105.1°"),
            ("k = 7", "--a7", "242.6°"),
            ("k = 8", "--a8", "20.1°"),
        ],
        _ => vec![
            ("ω⁰ amber", "--a1", "85°"),
            ("ω¹ cyan", "--a2", "215°"),
            ("ω² magenta", "--a3", "345°"),
        ],
    }
}

/// The `/design` route.
#[component]
pub fn DesignLab() -> impl IntoView {
    let palette = RwSignal::new("stations".to_string());
    let noise = RwSignal::new(true);
    let scan = RwSignal::new(false);
    let vignette = RwSignal::new(true);
    let whiteboard = RwSignal::new(false);

    let onoff = |b: bool| if b { "on" } else { "off" };

    view! {
        <div
            class="dlab"
            data-palette=move || palette.get()
            data-noise=move || onoff(noise.get())
            data-scan=move || onoff(scan.get())
            data-vignette=move || onoff(vignette.get())
            data-theme-preview=move || if whiteboard.get() { "whiteboard" } else { "blackboard" }
        >
            <header class="dlab-head">
                <h1>"the design lab"</h1>
                <p>"round 2 — palette × texture × theme, live. the winner becomes canon; the losers stay here as record."</p>
            </header>

            <div class="dlab-controls">
                <div class="group">
                    <span>"palette"</span>
                    {PALETTES
                        .iter()
                        .map(|(id, label, _)| {
                            let id = *id;
                            view! {
                                <button
                                    class:sel=move || palette.get() == id
                                    on:click=move |_| palette.set(id.to_string())
                                >
                                    {*label}
                                </button>
                            }
                        })
                        .collect_view()}
                </div>
                <div class="group">
                    <span>"texture"</span>
                    <button class:sel=move || noise.get() on:click=move |_| noise.update(|b| *b = !*b)>"noise"</button>
                    <button class:sel=move || scan.get() on:click=move |_| scan.update(|b| *b = !*b)>"scanlines"</button>
                    <button class:sel=move || vignette.get() on:click=move |_| vignette.update(|b| *b = !*b)>"vignette"</button>
                </div>
                <div class="group">
                    <span>"theme"</span>
                    <button class:sel=move || !whiteboard.get() on:click=move |_| whiteboard.set(false)>"blackboard"</button>
                    <button class:sel=move || whiteboard.get() on:click=move |_| whiteboard.set(true)>"whiteboard"</button>
                </div>
            </div>

            <p class="dlab-note">
                {move || {
                    PALETTES
                        .iter()
                        .find(|(id, _, _)| *id == palette.get())
                        .map(|(_, _, desc)| *desc)
                        .unwrap_or_default()
                }}
            </p>

            <div class="swatches">
                {move || {
                    swatch_rows(&palette.get())
                        .into_iter()
                        .map(|(name, var, angle)| {
                            view! {
                                <div class="swatch">
                                    <div class="chip" style=("background", format!("var({var})"))></div>
                                    <div class="meta">
                                        <span>{name}</span>
                                        <span>{angle}</span>
                                    </div>
                                </div>
                            }
                        })
                        .collect_view()
                }}
            </div>

            <section class="comp">
                <h2 class="wordmark">"phunction"</h2>
                <p class="theorem">
                    <span class="thm-label">"Theorem "</span>
                    "(phunction). "
                    <em>"Any browser is a synthesizer; any screen, a canvas."</em>
                    " "
                    <span class="qed">"∎"</span>
                </p>

                <div class="figs">
                    <span class="fig">
                        <span class="fig-glyph">"∿"</span>
                        <span class="fig-label">"fig. 1"</span>
                        <span class="fig-name">"phazor"</span>
                        <span class="fig-desc">"a DAW whose engine runs as a thread inside your audio driver."</span>
                    </span>
                    <span class="fig">
                        <span class="fig-glyph">"ℂ"</span>
                        <span class="fig-label">"fig. 2"</span>
                        <span class="fig-name">"the lab"</span>
                        <span class="fig-desc">"shader experiments on the complex plane."</span>
                    </span>
                </div>

                <div class="strip">
                    {(0..16usize)
                        .map(|i| {
                            let armed = matches!(i, 0 | 2 | 5 | 8 | 11 | 14);
                            view! {
                                <span
                                    class="cell"
                                    class:armed=armed
                                    style=("--i", i.to_string())
                                ></span>
                            }
                        })
                        .collect_view()}
                </div>

                <div class="buttons">
                    <button class="btn primary">"⏻ power on"</button>
                    <button class="btn">"▶ play"</button>
                    <button class="btn danger">"✕ panic"</button>
                </div>

                <pre class="codeblock"><code>
                    <span class="kw">"let "</span>
                    <span class="v1">"θ"</span>
                    " = "
                    <span class="v2">"ω"</span>
                    " * "
                    <span class="v3">"t"</span>
                    ";  "
                    <span class="kw">"// colorized variables: each symbol owns a hue"</span>
                    "\n"
                    <span class="v4">"z"</span>
                    " = e^(i"
                    <span class="v1">"θ"</span>
                    ")"
                </code></pre>
            </section>

            <header class="dlab-head">
                <h1>"round 3 — type"</h1>
                <p>"four castings of the same page. the question: who speaks — the professor, the wizard, or both?"</p>
            </header>
            <div class="specimens">
                {TYPE_SPECIMENS
                    .iter()
                    .map(|(face, tag, note)| {
                        view! {
                            <div class="spec" data-face=*face>
                                <span class="spec-tag">{*tag}</span>
                                <span class="spec-word">"phunction"</span>
                                <span class="spec-thm">
                                    <b>"Theorem. "</b>
                                    <em>"Any browser is a synthesizer. "</em>
                                    <span class="qed">"∎"</span>
                                </span>
                                <span class="spec-chrome">
                                    <button class="btn primary">"⏻ power on"</button>
                                    <span class="spec-cap">"fig. 1 — phazor · 128 frames"</span>
                                </span>
                                <span class="spec-tag" style="margin-top:0.8rem; margin-bottom:0;">{*note}</span>
                            </div>
                        }
                    })
                    .collect_view()}
            </div>
        </div>
    }
}

/// (data-face, label, note) for the round-3 type castings.
const TYPE_SPECIMENS: [(&str, &str, &str); 4] = [
    (
        "professor",
        "I · the professor",
        "display: Computer Modern italic · statements: CM · chrome: Iosevka — the shipped pairing",
    ),
    (
        "wizard",
        "II · the wizard",
        "display: Redaction 20 (deteriorating print) · body: Redaction 50 · chrome: Monaspace Krypton — the old site's voice",
    ),
    (
        "hermetic",
        "III · hermetic fusion",
        "display: Redaction 20 · statements: Computer Modern · chrome: Iosevka — ritual name, rigorous claims",
    ),
    (
        "manuscript",
        "IV · the manuscript",
        "display: IM Fell English italic (1600s) · statements: CM · chrome: Monaspace Krypton — oldest ink, strangest pairing",
    ),
];
