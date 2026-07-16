//! The patchbay: phunction-graph made touchable. Nodes drag, cables patch
//! by gesture, expr nodes take code, knobs take a thumb, sinks pick their
//! target — and every node wears a live sparkline of its own output.
//!
//! Reactivity model (qualia-inspired): the graph itself lives in a
//! thread-local (evaluated every frame in a raf loop, like `QualiaCore`'s
//! render loop); the UI subscribes to two clocks — `rev` (bumped on any
//! structural change: add/remove/patch/drag) and `frame` (bumped per raf,
//! drives sparklines and port value readouts). Views read the thread-local
//! untracked and re-render on clock ticks, so 60 fps previews don't churn
//! the reactive graph.

#![allow(clippy::cast_possible_truncation)]

use crate::rack::RackPanel;
use leptos::prelude::*;

/// Board keys a `param-out` sink can drive. `mind.*` modulate the viewport
/// bus additively; `voice.*`/`fx.*` are forwarded to the audio engine.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))] // consumed by the wasm node UI
pub const TARGET_KEYS: [&str; 7] = [
    "mind.scale",
    "mind.warp",
    "mind.hue",
    "mind.dolly",
    "voice.cutoff",
    "fx.echo",
    "fx.wash",
];

/// Additive viewport modulation produced by the patch this frame, indexed
/// scale/warp/hue/dolly. Read by the mind-field render loop.
#[cfg(target_arch = "wasm32")]
pub fn mind_mods() -> [f32; 4] {
    state::MIND_MODS.with(std::cell::Cell::get)
}

/// Cancel any in-flight gesture (cable or node drag) — Escape's job.
#[cfg(target_arch = "wasm32")]
pub fn cancel_gestures() {
    state::CABLE.with(|c| c.set(None));
    state::NODE_DRAG.with(|d| d.set(None));
    state::CANCELLED.with(|c| c.set(true));
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub fn cancel_gestures() {}

/// Ask the patchbay to replace the whole graph with `text` (a world's
/// signature patch). Applied on the next graph tick.
#[cfg(target_arch = "wasm32")]
pub fn request_patch(text: &str) {
    state::REQUEST_PATCH.with(|r| *r.borrow_mut() = Some(text.to_string()));
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub fn request_patch(_text: &str) {}

#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)] // wasm render paths only
pub fn mind_mods() -> [f32; 4] {
    [0.0; 4]
}

#[cfg(target_arch = "wasm32")]
mod state {
    use phunction_graph::graph::{Graph, NodeId};
    use std::cell::{Cell, RefCell};

    /// UI-side node record (position, sparkline ring, code mirror).
    pub struct NodeUi {
        pub id: NodeId,
        pub kind: String,
        pub x: f64,
        pub y: f64,
        pub ring: Vec<f32>,
    }

    /// An in-flight cable drag: from (node, out-port) to the cursor.
    #[derive(Clone, Copy)]
    pub struct CableDrag {
        pub from: (NodeId, usize),
        pub x: f64,
        pub y: f64,
    }

    thread_local! {
        pub static GRAPH: RefCell<Graph> = RefCell::new(Graph::new());
        pub static NODES: RefCell<Vec<NodeUi>> = const { RefCell::new(Vec::new()) };
        pub static MIND_MODS: Cell<[f32; 4]> = const { Cell::new([0.0; 4]) };
        pub static CABLE: Cell<Option<CableDrag>> = const { Cell::new(None) };
        pub static NODE_DRAG: Cell<Option<(NodeId, f64, f64)>> = const { Cell::new(None) };
        pub static SEEDED: Cell<bool> = const { Cell::new(false) };
        pub static TICKING: Cell<bool> = const { Cell::new(false) };
        pub static SAVED_REV: Cell<u64> = const { Cell::new(u64::MAX) };
        pub static SAVE_TICK: Cell<u32> = const { Cell::new(0) };
        /// A pending whole-patch install from outside (worlds/presets).
        pub static REQUEST_PATCH: RefCell<Option<String>> = const { RefCell::new(None) };
        /// A cancel arrived (Escape): repaint to drop the ghost cable.
        pub static CANCELLED: Cell<bool> = const { Cell::new(false) };
        /// Set when a cable just landed, so the click that follows the
        /// pointerup doesn't immediately unplug the fresh connection.
        pub static JUST_LANDED: Cell<bool> = const { Cell::new(false) };
    }
}

/// The patchbay rack module.
#[component]
#[allow(clippy::too_many_lines)]
pub fn Patchbay() -> impl IntoView {
    #[cfg(not(target_arch = "wasm32"))]
    return view! {
        <RackPanel title="patchbay · the constructive graph" class="span12" folded=true hue=280.0>
            <p class="pb-status">"the patchbay wakes in the browser"</p>
        </RackPanel>
    };
    #[cfg(target_arch = "wasm32")]
    {
        use phunction_graph::graph::{Ctx, NodeId};
        use phunction_graph::library;
        use phunction_graph::value::Value;
        use state::{NodeUi, CABLE, GRAPH, MIND_MODS, NODES, NODE_DRAG};
        use wasm_bindgen::JsCast;

        // node geometry shared by cable rendering and hit logic
        const NODE_W: f64 = 170.0;
        const PORT_Y0: f64 = 34.0;
        const PORT_DY: f64 = 20.0;

        let rev = RwSignal::new(0u64);
        let frame = RwSignal::new(0u64);
        let code_src = RwSignal::new(String::new());
        let code_msg = RwSignal::new(String::from(
            "the patch as text · run rebuilds the graph · to code writes the graph back",
        ));
        let code_bad = RwSignal::new(false);
        let help_open = RwSignal::new(false);
        let status = RwSignal::new(String::from(
            "drag headers to move · drag an out-port onto an in-port to patch · click an in-port to unplug",
        ));

        let bump = move || rev.update(|r| *r += 1);

        // build a graph from patch text: shared by run-patch, boot restore
        let install_patch = move |src: &str| -> Result<(), String> {
            let plan = phunction_graph::patch::compile(src, &crate::patchbay::TARGET_KEYS)
                .map_err(|e| format!("line {}: {}", e.line, e.msg))?;
            let (graph, ids) = phunction_graph::patch::build(&plan)?;
            let n = plan.nodes.len();
            let total = ids.len();
            let mut depth = vec![0usize; total];
            for _ in 0..n {
                for ((si, _), (di, _)) in &plan.links {
                    if depth[*di] <= depth[*si] {
                        depth[*di] = depth[*si] + 1;
                    }
                }
            }
            for (r, (si, _, _)) in plan.routes.iter().enumerate() {
                depth[n + r] = depth[*si] + 1;
            }
            // saved positions (order-keyed) beat the auto-layout
            let saved_xy: Vec<(f64, f64)> =
                crate::phazor_panel::wiring::load_state("phazor:patch-xy")
                    .map(|t| {
                        t.split(';')
                            .filter_map(|p| {
                                let (x, y) = p.split_once(',')?;
                                Some((x.parse().ok()?, y.parse().ok()?))
                            })
                            .collect()
                    })
                    .unwrap_or_default();
            GRAPH.with(|g| *g.borrow_mut() = graph);
            NODES.with(|nlist| {
                let mut nlist = nlist.borrow_mut();
                nlist.clear();
                let mut rows = std::collections::HashMap::<usize, usize>::new();
                for (i, id) in ids.iter().enumerate() {
                    let kind = if i < n {
                        plan.nodes[i].kind.to_string()
                    } else {
                        "param-out".to_string()
                    };
                    let row = rows.entry(depth[i]).or_insert(0);
                    *row += 1;
                    #[allow(clippy::cast_precision_loss)]
                    let (ax, ay) = (
                        26.0 + depth[i] as f64 * 210.0,
                        24.0 + (*row - 1) as f64 * 128.0,
                    );
                    let (x, y) = saved_xy
                        .get(i)
                        .copied()
                        .filter(|_| saved_xy.len() == total)
                        .unwrap_or((ax, ay));
                    nlist.push(NodeUi {
                        id: *id,
                        kind,
                        x,
                        y,
                        ring: Vec::new(),
                    });
                }
            });
            bump();
            Ok(())
        };

        // boot: the saved patch if there is one (a live set never
        // evaporates), else the starter — the bay is never an empty void
        if !state::SEEDED.with(std::cell::Cell::get) {
            state::SEEDED.with(|s| s.set(true));
            let saved = crate::phazor_panel::wiring::load_state("phazor:patch");
            let restored = saved
                .as_deref()
                .is_some_and(|text| install_patch(text).is_ok());
            if let Some(text) = saved {
                code_src.set(text);
            }
            if !restored {
                let _ = install_patch("k = knob 0.5\nl = lfo rate=k\nl -> mind.warp");
            }
        }

        // the patch clock: tick the graph every animation frame against the
        // engine's telemetry, publish mind mods, refresh sparklines
        let t0 = web_time::Instant::now();
        let already_ticking = state::TICKING.with(std::cell::Cell::get);
        state::TICKING.with(|t| t.set(true));
        if !already_ticking {
            crate::raf::raf_loop(move || {
                if let Some(text) = state::REQUEST_PATCH.with(|r| r.borrow_mut().take()) {
                    let _ = install_patch(&text);
                    code_src.set(text);
                }
                if state::CANCELLED.with(std::cell::Cell::take) {
                    rev.update(|r| *r += 1); // repaint: the ghost cable dies
                }
                let met = crate::phazor_panel::wiring::last_meter();
                // world inputs: mic level + first gamepad, polled per frame
                let uses_mic = NODES.with(|n| n.borrow().iter().any(|nd| nd.kind == "mic-in"));
                let mic = if uses_mic {
                    crate::mic::request();
                    crate::mic::level()
                } else {
                    0.0
                };
                let mut ext = [0.0f32; 8];
                ext[0] = mic;
                if NODES.with(|n| n.borrow().iter().any(|nd| nd.kind == "midi-in")) {
                    crate::midi::request();
                    let (note, vel, cc1) = crate::midi::snapshot();
                    ext[4] = note;
                    ext[5] = vel;
                    ext[6] = cc1;
                }
                if let Some(pad) = web_sys::window()
                    .and_then(|w| w.navigator().get_gamepads().ok())
                    .and_then(|pads| {
                        pads.iter()
                            .find_map(|p| p.dyn_into::<web_sys::Gamepad>().ok())
                    })
                {
                    let axes = pad.axes();
                    #[allow(clippy::cast_possible_truncation)]
                    {
                        ext[1] = axes.get(0).as_f64().unwrap_or(0.0) as f32;
                        ext[2] = axes.get(1).as_f64().unwrap_or(0.0) as f32;
                    }
                    if let Ok(b) = pad.buttons().get(7).dyn_into::<web_sys::GamepadButton>() {
                        #[allow(clippy::cast_possible_truncation)]
                        {
                            ext[3] = b.value() as f32;
                        }
                    }
                }
                let ctx = Ctx {
                    time: t0.elapsed().as_secs_f32(),
                    beats: met.beats,
                    playing: met.playing,
                    rms: (met.rms_l + met.rms_r) * 1.5,
                    peak: met.peak_l.max(met.peak_r),
                    camera: phunction_graph::value::FieldId(u32::from(
                        crate::camera::video().is_some(),
                    )),
                    ext,
                    ..Ctx::default()
                };
                GRAPH.with(|g| {
                    let mut g = g.borrow_mut();
                    g.tick(&ctx);
                    // sparkline rings from port-0 previews
                    NODES.with(|n| {
                        for node in n.borrow_mut().iter_mut() {
                            let v = match g.preview(node.id, 0) {
                                Some(Value::Signal(s) | Value::Phase(s)) => s,
                                Some(Value::Gate(b)) => f32::from(u8::from(b)),
                                _ => 0.0,
                            };
                            node.ring.push(v);
                            if node.ring.len() > 48 {
                                node.ring.remove(0);
                            }
                        }
                    });
                });
                // drain the board: mind.* accumulate, engine keys forward
                let mut mods = [0.0f32; 4];
                for (key, v) in ctx.board.borrow().iter() {
                    match *key {
                        "mind.scale" => mods[0] += v,
                        "mind.warp" => mods[1] += v,
                        "mind.hue" => mods[2] += v,
                        "mind.dolly" => mods[3] += v,
                        "voice.cutoff" => {
                            crate::phazor_panel::wiring::send(phazor_core::Command::SetParam {
                                id: phazor_core::ParamId::FilterCutoff,
                                value: 200.0 * (12_000.0f32 / 200.0).powf(v.clamp(0.0, 1.0)),
                            });
                        }
                        "fx.echo" => {
                            crate::phazor_panel::wiring::send(phazor_core::Command::SetParam {
                                id: phazor_core::ParamId::DelayMix,
                                value: v.clamp(0.0, 1.0),
                            });
                        }
                        "fx.wash" => {
                            crate::phazor_panel::wiring::send(phazor_core::Command::SetParam {
                                id: phazor_core::ParamId::ReverbMix,
                                value: v.clamp(0.0, 1.0),
                            });
                        }
                        _ => {}
                    }
                }
                MIND_MODS.with(|m| m.set(mods));
                frame.update(|f| *f += 1);
                // autosave: any structural change (rev moved) lands in
                // storage within a second — a live set never evaporates
                let tick = state::SAVE_TICK.with(|t| {
                    let v = t.get() + 1;
                    t.set(v);
                    v
                });
                if tick.is_multiple_of(60) {
                    let now_rev = rev.get_untracked();
                    if state::SAVED_REV.with(std::cell::Cell::get) != now_rev {
                        state::SAVED_REV.with(|r| r.set(now_rev));
                        let listed: Vec<(NodeId, String)> = NODES.with(|nl| {
                            nl.borrow()
                                .iter()
                                .map(|nd| (nd.id, nd.kind.clone()))
                                .collect()
                        });
                        let text =
                            GRAPH.with(|g| phunction_graph::patch::render(&g.borrow(), &listed));
                        crate::phazor_panel::wiring::save_state("phazor:patch", &text);
                        let xy: String = NODES.with(|nl| {
                            nl.borrow()
                                .iter()
                                .map(|nd| format!("{:.0},{:.0}", nd.x, nd.y))
                                .collect::<Vec<_>>()
                                .join(";")
                        });
                        crate::phazor_panel::wiring::save_state("phazor:patch-xy", &xy);
                    }
                }
                true
            });
        }

        let canvas_ref = NodeRef::<leptos::html::Div>::new();
        let local_xy = move |cx: f64, cy: f64| -> (f64, f64) {
            canvas_ref.get_untracked().map_or((cx, cy), |el| {
                let r = el.get_bounding_client_rect();
                (cx - r.left(), cy - r.top())
            })
        };

        let out_pos = move |id: NodeId, port: usize| -> (f64, f64) {
            NODES.with(|n| {
                n.borrow()
                    .iter()
                    .find(|nd| nd.id == id)
                    .map_or((0.0, 0.0), |nd| {
                        (nd.x + NODE_W, nd.y + PORT_Y0 + PORT_DY * port as f64 + 7.0)
                    })
            })
        };
        let in_pos = move |id: NodeId, port: usize| -> (f64, f64) {
            NODES.with(|n| {
                n.borrow()
                    .iter()
                    .find(|nd| nd.id == id)
                    .map_or((0.0, 0.0), |nd| {
                        (nd.x, nd.y + PORT_Y0 + PORT_DY * port as f64 + 7.0)
                    })
            })
        };

        let add_node = move |kind: &'static str| {
            GRAPH.with(|g| {
                if let Some(block) = library::build(kind) {
                    let id = g.borrow_mut().add(block);
                    NODES.with(|n| {
                        let count = n.borrow().len() as f64;
                        n.borrow_mut().push(NodeUi {
                            id,
                            kind: kind.into(),
                            x: 40.0 + (count * 60.0) % 520.0,
                            y: 40.0 + (count * 42.0) % 240.0,
                            ring: Vec::new(),
                        });
                    });
                }
            });
            bump();
        };

        let remove_node = move |id: NodeId| {
            GRAPH.with(|g| g.borrow_mut().remove(id));
            NODES.with(|n| n.borrow_mut().retain(|nd| nd.id != id));
            bump();
        };

        // code → graph: compile, build, lay out (saved positions win)
        let run_patch =
            move |_ev: web_sys::MouseEvent| match install_patch(&code_src.get_untracked()) {
                Ok(()) => {
                    code_bad.set(false);
                    code_msg.set("patched — the graph is the code now".into());
                }
                Err(e) => {
                    code_bad.set(true);
                    code_msg.set(format!("✗ {e}"));
                }
            };

        // graph → code: the inverse door
        let to_code = move |_ev: web_sys::MouseEvent| {
            let listed: Vec<(NodeId, String)> = NODES.with(|nl| {
                nl.borrow()
                    .iter()
                    .map(|nd| (nd.id, nd.kind.clone()))
                    .collect()
            });
            let text = GRAPH.with(|g| phunction_graph::patch::render(&g.borrow(), &listed));
            code_src.set(text);
            code_bad.set(false);
            code_msg.set("the graph, written down — edit and run".into());
        };

        // pointer plumbing on the canvas: node drags and cable drags both
        // end here, so one surface owns the gesture state
        let on_move = move |ev: web_sys::PointerEvent| {
            let (x, y) = local_xy(f64::from(ev.client_x()), f64::from(ev.client_y()));
            if let Some((id, dx, dy)) = NODE_DRAG.with(std::cell::Cell::get) {
                NODES.with(|n| {
                    if let Some(nd) = n.borrow_mut().iter_mut().find(|nd| nd.id == id) {
                        nd.x = (x - dx).max(0.0);
                        nd.y = (y - dy).clamp(0.0, 360.0);
                    }
                });
                bump();
            }
            if let Some(mut c) = CABLE.with(std::cell::Cell::get) {
                c.x = x;
                c.y = y;
                CABLE.with(|cell| cell.set(Some(c)));
                bump();
            }
        };
        let on_up = move |_ev: web_sys::PointerEvent| {
            NODE_DRAG.with(|d| d.set(None));
            if CABLE.with(std::cell::Cell::get).is_some() {
                CABLE.with(|c| c.set(None));
                bump();
            }
        };

        // one cable path, phase-hued by its source port type
        let cable_path = move |(fx, fy): (f64, f64), (tx, ty): (f64, f64)| -> String {
            let dx = ((tx - fx) * 0.5).max(24.0);
            format!(
                "M{fx:.1} {fy:.1} C {:.1} {fy:.1}, {:.1} {ty:.1}, {tx:.1} {ty:.1}",
                fx + dx,
                tx - dx
            )
        };

        view! {
            <RackPanel title="patchbay · the constructive graph" class="span12" folded=true hue=280.0>
                <div class="pb-shelf">
                    {library::SHELF
                        .iter()
                        .map(|meta| {
                            let id = meta.id;
                            let name = meta.name;
                            view! {
                                <button class="xport pb-add" on:click=move |_| add_node(id)>
                                    {"+ "}{name}
                                </button>
                            }
                        })
                        .collect_view()}
                </div>
                <div
                    class="pb-canvas"
                    node_ref=canvas_ref
                    on:pointermove=on_move
                    on:pointerup=on_up
                    on:pointerleave=on_up
                >
                    <svg class="pb-wires" aria-hidden="true">
                        {move || {
                            rev.get();
                            let mut paths = Vec::new();
                            GRAPH.with(|g| {
                                for (from, to, adapter) in g.borrow().cables() {
                                    let hue = g
                                        .borrow()
                                        .block(from.0)
                                        .map_or(145.0, |b| {
                                            b.meta().outputs.get(from.1).map_or(145.0, |p| p.ty.hue())
                                        });
                                    let d = cable_path(out_pos(from.0, from.1), in_pos(to.0, to.1));
                                    paths.push(view! {
                                        <g>
                                            <path class="pb-cable-shadow" d=d.clone()></path>
                                            <path
                                                class="pb-cable"
                                                class:adapted=adapter.is_some()
                                                style=("--hue", format!("{hue}"))
                                                d=d
                                            ></path>
                                        </g>
                                    }
                                    .into_any());
                                }
                            });
                            if let Some(c) = CABLE.with(std::cell::Cell::get) {
                                let d = cable_path(out_pos(c.from.0, c.from.1), (c.x, c.y));
                                paths.push(view! {
                                    <g>
                                        <path class="pb-cable-shadow" d=d.clone()></path>
                                        <path class="pb-cable live" style=("--hue", "55".to_string()) d=d></path>
                                    </g>
                                }
                                .into_any());
                            }
                            paths
                        }}
                    </svg>
                    {move || {
                        rev.get();
                        NODES.with(|nodes| {
                            nodes
                                .borrow()
                                .iter()
                                .map(|nd| {
                                    node_view(nd, rev, frame, status, remove_node)
                                })
                                .collect_view()
                        })
                    }}
                </div>
                <p class="pb-status">{move || status.get()}</p>
                <div class="pb-shelf">
                    <span class="pb-shelf-label">"library:"</span>
                    {phunction_graph::patch::LIBRARY
                        .iter()
                        .map(|(name, text)| {
                            view! {
                                <button
                                    class="xport pb-add"
                                    on:click=move |_| {
                                        crate::patchbay::request_patch(text);
                                    }
                                >
                                    {*name}
                                </button>
                            }
                        })
                        .collect_view()}
                    <button
                        class="xport pb-add"
                        on:click=move |_| help_open.update(|h| *h = !*h)
                    >"?"</button>
                </div>
                {move || help_open.get().then(|| view! {
                    <pre class="pb-help">
"the patch language, whole:
  name = kind arg=ref …      declare a node; literals become knobs
  name = expr \"a*2\" a=ref    expr nodes take code in quotes
  name -> mind.warp          route first output to a board key
  refs: name or name.port    (beat.phase, pads.trig)
kinds: knob lfo beat audio-in camera-in mic-in pads expr
       scale mix slew split param-out
keys:  mind.scale/warp/hue/dolly · voice.cutoff · fx.echo/wash
expr vars: a b c t beat rms · sin cos tri sqr saw min max clamp lerp"
                    </pre>
                })}
                <div class="pb-codebar">
                    <textarea
                        class="pb-script"
                        spellcheck="false"
                        rows="5"
                        placeholder="k = knob 0.8\nl = lfo rate=k\nl -> mind.warp"
                        prop:value=move || code_src.get()
                        on:input=move |ev| code_src.set(event_target_value(&ev))
                        aria-label="patch script"
                    ></textarea>
                    <div class="pb-codeside">
                        <button class="xport" on:click=run_patch>"run patch"</button>
                        <button class="xport" on:click=to_code>"to code"</button>
                        <p class="pb-status" class:err=move || code_bad.get()>{move || code_msg.get()}</p>
                    </div>
                </div>
            </RackPanel>
        }
    }
}

/// Render one node: header (drag + remove), typed ports, live sparkline,
/// and the block's own settings surface (knob thumb / code field / key
/// picker) — full settings, per node, in place.
#[cfg(target_arch = "wasm32")]
#[allow(clippy::too_many_lines)]
fn node_view(
    nd: &state::NodeUi,
    rev: RwSignal<u64>,
    frame: RwSignal<u64>,
    status: RwSignal<String>,
    remove_node: impl Fn(phunction_graph::graph::NodeId) + Copy + 'static,
) -> impl IntoView {
    use phunction_graph::graph::ConnectError;
    use phunction_graph::library;
    use state::{CableDrag, CABLE, GRAPH, NODES, NODE_DRAG};

    let id = nd.id;
    let kind = nd.kind.clone();
    let meta = library::SHELF
        .iter()
        .find(|m| m.id == kind)
        .copied()
        .expect("node kind is always from the shelf");
    let x = nd.x;
    let y = nd.y;
    let bump = move || rev.update(|r| *r += 1);

    let start_drag = move |ev: web_sys::PointerEvent| {
        ev.prevent_default();
        if let Some(t) = ev
            .target()
            .and_then(|t| wasm_bindgen::JsCast::dyn_into::<web_sys::Element>(t).ok())
        {
            let _ = t.set_pointer_capture(ev.pointer_id());
        }
        NODES.with(|n| {
            if let Some(nd) = n.borrow().iter().find(|nd| nd.id == id) {
                // offset from node origin so the grab point stays under the thumb
                let parent = ev
                    .target()
                    .and_then(|t| wasm_bindgen::JsCast::dyn_into::<web_sys::Element>(t).ok())
                    .and_then(|e| e.closest(".pb-canvas").ok().flatten());
                let (px, py) = parent.map_or((0.0, 0.0), |p| {
                    let r = p.get_bounding_client_rect();
                    (r.left(), r.top())
                });
                let lx = f64::from(ev.client_x()) - px;
                let ly = f64::from(ev.client_y()) - py;
                NODE_DRAG.with(|d| d.set(Some((id, lx - nd.x, ly - nd.y))));
            }
        });
    };

    let start_cable = move |port: usize| {
        move |ev: web_sys::PointerEvent| {
            ev.prevent_default();
            ev.stop_propagation();
            CABLE.with(|c| {
                c.set(Some(CableDrag {
                    from: (id, port),
                    x: 0.0,
                    y: 0.0,
                }));
            });
        }
    };

    let land_cable = move |port: usize| {
        move |ev: web_sys::PointerEvent| {
            ev.stop_propagation();
            if let Some(c) = CABLE.with(std::cell::Cell::get) {
                CABLE.with(|cell| cell.set(None));
                state::JUST_LANDED.with(|j| j.set(true));
                let result = GRAPH.with(|g| g.borrow_mut().connect(c.from, (id, port)));
                match result {
                    Ok(None) => status.set("patched · direct".into()),
                    Ok(Some(k)) => status.set(format!("patched · through a {k:?} adapter")),
                    Err(ConnectError::TypeMismatch { from, to }) => {
                        status.set(format!("refused · {from:?} does not speak {to:?}"));
                    }
                    Err(ConnectError::Cycle) => {
                        status.set("refused · that cable closes a loop".into());
                    }
                    Err(ConnectError::BadPort) => status.set("refused · no such port".into()),
                }
                bump();
            }
        }
    };

    let unplug = move |port: usize| {
        move |_ev: web_sys::MouseEvent| {
            // the click after a landing pointerup is the same gesture, not
            // an unplug request
            if state::JUST_LANDED.with(std::cell::Cell::get) {
                state::JUST_LANDED.with(|j| j.set(false));
                return;
            }
            GRAPH.with(|g| g.borrow_mut().disconnect((id, port)));
            status.set("unplugged".into());
            bump();
        }
    };

    // sparkline text from the ring, on the frame clock
    let spark = move || {
        frame.get();
        NODES.with(|n| {
            n.borrow()
                .iter()
                .find(|nd| nd.id == id)
                .map_or(String::new(), |nd| {
                    let mut pts = String::with_capacity(nd.ring.len() * 8);
                    for (i, v) in nd.ring.iter().enumerate() {
                        use core::fmt::Write;
                        let _ = write!(pts, "{i},{:.1} ", 14.0 - v.clamp(-1.2, 1.2) * 10.0);
                    }
                    pts
                })
        })
    };

    // block-specific settings surface
    let is_knob = kind == "knob";
    let is_expr = kind == "expr";
    let is_sink = kind == "param-out";
    let code_now = GRAPH.with(|g| {
        g.borrow()
            .block(id)
            .and_then(phunction_graph::library::Block::code)
    });

    view! {
        <div class="pb-node" style=("left", format!("{x}px")) style=("top", format!("{y}px"))>
            <header on:pointerdown=start_drag>
                <span>{meta.name}</span>
                <button class="pb-x" on:click=move |_| remove_node(id) aria-label="remove node">"✕"</button>
            </header>
            <div class="pb-ports">
                <div class="pb-col">
                    {meta.inputs.iter().enumerate().map(|(i, p)| {
                        view! {
                            <div class="pb-portrow">
                                <button
                                    class="pb-port in"
                                    style=("--hue", format!("{}", p.ty.hue()))
                                    attr:data-node=id.0.to_string()
                                    attr:data-port=i.to_string()
                                    on:pointerup=land_cable(i)
                                    on:click=unplug(i)
                                    aria-label=format!("input {}", p.name)
                                ></button>
                                <span class="pb-portname">{p.name}</span>
                            </div>
                        }
                    }).collect_view()}
                </div>
                <div class="pb-col out">
                    {meta.outputs.iter().enumerate().map(|(i, p)| {
                        view! {
                            <div class="pb-portrow">
                                <span class="pb-portname">{p.name}</span>
                                <button
                                    class="pb-port out"
                                    style=("--hue", format!("{}", p.ty.hue()))
                                    on:pointerdown=start_cable(i)
                                    aria-label=format!("output {}", p.name)
                                ></button>
                            </div>
                        }
                    }).collect_view()}
                </div>
            </div>
            <svg class="pb-spark" viewBox="0 0 48 16" preserveAspectRatio="none" aria-hidden="true">
                <polyline points=spark></polyline>
            </svg>
            {is_knob.then(|| view! {
                <input
                    type="range" class="pb-thumb" min="0" max="1" step="0.01"
                    aria-label="knob value"
                    prop:value=move || {
                        rev.get();
                        GRAPH
                            .with(|g| {
                                g.borrow()
                                    .block(id)
                                    .and_then(phunction_graph::library::Block::param)
                                    .unwrap_or(0.5)
                            })
                            .to_string()
                    }
                    on:input=move |ev| {
                        if let Ok(v) = event_target_value(&ev).parse::<f32>() {
                            GRAPH.with(|g| {
                                if let Some(b) = g.borrow_mut().block_mut(id) {
                                    b.set_param(v);
                                }
                            });
                            bump(); // autosave sees the turn
                        }
                    }
                />
            })}
            {is_expr.then(|| view! {
                <input
                    type="text" class="pb-code" spellcheck="false" autocomplete="off"
                    prop:value=code_now.clone().unwrap_or_default()
                    on:input=move |ev| {
                        let src = event_target_value(&ev);
                        let result = GRAPH.with(|g| {
                            g.borrow_mut().block_mut(id).map(|b| b.set_code(&src))
                        });
                        match result {
                            Some(Ok(())) => {
                                status.set("expr compiled".into());
                                bump(); // autosave sees the new program
                            }
                            Some(Err(e)) => status.set(format!("expr ✗ {e}")),
                            None => {}
                        }
                    }
                    aria-label="node program"
                />
            })}
            {is_sink.then(|| view! {
                <div class="pb-keys">
                    {crate::patchbay::TARGET_KEYS.map(|key| {
                        view! {
                            <button
                                class="pb-key"
                                class:lit=move || {
                                    rev.get();
                                    GRAPH.with(|g| {
                                        g.borrow()
                                            .block(id)
                                            .and_then(phunction_graph::library::Block::key)
                                            == Some(key)
                                    })
                                }
                                on:click=move |_| {
                                    GRAPH.with(|g| {
                                        if let Some(b) = g.borrow_mut().block_mut(id) {
                                            b.set_key(key);
                                        }
                                    });
                                    bump();
                                }
                            >
                                {key}
                            </button>
                        }
                    })}
                </div>
            })}
        </div>
    }
}
