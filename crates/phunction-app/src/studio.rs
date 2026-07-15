//! `/studio` — the toolkit playground and the seed of the performance
//! workspace (VISION §III). Every phunction-ui primitive docks here,
//! foldable, compact, and wired to a visible event bus: the CONSOLE strip
//! prints every control message, because a playground that can't show its
//! signals is just a brochure.

use crate::rack::{Fader, Jack, Knob, Led, LedMeter, RackPanel, XyPad};
use leptos::prelude::*;

/// Glyphs the scrambler is allowed to hallucinate.
const POOL: [char; 20] = [
    '∂', 'ψ', 'φ', 'ω', '∎', '∿', '⌬', 'ℂ', '∄', 'θ', 'π', 'Σ', '∫', 'λ', 'ζ', 'ξ', '░', '▒', '▚',
    '◬',
];
/// The phrase the text-fx module performs.
const PHRASE: &str = "we are transient functions of a longer code";

/// The text-fx module: typography as a performable medium (VISION §II).
/// Two knobs — scramble probability and per-glyph hue spread — modulate a
/// live Redaction line. Same toolkit, different medium: the point of the
/// whole exercise.
#[component]
fn TextFx() -> impl IntoView {
    let amount = RwSignal::new(0.18f32);
    let spread = RwSignal::new(9.0f32);
    let frame = RwSignal::new(0u32);

    #[cfg(target_arch = "wasm32")]
    crate::raf::raf_loop(move || {
        // ~20 Hz is plenty for a scramble; full rate reads as noise
        frame.update(|f| *f = f.wrapping_add(1));
        true
    });

    view! {
        <RackPanel title="text fx" class="span12">
            <div class="textfx" aria-label=PHRASE>
                {move || {
                    let _tick = frame.get() / 3;
                    let a = amount.get();
                    let s = spread.get();
                    PHRASE
                        .chars()
                        .enumerate()
                        .map(|(i, ch)| {
                            let scrambled = ch != ' '
                                && js_sys::Math::random() < f64::from(a) * 0.35;
                            let shown = if scrambled {
                                POOL[(js_sys::Math::random() * POOL.len() as f64) as usize % POOL.len()]
                            } else {
                                ch
                            };
                            let hue = 10.0 + i as f32 * s;
                            view! {
                                <span
                                    class="textfx-glyph"
                                    class:hot=scrambled
                                    style=("--gh", format!("{hue:.0}"))
                                >
                                    {shown}
                                </span>
                            }
                        })
                        .collect_view()
                }}
            </div>
            <div class="textfx-knobs">
                <Knob label="scramble" min=0.0 max=1.0 init=0.18 hue=325.0
                    fmt=|v| format!("{v:.2}")
                    on_value=move |v: f32| amount.set(v) />
                <Knob label="spread" min=0.0 max=45.0 init=9.0 hue=235.0
                    fmt=|v| format!("{v:.1}°")
                    on_value=move |v: f32| spread.set(v) />
            </div>
        </RackPanel>
    }
}

/// The `/studio` route.
#[component]
pub fn Studio() -> impl IntoView {
    // the visible control bus: last few events, newest first
    let bus = RwSignal::new(Vec::<String>::new());
    let log = move |line: String| {
        bus.update(|b| {
            b.insert(0, line);
            b.truncate(4);
        });
    };

    // a synthetic LFO drives the meters so the playground breathes even
    // without the audio engine powered
    let lfo = RwSignal::new(0.0f32);
    #[cfg(target_arch = "wasm32")]
    {
        let t0 = web_time::Instant::now();
        crate::raf::raf_loop(move || {
            let t = t0.elapsed().as_secs_f32();
            lfo.set(((t * 0.9).sin() * 0.5 + 0.5) * ((t * 2.3).sin() * 0.2 + 0.8));
            true
        });
    }

    view! {
        <main class="panel">
            <header class="panel-head">
                <h1>"the studio"</h1>
                <span class="sub">"⌬ toolkit playground · every control is a signal · fold what you don't need"</span>
            </header>

            <div class="rack">
                <RackPanel title="faders" class="span4">
                    <Fader label="alpha" hue=55.0 on_value=move |v: f32| log(format!("alpha → {v:.3}")) />
                    <Fader label="beta" hue=145.0 init=0.5 on_value=move |v: f32| log(format!("beta → {v:.3}")) />
                    <Fader label="gamma" hue=235.0 init=0.33 on_value=move |v: f32| log(format!("gamma → {v:.3}")) />
                    <Fader label="delta" hue=325.0 init=0.9 on_value=move |v: f32| log(format!("delta → {v:.3}")) />
                </RackPanel>

                <RackPanel title="surface" class="span5">
                    <XyPad
                        label="morph"
                        hue_x=235.0
                        hue_y=325.0
                        on_value=move |(x, y): (f32, f32)| log(format!("morph → {x:.2}, {y:.2}"))
                    />
                    <Jack label="x" />
                    <Jack label="y" />
                </RackPanel>

                <RackPanel title="signals" class="span3">
                    <LedMeter label="lfo" level=Signal::derive(move || lfo.get()) />
                    <LedMeter label="inv" level=Signal::derive(move || 1.0 - lfo.get()) />
                    <Led on={Signal::derive(move || lfo.get() > 0.85)} hue=10.0 label="hot" />
                </RackPanel>

                <RackPanel title="rotors (folded — click to open)" folded=true>
                    <Knob label="phase" min=0.0 max=360.0 init=85.0 hue=100.0
                        fmt=|v| format!("{v:.0}°")
                        on_value=move |v: f32| log(format!("phase → {v:.0}°")) />
                    <Knob label="rate" min=0.05 max=20.0 init=1.1 log=true hue=190.0
                        fmt=|v| format!("{v:.2} Hz")
                        on_value=move |v: f32| log(format!("rate → {v:.2} Hz")) />
                </RackPanel>

                <TextFx />

                <RackPanel title="console">
                    <div class="lcd lcd-wide">
                        {move || {
                            let b = bus.get();
                            if b.is_empty() {
                                view! { <span class="lcd-dim">"· touch anything — the bus prints here ·"</span> }.into_any()
                            } else {
                                b.iter().map(|l| view! { <span>{l.clone()}</span> }).collect_view().into_any()
                            }
                        }}
                    </div>
                </RackPanel>
            </div>

            <div class="keyhints">
                <span><kbd>"drag"</kbd>" faders & pad (touch works)"</span>
                <span><kbd>"dbl-click"</kbd>" reset"</span>
                <span><kbd>"▾"</kbd>" fold any module"</span>
            </div>
        </main>
    }
}
