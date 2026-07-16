//! The site around the instrument: /blog and /info. Secondary to /phazor
//! by decree, same canon — theorem voice, station hues, no filler.

use leptos::prelude::*;

/// `/blog` — transmissions. Entries land as they are written; the index
/// is honest about how many exist.
#[component]
pub fn Blog() -> impl IntoView {
    view! {
        <main class="page">
            <h1>"transmissions"</h1>
            <p class="page-lede">
                "field notes from building a browser that plays itself: the engine, the graph, the little languages."
            </p>
            <article class="entry">
                <span class="entry-date">"2026 · 07"</span>
                <h2>"the machine writes its own weather"</h2>
                <p>
                    "How phazor\u{2019}s score works: a drone that walks i\u{2192}VI\u{2192}iv\u{2192}v in geological time, "
                    "an arp that breathes, a lead that rolls seeded dice \u{2014} all pure frame arithmetic, so the same "
                    "seed replays the same music forever, and a hash step every 64 beats means it never loops. "
                    "Full write-up soon; the source is the honest version in the meantime."
                </p>
                <a class="entry-link" href="https://github.com/cadebrown/phunction.sh" rel="external">"read the source \u{2192}"</a>
            </article>
            <p class="page-note">"more transmissions as they are proven."</p>
        </main>
    }
}

/// `/info` — what this place is, who runs it, what it runs on.
#[component]
pub fn Info() -> impl IntoView {
    view! {
        <main class="page">
            <h1>"info"</h1>
            <p class="page-lede">
                "phunction is an audiovisual instrument by Cade Brown \u{2014} math, code, and phase, performed live."
            </p>
            <dl class="info-grid">
                <dt>"the instrument"</dt>
                <dd>
                    <a href="/phazor">"/phazor"</a>
                    " \u{2014} a DAW whose engine runs as a thread inside your audio driver, a typed patchbay, "
                    "three little languages (expr, patch, live wgsl), and seven minds on one fullscreen field."
                </dd>
                <dt>"the stack"</dt>
                <dd>"Rust \u{2192} WASM head to toe: Leptos (UI), wgpu (fields), a hand-rolled engine on shared-memory threads. No JS framework, no build tricks \u{2014} the browser is the venue."</dd>
                <dt>"the lineage"</dt>
                <dd>
                    "performed alongside "
                    <a href="https://voidstar.sh" rel="external">"voidstar"</a>
                    " \u{2014} as above, so below."
                </dd>
                <dt>"the source"</dt>
                <dd><a href="https://github.com/cadebrown/phunction.sh" rel="external">"github.com/cadebrown/phunction.sh"</a>" \u{00b7} MIT"</dd>
            </dl>
        </main>
    }
}
