//! The rack: modular-synth UI components, machined, and wired to the
//! engine for real. A [`Knob`] doesn't *represent* a parameter — it sends
//! [`Command::SetParam`] down the ring; an [`LedMeter`] doesn't animate —
//! it reads `MeterFrame` telemetry. Skeuomorphism with a real signal path,
//! per the canon: instruments move because they are instruments.

use leptos::prelude::*;
use wasm_bindgen::JsCast;

/// Knob rotation sweep: ±135° like every synth knob since the dawn of volts.
const SWEEP: f64 = 135.0;

/// Map a normalized position to an engine value.
fn map_pos(pos: f32, min: f32, max: f32, log: bool) -> f32 {
    if log {
        min * (max / min).powf(pos)
    } else {
        min + pos * (max - min)
    }
}

/// Inverse of [`map_pos`].
fn unmap(value: f32, min: f32, max: f32, log: bool) -> f32 {
    if log {
        (value / min).ln() / (max / min).ln()
    } else {
        (value - min) / (max - min)
    }
}

/// SVG arc path from -SWEEP to `deg` on radius `r` around (40,40).
fn arc_path(deg: f64, r: f64) -> String {
    let a0 = (-SWEEP - 90.0).to_radians();
    let a1 = (deg - 90.0).to_radians();
    let (x0, y0) = (40.0 + r * a0.cos(), 40.0 + r * a0.sin());
    let (x1, y1) = (40.0 + r * a1.cos(), 40.0 + r * a1.sin());
    let large = u8::from(deg + SWEEP > 180.0);
    format!("M{x0:.2} {y0:.2} A{r} {r} 0 {large} 1 {x1:.2} {y1:.2}")
}

/// A machined rotary control. Drag vertically (shift = fine), scroll to
/// nudge, double-click to reset. Every movement calls `on_value` with the
/// mapped engine value — the caller wires it to the ring.
#[component]
pub fn Knob(
    /// Engraved label.
    label: &'static str,
    /// Minimum engine value.
    min: f32,
    /// Maximum engine value.
    max: f32,
    /// Initial engine value.
    init: f32,
    /// Logarithmic mapping (frequencies want this).
    #[prop(default = false)]
    log: bool,
    /// Station hue (degrees) for the value arc and readout.
    hue: f32,
    /// Value formatter for the readout.
    fmt: fn(f32) -> String,
    /// Receives the mapped engine value on every change.
    #[prop(into)]
    on_value: Callback<f32>,
) -> impl IntoView {
    let init_pos = unmap(init, min, max, log).clamp(0.0, 1.0);
    let pos = RwSignal::new(init_pos);
    let value = move || map_pos(pos.get(), min, max, log);
    let angle = move || -SWEEP + f64::from(pos.get()) * 2.0 * SWEEP;

    let apply = move |p: f32| {
        let p = p.clamp(0.0, 1.0);
        pos.set(p);
        on_value.run(map_pos(p, min, max, log));
    };

    // drag state: (position at grab, pointer y at grab)
    let grab = StoredValue::new(None::<(f32, f32)>);

    view! {
        <div class="knob" style=("--hue", format!("{hue}"))>
            <svg
                viewBox="0 0 80 80"
                role="slider"
                aria-label=label
                aria-valuemin=min
                aria-valuemax=max
                aria-valuenow=value
                tabindex="0"
                on:pointerdown=move |ev: web_sys::PointerEvent| {
                    ev.prevent_default();
                    if let Some(t) = ev.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) {
                        let _ = t.set_pointer_capture(ev.pointer_id());
                    }
                    grab.set_value(Some((pos.get_untracked(), ev.client_y() as f32)));
                }
                on:pointermove=move |ev: web_sys::PointerEvent| {
                    if let Some((p0, y0)) = grab.get_value() {
                        let fine = if ev.shift_key() { 0.2 } else { 1.0 };
                        apply(p0 + (y0 - ev.client_y() as f32) / 160.0 * fine);
                    }
                }
                on:pointerup=move |_| grab.set_value(None)
                on:pointercancel=move |_| grab.set_value(None)
                on:wheel=move |ev: web_sys::WheelEvent| {
                    ev.prevent_default();
                    apply(pos.get_untracked() - (ev.delta_y() as f32) / 1200.0);
                }
                on:dblclick=move |_| apply(init_pos)
            >
                // tick marks around the sweep
                {(0..=10)
                    .map(|i| {
                        let a = (-SWEEP + f64::from(i) * SWEEP / 5.0 - 90.0).to_radians();
                        view! {
                            <line
                                class="knob-tick"
                                x1=40.0 + 34.0 * a.cos()
                                y1=40.0 + 34.0 * a.sin()
                                x2=40.0 + 37.5 * a.cos()
                                y2=40.0 + 37.5 * a.sin()
                            ></line>
                        }
                    })
                    .collect_view()}
                // the value arc, in the parameter's station hue
                <path class="knob-arc-bg" d=arc_path(SWEEP, 30.0)></path>
                <path class="knob-arc" d=move || arc_path(angle(), 30.0)></path>
                // machined body, outside in: countersink, cast shadow, side
                // wall, knurled skirt (28 real teeth), domed cap, specular
                <circle class="knob-sink" cx="40" cy="40" r="27"></circle>
                <ellipse class="knob-shadow" cx="41.5" cy="43" rx="25" ry="24"></ellipse>
                <circle class="knob-body" cx="40" cy="40" r="25"></circle>
                <circle class="knob-wall" cx="40" cy="40" r="23.8"></circle>
                {(0..28)
                    .map(|i| {
                        let a = core::f64::consts::TAU * f64::from(i) / 28.0;
                        view! {
                            <line
                                class="knob-tooth"
                                x1=40.0 + 20.6 * a.cos()
                                y1=40.0 + 20.6 * a.sin()
                                x2=40.0 + 24.2 * a.cos()
                                y2=40.0 + 24.2 * a.sin()
                            ></line>
                        }
                    })
                    .collect_view()}
                <circle class="knob-cap" cx="40" cy="40" r="16"></circle>
                <circle class="knob-dome" cx="37.5" cy="37" r="12"></circle>
                <path class="knob-spec" d="M27 31 A16.5 16.5 0 0 1 38 24"></path>
                // indicator
                <g style=("transform", move || format!("rotate({}deg)", angle())) class="knob-rotor">
                    <line class="knob-needle" x1="40" y1="28" x2="40" y2="17"></line>
                </g>
            </svg>
            <span class="knob-label">{label}</span>
            <span class="knob-value">{move || fmt(value())}</span>
        </div>
    }
}

/// A panel lamp. Lit state and hue come from the caller; glow is CSS.
#[component]
pub fn Led(
    /// Whether the lamp is lit.
    #[prop(into)]
    on: Signal<bool>,
    /// Station hue in degrees.
    hue: f32,
    /// Tiny engraved label under the lamp.
    #[prop(default = "")]
    label: &'static str,
) -> impl IntoView {
    view! {
        <span class="led-wrap">
            <span class="led" class:lit=move || on.get() style=("--hue", format!("{hue}"))></span>
            {(!label.is_empty()).then(|| view! { <span class="led-label">{label}</span> })}
        </span>
    }
}

/// A vertical LED ladder meter: ichor floor, phosphor shoulder, blood
/// ceiling — the classic VU ladder translated to the stations.
#[component]
pub fn LedMeter(
    /// Channel label engraved under the ladder.
    label: &'static str,
    /// Level in 0..=1, read every frame.
    level: Signal<f32>,
) -> impl IntoView {
    const SEGS: usize = 12;
    view! {
        <div class="ledmeter">
            <div class="ledmeter-col">
                {(0..SEGS)
                    .rev()
                    .map(|i| {
                        let hue = match i {
                            10.. => 10.0,   // blood
                            7.. => 100.0,   // phosphor
                            _ => 145.0,     // ichor
                        };
                        let threshold = (i as f32 + 0.5) / SEGS as f32;
                        view! {
                            <span
                                class="seg"
                                class:lit={move || level.get() > threshold}
                                style=("--hue", format!("{hue}"))
                            ></span>
                        }
                    })
                    .collect_view()}
            </div>
            <span class="ledmeter-label">{label}</span>
        </div>
    }
}

/// A 3.5mm jack socket with its hex nut — the honest Eurorack anchor.
/// Decorative today, modulation routing tomorrow; the nut is real either way.
#[component]
pub fn Jack(
    /// Engraved label under the socket.
    label: &'static str,
) -> impl IntoView {
    // hex nut vertices, flat side up
    let mut nut = String::new();
    for i in 0..6 {
        use core::fmt::Write as _;
        let a = core::f64::consts::TAU * (f64::from(i) + 0.5) / 6.0;
        let _ = write!(
            nut,
            "{:.2},{:.2} ",
            20.0 + 17.0 * a.cos(),
            20.0 + 17.0 * a.sin()
        );
    }
    view! {
        <div class="jack">
            <svg viewBox="0 0 40 40" aria-hidden="true">
                <polygon class="jack-nut" points=nut.trim_end().to_string()></polygon>
                <circle class="jack-ring" cx="20" cy="20" r="12"></circle>
                <circle class="jack-throat" cx="20" cy="20" r="9"></circle>
                <circle class="jack-socket" cx="20" cy="20" r="6"></circle>
                <path class="jack-glint" d="M12 15 A9.5 9.5 0 0 1 18 10.6"></path>
            </svg>
            <span class="jack-label">{label}</span>
        </div>
    }
}

/// A rack module: machined panel, engraved title, corner screws.
/// Collapsible: the title bar is a latch — click folds the module down to
/// its faceplate strip (TouchDesigner density rule: everything folds).
#[component]
pub fn RackPanel(
    /// Engraved module name.
    title: &'static str,
    /// Extra classes (grid spans).
    #[prop(default = "")]
    class: &'static str,
    /// Start folded (compact workspaces).
    #[prop(default = false)]
    folded: bool,
    /// Module contents.
    children: Children,
) -> impl IntoView {
    let folded = RwSignal::new(folded);
    view! {
        <section class=format!("rack-panel {class}") class:folded=move || folded.get()>
            <span class="screw tl"></span>
            <span class="screw tr"></span>
            <span class="screw bl"></span>
            <span class="screw br"></span>
            <button
                class="rack-latch"
                aria-expanded=move || (!folded.get()).to_string()
                on:click=move |_| folded.update(|f| *f = !*f)
            >
                <span class="rack-fold" aria-hidden="true">
                    {move || if folded.get() { "▸" } else { "▾" }}
                </span>
                <span class="rack-title">{title}</span>
            </button>
            <div class="rack-body" class:hidden=move || folded.get()>{children()}</div>
        </section>
    }
}
