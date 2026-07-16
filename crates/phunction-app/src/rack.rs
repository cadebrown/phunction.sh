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
    /// External truth in ENGINE units: when it changes, the needle follows.
    #[prop(optional, into)]
    sync: Option<Signal<f32>>,
) -> impl IntoView {
    let init_pos = unmap(init, min, max, log).clamp(0.0, 1.0);
    let pos = RwSignal::new(init_pos);
    if let Some(sync) = sync {
        Effect::new(move |_| {
            let p = unmap(sync.get(), min, max, log).clamp(0.0, 1.0);
            if (pos.get_untracked() - p).abs() > 1e-4 {
                pos.set(p);
            }
        });
    }
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
                on:keydown=move |ev: web_sys::KeyboardEvent| {
                    let step = if ev.shift_key() { 0.005 } else { 0.02 };
                    let p = pos.get_untracked();
                    match ev.key().as_str() {
                        "ArrowUp" | "ArrowRight" => { ev.prevent_default(); apply(p + step); }
                        "ArrowDown" | "ArrowLeft" => { ev.prevent_default(); apply(p - step); }
                        "Home" => { ev.prevent_default(); apply(0.0); }
                        "End" => { ev.prevent_default(); apply(1.0); }
                        _ => {}
                    }
                }
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

/// A machined slot fader. Absolute-position drag (grab anywhere on the
/// slot, the cap comes to your finger — FL Studio rules, not hardware
/// rules), wheel to nudge, double-click to reset.
#[component]
pub fn Fader(
    /// Engraved label.
    label: &'static str,
    /// Initial normalized position `0..=1`.
    #[prop(default = 0.75)]
    init: f32,
    /// Station hue for the slot fill and readout.
    hue: f32,
    /// Receives the normalized position on every change.
    #[prop(into)]
    on_value: Callback<f32>,
    /// External truth: when this changes (presets), the cap follows.
    #[prop(optional, into)]
    sync: Option<Signal<f32>>,
) -> impl IntoView {
    let pos = RwSignal::new(init.clamp(0.0, 1.0));
    let dragging = StoredValue::new(false);
    if let Some(sync) = sync {
        Effect::new(move |_| {
            let v = sync.get().clamp(0.0, 1.0);
            if (pos.get_untracked() - v).abs() > 1e-4 {
                pos.set(v);
            }
        });
    }

    let apply = move |p: f32| {
        let p = p.clamp(0.0, 1.0);
        pos.set(p);
        on_value.run(p);
    };
    // map a pointer event to a normalized position within the slot
    let from_event = move |ev: &web_sys::PointerEvent| -> Option<f32> {
        let t = ev.current_target()?.dyn_into::<web_sys::Element>().ok()?;
        let r = t.get_bounding_client_rect();
        let frac = (f64::from(ev.client_y()) - r.top()) / r.height().max(1.0);
        #[allow(clippy::cast_possible_truncation)]
        Some(1.0 - frac.clamp(0.0, 1.0) as f32)
    };

    view! {
        <div class="fader" style=("--hue", format!("{hue}"))>
            <svg
                viewBox="0 0 36 130"
                role="slider"
                aria-label=label
                aria-valuemin="0"
                aria-valuemax="1"
                aria-valuenow=move || pos.get()
                tabindex="0"
                on:pointerdown=move |ev: web_sys::PointerEvent| {
                    ev.prevent_default();
                    if let Some(t) = ev.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) {
                        let _ = t.set_pointer_capture(ev.pointer_id());
                    }
                    dragging.set_value(true);
                    if let Some(p) = from_event(&ev) {
                        apply(p);
                    }
                }
                on:pointermove=move |ev: web_sys::PointerEvent| {
                    if dragging.get_value() {
                        if let Some(p) = from_event(&ev) {
                            apply(p);
                        }
                    }
                }
                on:pointerup=move |_| dragging.set_value(false)
                on:pointercancel=move |_| dragging.set_value(false)
                on:wheel=move |ev: web_sys::WheelEvent| {
                    ev.prevent_default();
                    apply(pos.get_untracked() - (ev.delta_y() as f32) / 1200.0);
                }
                on:dblclick=move |_| apply(init)
                on:keydown=move |ev: web_sys::KeyboardEvent| {
                    let step = if ev.shift_key() { 0.005 } else { 0.02 };
                    let p = pos.get_untracked();
                    match ev.key().as_str() {
                        "ArrowUp" | "ArrowRight" => { ev.prevent_default(); apply(p + step); }
                        "ArrowDown" | "ArrowLeft" => { ev.prevent_default(); apply(p - step); }
                        "Home" => { ev.prevent_default(); apply(0.0); }
                        "End" => { ev.prevent_default(); apply(1.0); }
                        _ => {}
                    }
                }
            >
                // slot: countersunk channel with the station-hued fill below the cap
                <rect class="fader-slot" x="14" y="8" width="8" height="114" rx="3"></rect>
                <rect
                    class="fader-fill"
                    x="16"
                    width="4"
                    y=move || 10.0 + f64::from(1.0 - pos.get()) * 106.0
                    height=move || f64::from(pos.get()) * 106.0
                ></rect>
                // machined cap with grip lines
                <g class="fader-cap-g" style=("transform", move || format!("translateY({}px)", f64::from(1.0 - pos.get()) * 106.0))>
                    <rect class="fader-cap-shadow" x="4.5" y="4" width="28" height="15" rx="2"></rect>
                    <rect class="fader-cap" x="4" y="2" width="28" height="15" rx="2"></rect>
                    <line class="fader-grip" x1="8" y1="9.5" x2="28" y2="9.5"></line>
                    <line class="fader-grip dim" x1="8" y1="6.5" x2="28" y2="6.5"></line>
                    <line class="fader-grip dim" x1="8" y1="12.5" x2="28" y2="12.5"></line>
                </g>
            </svg>
            <span class="knob-label">{label}</span>
            <span class="knob-value">{move || format!("{:.2}", pos.get())}</span>
        </div>
    }
}

/// A 2D control surface — one finger, two parameters. The crosshair wears
/// both stations; taps and drags are the same gesture.
#[component]
pub fn XyPad(
    /// Engraved label.
    label: &'static str,
    /// Hue for the X axis readout/line.
    hue_x: f32,
    /// Hue for the Y axis readout/line.
    hue_y: f32,
    /// Receives `(x, y)` in `0..=1` on every change.
    #[prop(into)]
    on_value: Callback<(f32, f32)>,
) -> impl IntoView {
    let xy = RwSignal::new((0.5f32, 0.5f32));
    let dragging = StoredValue::new(false);

    let apply = move |p: (f32, f32)| {
        let p = (p.0.clamp(0.0, 1.0), p.1.clamp(0.0, 1.0));
        xy.set(p);
        on_value.run(p);
    };
    let from_event = move |ev: &web_sys::PointerEvent| -> Option<(f32, f32)> {
        let t = ev.current_target()?.dyn_into::<web_sys::Element>().ok()?;
        let r = t.get_bounding_client_rect();
        #[allow(clippy::cast_possible_truncation)]
        Some((
            ((f64::from(ev.client_x()) - r.left()) / r.width().max(1.0)).clamp(0.0, 1.0) as f32,
            (1.0 - (f64::from(ev.client_y()) - r.top()) / r.height().max(1.0)).clamp(0.0, 1.0)
                as f32,
        ))
    };

    view! {
        <div class="xypad" style=("--hx", format!("{hue_x}")) style=("--hy", format!("{hue_y}"))>
            <div
                class="xypad-surface"
                on:pointerdown=move |ev: web_sys::PointerEvent| {
                    ev.prevent_default();
                    if let Some(t) = ev.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) {
                        let _ = t.set_pointer_capture(ev.pointer_id());
                    }
                    dragging.set_value(true);
                    if let Some(p) = from_event(&ev) {
                        apply(p);
                    }
                }
                on:pointermove=move |ev: web_sys::PointerEvent| {
                    if dragging.get_value() {
                        if let Some(p) = from_event(&ev) {
                            apply(p);
                        }
                    }
                }
                on:pointerup=move |_| dragging.set_value(false)
                on:pointercancel=move |_| dragging.set_value(false)
            >
                <span class="xypad-vline" style=("left", move || format!("{:.1}%", xy.get().0 * 100.0))></span>
                <span class="xypad-hline" style=("top", move || format!("{:.1}%", (1.0 - xy.get().1) * 100.0))></span>
                <span
                    class="xypad-dot"
                    style=("left", move || format!("{:.1}%", xy.get().0 * 100.0))
                    style=("top", move || format!("{:.1}%", (1.0 - xy.get().1) * 100.0))
                ></span>
            </div>
            <span class="knob-label">{label}</span>
            <span class="knob-value">
                {move || format!("{:.2} · {:.2}", xy.get().0, xy.get().1)}
            </span>
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
    /// Enclosure hue (pedal color-coding; 0 disables the tint).
    #[prop(default = 280.0)]
    hue: f64,
    /// Module contents.
    children: Children,
) -> impl IntoView {
    let folded = reorder::fold_signal(title, folded);
    let float = reorder::float_signal(title);
    let node = NodeRef::<leptos::html::Section>::new();
    // click-vs-drag on one surface: fold on a tap, float on a real drag
    let drag = StoredValue::new(reorder::DragState::default());

    let on_down = move |ev: web_sys::PointerEvent| {
        // only the header drags; ignore secondary buttons
        if ev.button() != 0 {
            return;
        }
        let Some(el) = node.get_untracked() else {
            return;
        };
        let rect = el.get_bounding_client_rect();
        let (px, py) = reorder::rack_origin(&el);
        drag.set_value(reorder::DragState {
            active: true,
            moved: false,
            grab_x: f64::from(ev.client_x()) - rect.left(),
            grab_y: f64::from(ev.client_y()) - rect.top(),
            rack_x: px,
            rack_y: py,
            width: rect.width(),
        });
        if let Some(t) = ev
            .target()
            .and_then(|t| wasm_bindgen::JsCast::dyn_into::<web_sys::Element>(t).ok())
        {
            let _ = t.set_pointer_capture(ev.pointer_id());
        }
    };
    let on_move = move |ev: web_sys::PointerEvent| {
        let mut st = drag.get_value();
        if !st.active {
            return;
        }
        let x = f64::from(ev.client_x()) - st.rack_x - st.grab_x;
        let y = f64::from(ev.client_y()) - st.rack_y - st.grab_y;
        // a few px of slop keeps taps folding instead of micro-dragging
        if !st.moved {
            let rect = node.get_untracked().map(|el| el.get_bounding_client_rect());
            let (ox, oy) = rect.map_or((0.0, 0.0), |r| (r.left() - st.rack_x, r.top() - st.rack_y));
            if (x - ox).abs() + (y - oy).abs() < 7.0 {
                return;
            }
            st.moved = true;
        }
        drag.set_value(st);
        float.set(Some((x.max(0.0), y.max(0.0), st.width)));
    };
    let on_up = move |ev: web_sys::PointerEvent| {
        let st = drag.get_value();
        if st.active {
            drag.set_value(reorder::DragState::default());
            if st.moved {
                ev.prevent_default();
                reorder::persist_float(title, float.get_untracked());
            }
        }
    };

    view! {
        <section
            node_ref=node
            class=format!("rack-panel {class}")
            style=("--enclosure", format!("{hue}"))
            class:folded=move || folded.get()
            class:floating=move || float.get().is_some()
            style=("left", move || float.get().map_or(String::new(), |(x, _, _)| format!("{x:.0}px")))
            style=("top", move || float.get().map_or(String::new(), |(_, y, _)| format!("{y:.0}px")))
            style=("width", move || float.get().map_or(String::new(), |(_, _, w)| format!("{w:.0}px")))
        >
            <span class="screw tl"></span>
            <span class="screw tr"></span>
            <span class="screw bl"></span>
            <span class="screw br"></span>
            <button
                class="rack-latch"
                aria-expanded=move || (!folded.get()).to_string()
                on:pointerdown=on_down
                on:pointermove=on_move
                on:pointerup=on_up
                on:pointercancel=move |_| drag.set_value(reorder::DragState::default())
                on:click=move |_| {
                    // a drag is not a fold request
                    if !drag.get_value().moved {
                        folded.update(|f| *f = !*f);
                    }
                }
                on:dblclick=move |_| {
                    // double-tap the latch: dock the panel back into the grid
                    float.set(None);
                    reorder::persist_float(title, None);
                }
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

/// The freeform workspace: grab any panel by its latch and it detaches
/// from the grid onto the canvas — position: wherever your hand left it,
/// persisted per title. Double-tap the latch to dock it back. Pointer
/// events end-to-end, so a thumb on an iPad is as first-class as a mouse.
pub mod reorder {
    use leptos::prelude::*;
    use std::cell::RefCell;
    use std::collections::HashMap;

    /// One in-flight panel drag.
    #[derive(Clone, Copy, Default)]
    pub struct DragState {
        pub active: bool,
        pub moved: bool,
        pub grab_x: f64,
        pub grab_y: f64,
        pub rack_x: f64,
        pub rack_y: f64,
        pub width: f64,
    }

    /// A detached panel's place on the canvas: (x, y, width).
    pub type Float = Option<(f64, f64, f64)>;

    thread_local! {
        static FLOATS: RefCell<HashMap<&'static str, RwSignal<Float>>> =
            RefCell::new(HashMap::new());
        static FOLDS: RefCell<HashMap<&'static str, RwSignal<bool>>> =
            RefCell::new(HashMap::new());
    }

    /// The fold signal for a panel, creatable from outside (layouts).
    pub fn fold_signal(title: &'static str, default: bool) -> RwSignal<bool> {
        FOLDS.with(|f| {
            *f.borrow_mut()
                .entry(title)
                .or_insert_with(|| RwSignal::new(default))
        })
    }

    /// A named workspace layout: which panels stand open. Panels not
    /// listed fold; unknown titles are ignored (routes without the panel).
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))] // wasm key handler only
    pub fn apply_layout(open: &[&'static str]) {
        FOLDS.with(|f| {
            for (title, sig) in f.borrow().iter() {
                sig.set(!open.contains(title));
            }
        });
    }

    /// The float signal for a panel: `Some((x, y, width))` when detached.
    pub fn float_signal(title: &'static str) -> RwSignal<Float> {
        FLOATS.with(|f| {
            *f.borrow_mut()
                .entry(title)
                .or_insert_with(|| RwSignal::new(stored_float(title)))
        })
    }

    /// The `.rack` container's viewport origin (floats are rack-relative).
    pub fn rack_origin(el: &web_sys::Element) -> (f64, f64) {
        el.closest(".rack").ok().flatten().map_or((0.0, 0.0), |r| {
            let b = r.get_bounding_client_rect();
            (b.left(), b.top())
        })
    }

    pub fn persist_float(title: &str, float: Float) {
        let key = format!("rack-float:{title}");
        match float {
            Some((x, y, w)) => {
                crate::phazor_panel::wiring::save_state(&key, &format!("{x:.0},{y:.0},{w:.0}"));
            }
            None => {
                if let Some(s) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
                    let _ = s.remove_item(&key);
                }
            }
        }
    }

    fn stored_float(title: &str) -> Float {
        let t = crate::phazor_panel::wiring::load_state(&format!("rack-float:{title}"))?;
        let mut it = t.split(',');
        Some((
            it.next()?.parse().ok()?,
            it.next()?.parse().ok()?,
            it.next()?.parse().ok()?,
        ))
    }
}
