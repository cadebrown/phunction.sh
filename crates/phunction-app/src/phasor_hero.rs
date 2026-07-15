//! The landing-page signature: a live phasor.
//!
//! A unit vector rotates on the Argand circle; its imaginary part is traced
//! as a wave flowing out of the projection point. Hue tracks the vector's
//! argument — the same "color = phase" rule that runs the whole design
//! system (docs/aesthetic.md) and the same e^{iωt} that powers every voice
//! in phazor-core. The site's thesis, drawn instead of stated.

use leptos::prelude::*;

/// Circle center / radius and trace geometry, in viewBox units.
const CX: f64 = 130.0;
const CY: f64 = 130.0;
const R: f64 = 90.0;
const TRACE_X0: f64 = 250.0;
const TRACE_DX: f64 = 3.0;
const TRACE_N: usize = 230;
/// Rotation rate: slow enough to contemplate, fast enough to feel alive.
const OMEGA: f64 = 1.1;

/// The animated figure.
#[component]
pub fn PhasorHero() -> impl IntoView {
    // One signal per animated attribute; leptos updates the exact DOM
    // attributes, nothing re-renders.
    // Seed with a frozen frame so the figure is whole before (or without)
    // the animation driver — also the reduced-motion rendering.
    let tip_x = RwSignal::new(CX + R * 0.9f64.cos());
    let tip_y = RwSignal::new(CY - R * 0.9f64.sin());
    let path_d = RwSignal::new(static_trace(0.9));
    let phase = RwSignal::new((0.9f64.to_degrees() + 85.0).rem_euclid(360.0));

    let svg_ref = NodeRef::<leptos::svg::Svg>::new();

    #[cfg(target_arch = "wasm32")]
    {
        use std::cell::Cell;
        let started = Cell::new(false);
        Effect::new(move |_| {
            let Some(svg) = svg_ref.get() else { return };
            if started.replace(true) {
                return;
            }
            drive(&svg, tip_x, tip_y, path_d, phase);
        });
    }

    view! {
        <figure class="phasor-fig" style=("--phase", move || format!("{:.1}", phase.get()))>
            <svg node_ref=svg_ref viewBox="0 0 960 260" role="img"
                aria-label="a rotating phasor on the unit circle, tracing a sine wave whose color follows its phase">
                // axes of the Argand plane + the projection baseline
                <line class="axis" x1="10" y1=CY x2="950" y2=CY></line>
                <line class="axis" x1=CX y1="15" x2=CX y2="245"></line>
                <circle class="unit" cx=CX cy=CY r=R></circle>
                // dashed projection from the tip to the start of the trace
                <line
                    class="projection"
                    x1=move || tip_x.get()
                    y1=move || tip_y.get()
                    x2=TRACE_X0
                    y2=move || tip_y.get()
                ></line>
                <line
                    class="radius"
                    x1=CX
                    y1=CY
                    x2=move || tip_x.get()
                    y2=move || tip_y.get()
                ></line>
                <circle class="tip" r="6" cx=move || tip_x.get() cy=move || tip_y.get()></circle>
                <path class="trace" d=move || path_d.get()></path>
            </svg>
            <figcaption>"fig. 0 — z = e^{iωt} · color is phase"</figcaption>
        </figure>
    }
}

/// Fill the trace with a static wave at angle `theta` (also the
/// reduced-motion rendering: the diagram is legible frozen).
fn static_trace(theta: f64) -> String {
    use core::fmt::Write as _;
    let mut d = String::with_capacity(TRACE_N * 14);
    for i in 0..TRACE_N {
        let y = CY - R * (theta - i as f64 * OMEGA / 60.0).sin();
        let x = TRACE_X0 + i as f64 * TRACE_DX;
        let _ = if i == 0 {
            write!(d, "M{x:.1} {y:.1}")
        } else {
            write!(d, " L{x:.1} {y:.1}")
        };
    }
    d
}

#[cfg(target_arch = "wasm32")]
fn drive(
    svg: &web_sys::SvgElement,
    tip_x: RwSignal<f64>,
    tip_y: RwSignal<f64>,
    path_d: RwSignal<String>,
    phase: RwSignal<f64>,
) {
    use wasm_bindgen::JsCast;

    let set_angle = move |theta: f64| {
        tip_x.set(CX + R * theta.cos());
        tip_y.set(CY - R * theta.sin());
        // CSS hue: counterclockwise from ω⁰'s amber, matching the wheel.
        phase.set((theta.to_degrees() + 85.0).rem_euclid(360.0));
    };

    let reduced = web_sys::window()
        .and_then(|w| {
            w.match_media("(prefers-reduced-motion: reduce)")
                .ok()
                .flatten()
        })
        .is_some_and(|m| m.matches());
    if reduced {
        let theta = 0.9;
        set_angle(theta);
        path_d.set(static_trace(theta));
        return;
    }

    let svg: web_sys::Element = svg.clone().unchecked_into();
    let t0 = web_time::Instant::now();
    crate::raf::raf_loop(move || {
        if !svg.is_connected() {
            return false;
        }
        let theta = t0.elapsed().as_secs_f64() * OMEGA;
        set_angle(theta);
        path_d.set(static_trace(theta));
        true
    });
}
