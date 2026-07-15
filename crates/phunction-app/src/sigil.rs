//! The sigil — phunction's mark, rebuilt from the old site's compound seal.
//!
//! Layers, outside → in: a 64-tick graduation ring (a transit instrument),
//! a deliberately wonky heptagram {7/3}, eight phase dots — one per palette
//! station, at its own angle — and φ at the still heart. Outer layers
//! counter-rotate on a very slow clock (ambient-life budget, round 4);
//! `prefers-reduced-motion` parks them via CSS.

use core::f64::consts::TAU;
use leptos::prelude::*;

/// Tick marks of the graduation ring as one SVG path.
fn graduation_path() -> String {
    use core::fmt::Write as _;
    let mut d = String::new();
    for k in 0..64 {
        let a = TAU * f64::from(k) / 64.0;
        // every 8th tick reaches deeper — the cardinal + station marks
        let r0 = if k % 8 == 0 { 41.0 } else { 43.5 };
        let (s, c) = a.sin_cos();
        let _ = write!(
            d,
            "M{:.2} {:.2}L{:.2} {:.2}",
            50.0 + r0 * c,
            50.0 + r0 * s,
            50.0 + 46.0 * c,
            50.0 + 46.0 * s
        );
    }
    d
}

/// The {7/3} star polygon, hand-wobbled: each vertex is nudged by a fixed
/// pseudo-random amount so the star reads drawn, not plotted.
fn heptagram_points() -> String {
    use core::fmt::Write as _;
    let mut pts = String::new();
    for k in 0..7 {
        // visit vertices in step-3 order: 0,3,6,2,5,1,4
        let v = f64::from((k * 3) % 7);
        let a = TAU * v / 7.0 - TAU / 4.0;
        let wobble = 1.4 * (v * 2.7).sin();
        let r = 34.0 + wobble;
        let _ = write!(pts, "{:.2},{:.2} ", 50.0 + r * a.cos(), 50.0 + r * a.sin());
    }
    pts.trim_end().to_string()
}

/// The eight station dots: one per palette accent, each at its own phase
/// angle on the inner ring. The palette wearing itself.
fn station_dots() -> Vec<(f64, f64, &'static str)> {
    const STATIONS: [(f64, &str); 8] = [
        (10.0, "var(--blood)"),
        (55.0, "var(--rust)"),
        (100.0, "var(--phosphor)"),
        (145.0, "var(--ichor)"),
        (190.0, "var(--verdigris)"),
        (235.0, "var(--aether)"),
        (280.0, "var(--sigil)"),
        (325.0, "var(--philtre)"),
    ];
    STATIONS
        .iter()
        .map(|(deg, color)| {
            let a = deg.to_radians() - TAU / 4.0;
            (50.0 + 21.0 * a.cos(), 50.0 + 21.0 * a.sin(), *color)
        })
        .collect()
}

/// The mark. `size` in px; `spin` enables the slow layer rotation.
#[component]
pub fn Sigil(
    /// Rendered size in pixels.
    #[prop(default = 28)]
    size: u32,
    /// Whether the outer layers rotate (topbar: yes but slow; 404: yes).
    #[prop(default = true)]
    spin: bool,
) -> impl IntoView {
    view! {
        <svg
            class="sigil"
            class:spin=spin
            width=size
            height=size
            viewBox="0 0 100 100"
            role="img"
            aria-label="the phunction sigil"
        >
            <g class="sigil-ring">
                <path d=graduation_path() />
            </g>
            <g class="sigil-hepta">
                <polygon points=heptagram_points() />
            </g>
            <circle class="sigil-inner" cx="50" cy="50" r="13" />
            {station_dots()
                .into_iter()
                .map(|(x, y, color)| {
                    view! { <circle class="sigil-dot" cx=x cy=y r="2.1" style=("fill", color) /> }
                })
                .collect_view()}
            <text class="sigil-phi" x="50" y="50" text-anchor="middle" dominant-baseline="central">
                "φ"
            </text>
        </svg>
    }
}
