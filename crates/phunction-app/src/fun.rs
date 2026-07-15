//! The fun layer (canon addendum: "the site plays back").
//! Transmission marquee, cursor comet, fortunes, and the fnord door.

use leptos::prelude::*;

/// Hermetic fragments for the marquee. Order is shuffled by CSS duration
/// being irrational relative to reading speed, not by RNG.
const FRAGMENTS: [&str; 10] = [
    "as above, so below",
    "∂ψ/∂t = ĤΨ",
    "we are transient functions of a longer code",
    "panta rhei",
    "fnord",
    "color is phase",
    "the demo never ends",
    "every browser a synthesizer",
    "proof by audio",
    "e^{iπ} + 1 = 0 and yet we dance",
];

/// Rotating absurd colophons (click to redraw your fortune).
const FORTUNES: [&str; 7] = [
    "compiled under a waxing moon",
    "no electrons were harmed",
    "works on my machine ∎",
    "all samples cleared with the muses",
    "16 steps, 0 regrets",
    "this website is a wavefunction until you click",
    "handle with clean hands and dirty synths",
];

/// Tilted scholarly pull-quotes with suspicious attributions. Click for
/// the next citation.
const APHORISMS: [(&str, &str); 4] = [
    (
        "Description is revelation. The world is the description, & the description is the world.",
        "W. STEVENS, ALT.",
    ),
    (
        "The map is the territory, if you render it at native resolution.",
        "A. KORZYBSKI, REV.",
    ),
    (
        "Any sufficiently rigorous magic is indistinguishable from mathematics.",
        "A. C. CLARKE, CONV.",
    ),
    (
        "God made the integers; all else is a shader.",
        "L. KRONECKER, APOCR.",
    ),
];

/// One tilted, stamped aphorism.
#[component]
pub fn Aphorism() -> impl IntoView {
    let ix = RwSignal::new(0usize);
    view! {
        <figure class="aphor" on:click=move |_| ix.update(|i| *i = (*i + 1) % APHORISMS.len())>
            <span class="aphor-glyph" aria-hidden="true">"✶"</span>
            <blockquote class="aphor-q">{move || APHORISMS[ix.get()].0}</blockquote>
            <figcaption class="aphor-by">{move || format!("— {}", APHORISMS[ix.get()].1)}</figcaption>
        </figure>
    }
}

/// The slow strip of weird text under the header.
#[component]
pub fn Transmission() -> impl IntoView {
    let line = FRAGMENTS.join("  ⌬  ");
    let doubled = format!("{line}  ⌬  {line}  ⌬  ");
    view! {
        <div class="transmission" aria-hidden="true">
            <span class="transmission-track">{doubled}</span>
        </div>
    }
}

/// Footer fortune: starts deterministic, redraws on click.
#[component]
pub fn Fortune() -> impl IntoView {
    let ix = RwSignal::new(0usize);
    view! {
        <button class="fortune" title="redraw your fortune" on:click=move |_| ix.update(|i| *i = (*i + 1) % FORTUNES.len())>
            {move || FORTUNES[ix.get()]}
        </button>
    }
}

/// Install the global toys: cursor comet + the fnord door. Call once.
#[cfg(target_arch = "wasm32")]
pub fn mount(_owner: ()) {
    use std::cell::RefCell;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;

    let window = web_sys::window().expect("window");
    let document = window.document().expect("document");
    let reduced = window
        .match_media("(prefers-reduced-motion: reduce)")
        .ok()
        .flatten()
        .is_some_and(|m| m.matches());

    // -- cursor comet: short-lived phase-hued dots; hue = pointer's own
    //    argument measured from screen center (color is phase, always).
    if !reduced {
        let doc = document.clone();
        let on_move =
            Closure::<dyn FnMut(web_sys::PointerEvent)>::new(move |ev: web_sys::PointerEvent| {
                let Some(body) = doc.body() else { return };
                let (w, h) = (
                    f64::from(body.client_width()).max(1.0),
                    f64::from(body.client_height()).max(1.0),
                );
                let (x, y) = (f64::from(ev.client_x()), f64::from(ev.client_y()));
                let hue = (y - h / 2.0).atan2(x - w / 2.0).to_degrees() + 10.0;
                let Ok(dot) = doc.create_element("span") else {
                    return;
                };
                let _ = dot.set_attribute("class", "comet-dot");
                let _ = dot.set_attribute(
                    "style",
                    &format!("left:{x:.0}px; top:{y:.0}px; --h:{hue:.0};"),
                );
                let _ = body.append_child(&dot);
                // the dot removes itself when its fade animation ends
                let dot2 = dot.clone();
                let cleanup = Closure::once_into_js(move || {
                    dot2.remove();
                });
                let _ =
                    dot.add_event_listener_with_callback("animationend", cleanup.unchecked_ref());
            });
        let _ = window
            .add_event_listener_with_callback("pointermove", on_move.as_ref().unchecked_ref());
        on_move.forget();
    }

    // -- the fnord door: type it anywhere, see briefly.
    let buffer: RefCell<String> = RefCell::new(String::new());
    let doc = document.clone();
    let on_key =
        Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |ev: web_sys::KeyboardEvent| {
            let key = ev.key();
            if key.len() != 1 {
                return;
            }
            let mut b = buffer.borrow_mut();
            b.push_str(&key.to_lowercase());
            if b.len() > 5 {
                let cut = b.len() - 5;
                b.drain(..cut);
            }
            if *b == "fnord" {
                b.clear();
                if let Some(root) = doc.document_element() {
                    let _ = root.class_list().add_1("fnord");
                    let root2 = root.clone();
                    let end = Closure::once_into_js(move || {
                        let _ = root2.class_list().remove_1("fnord");
                    });
                    let _ = web_sys::window()
                        .expect("window")
                        .set_timeout_with_callback_and_timeout_and_arguments_0(
                            end.unchecked_ref(),
                            4000,
                        );
                }
            }
        });
    let _ = window.add_event_listener_with_callback("keydown", on_key.as_ref().unchecked_ref());
    on_key.forget();
}

#[cfg(not(target_arch = "wasm32"))]
/// Native stub (browser-only toys).
pub fn mount(_owner: ()) {}
