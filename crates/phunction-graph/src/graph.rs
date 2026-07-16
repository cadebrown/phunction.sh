//! The graph: typed nodes, validated edges, topological evaluation.

use crate::library::Block;
use crate::value::{adapt, compat, AdapterKind, Compat, PortType, Value};
use crate::value::{AudioId, FieldId};
use core::cell::RefCell;

/// Node handle (index; stable for the life of the node).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub usize);

/// One patched cable, possibly wearing an inline adapter.
#[derive(Debug, Clone, Copy)]
struct Edge {
    from: (NodeId, usize),
    to: (NodeId, usize),
    adapter: Option<AdapterKind>,
}

/// Per-tick world snapshot the blocks read, plus the output board sinks
/// write. The runtime builds one each frame.
pub struct Ctx {
    /// Seconds since the graph started.
    pub time: f32,
    /// Musical position from the engine.
    pub beats: f64,
    /// Transport running.
    pub playing: bool,
    /// Engine RMS (summed, scaled).
    pub rms: f32,
    /// Engine peak.
    pub peak: f32,
    /// The live master audio bus handle.
    pub audio_bus: AudioId,
    /// The live camera field handle (0 until granted).
    pub camera: FieldId,
    /// External world inputs the runtime fills per frame:
    /// `[mic, pad_x, pad_y, pad_trigger, …]` — blocks read, never write.
    pub ext: [f32; 8],
    /// Where [`crate::library::ParamOut`] sinks write: `(key, value)` per
    /// tick, drained by the runtime.
    pub board: RefCell<Vec<(&'static str, f32)>>,
}

impl Default for Ctx {
    fn default() -> Self {
        Self {
            time: 0.0,
            beats: 0.0,
            playing: false,
            rms: 0.0,
            peak: 0.0,
            audio_bus: AudioId(1),
            camera: FieldId(0),
            ext: [0.0; 8],
            board: RefCell::new(Vec::new()),
        }
    }
}

/// Why a connection was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectError {
    /// No such node/port.
    BadPort,
    /// Types have no meaning together (compat = Never).
    TypeMismatch {
        /// Source port type.
        from: PortType,
        /// Destination port type.
        to: PortType,
    },
    /// The cable would create a cycle.
    Cycle,
}

/// The patch: blocks + cables. Evaluated as a topological sweep.
#[derive(Default)]
pub struct Graph {
    nodes: Vec<Option<Box<dyn Block>>>,
    edges: Vec<Edge>,
    /// Scratch: per-node output values from the last tick (for previews —
    /// "good preview of intermediate results" is a first-class feature).
    last_out: Vec<Vec<Value>>,
}

impl Graph {
    /// Empty patch.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a block; returns its handle.
    pub fn add(&mut self, block: Box<dyn Block>) -> NodeId {
        self.nodes.push(Some(block));
        self.last_out.push(Vec::new());
        NodeId(self.nodes.len() - 1)
    }

    /// Remove a node and every cable touching it.
    pub fn remove(&mut self, id: NodeId) {
        if let Some(slot) = self.nodes.get_mut(id.0) {
            *slot = None;
            self.edges.retain(|e| e.from.0 != id && e.to.0 != id);
        }
    }

    /// Patch `from` (node, output port) into `to` (node, input port).
    /// Typed: incompatible cables are refused; convertible ones get their
    /// adapter recorded (the patchbay renders it as an inline bead).
    ///
    /// # Errors
    /// [`ConnectError`] with the reason — the UI shows it verbatim.
    pub fn connect(
        &mut self,
        from: (NodeId, usize),
        to: (NodeId, usize),
    ) -> Result<Option<AdapterKind>, ConnectError> {
        let from_ty = self
            .port_ty(from.0, from.1, false)
            .ok_or(ConnectError::BadPort)?;
        let to_ty = self
            .port_ty(to.0, to.1, true)
            .ok_or(ConnectError::BadPort)?;
        let adapter = match compat(from_ty, to_ty) {
            Compat::Direct => None,
            Compat::Adapter(k) => Some(k),
            Compat::Never => {
                return Err(ConnectError::TypeMismatch {
                    from: from_ty,
                    to: to_ty,
                })
            }
        };
        // an input takes exactly one cable: replacing is the gesture
        self.edges.retain(|e| e.to != to);
        self.edges.push(Edge { from, to, adapter });
        if self.topo_order().is_none() {
            // undo: the cable closed a loop
            self.edges.pop();
            return Err(ConnectError::Cycle);
        }
        Ok(adapter)
    }

    /// Unplug whatever feeds `to`.
    pub fn disconnect(&mut self, to: (NodeId, usize)) {
        self.edges.retain(|e| e.to != to);
    }

    fn port_ty(&self, id: NodeId, port: usize, input: bool) -> Option<PortType> {
        let node = self.nodes.get(id.0)?.as_ref()?;
        let spec = if input {
            node.meta().inputs.get(port)
        } else {
            node.meta().outputs.get(port)
        };
        Some(spec?.ty)
    }

    /// Kahn's algorithm over live nodes; `None` if a cycle exists.
    fn topo_order(&self) -> Option<Vec<usize>> {
        let n = self.nodes.len();
        let mut indeg = vec![0usize; n];
        for e in &self.edges {
            indeg[e.to.0 .0] += 1;
        }
        let mut queue: Vec<usize> = (0..n)
            .filter(|&i| self.nodes[i].is_some() && indeg[i] == 0)
            .collect();
        let mut order = Vec::with_capacity(n);
        while let Some(i) = queue.pop() {
            order.push(i);
            for e in &self.edges {
                if e.from.0 .0 == i {
                    indeg[e.to.0 .0] -= 1;
                    if indeg[e.to.0 .0] == 0 {
                        queue.push(e.to.0 .0);
                    }
                }
            }
        }
        (order.len() == self.nodes.iter().flatten().count()).then_some(order)
    }

    /// One tick: evaluate every block in dependency order. Sinks write to
    /// `ctx.board`; intermediate outputs are retained for previews.
    pub fn tick(&mut self, ctx: &Ctx) {
        let Some(order) = self.topo_order() else {
            return;
        };
        for i in order {
            let meta = match &self.nodes[i] {
                Some(b) => b.meta(),
                None => continue,
            };
            // gather inputs from upstream last_out (same tick: topo order
            // guarantees they're fresh)
            let inputs: Vec<Value> = (0..meta.inputs.len())
                .map(|port| {
                    self.edges
                        .iter()
                        .find(|e| e.to == (NodeId(i), port))
                        .and_then(|e| {
                            let raw = *self.last_out[e.from.0 .0].get(e.from.1)?;
                            Some(e.adapter.map_or(raw, |k| adapt(k, raw)))
                        })
                        .unwrap_or(Value::default_for(meta.inputs[port].ty))
                })
                .collect();
            let mut out = core::mem::take(&mut self.last_out[i]);
            out.clear();
            if let Some(block) = self.nodes[i].as_mut() {
                block.eval(ctx, &inputs, &mut out);
            }
            self.last_out[i] = out;
        }
    }

    /// Last-tick output of a port (the preview API).
    #[must_use]
    pub fn preview(&self, id: NodeId, port: usize) -> Option<Value> {
        self.last_out.get(id.0)?.get(port).copied()
    }

    /// Mutable access to a live block (the patchbay's settings surface).
    pub fn block_mut(&mut self, id: NodeId) -> Option<&mut Box<dyn Block>> {
        self.nodes.get_mut(id.0)?.as_mut()
    }

    /// Read access to a live block.
    #[must_use]
    pub fn block(&self, id: NodeId) -> Option<&dyn Block> {
        self.nodes.get(id.0)?.as_deref()
    }

    /// Every cable, for rendering: (from, to, adapter).
    pub fn cables(
        &self,
    ) -> impl Iterator<Item = ((NodeId, usize), (NodeId, usize), Option<AdapterKind>)> + '_ {
        self.edges.iter().map(|e| (e.from, e.to, e.adapter))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library;
    use crate::value::PortType;

    #[test]
    fn lfo_through_scale_reaches_the_board() {
        let mut g = Graph::new();
        let lfo = g.add(library::build("lfo").unwrap());
        let scale = g.add(library::build("scale").unwrap());
        let sink = g.add(Box::new(library::ParamOut {
            key: "citadel.warp",
        }));
        g.connect((lfo, 0), (scale, 0)).unwrap();
        g.connect((scale, 0), (sink, 0)).unwrap();
        let ctx = Ctx {
            time: 0.25,
            ..Ctx::default()
        };
        g.tick(&ctx);
        let board = ctx.board.borrow();
        assert_eq!(board.len(), 1);
        assert_eq!(board[0].0, "citadel.warp");
        // scale's mul input is unconnected → default 0 → output must be 0
        assert!(board[0].1.abs() < 1e-6);
    }

    #[test]
    fn incompatible_cables_are_refused_with_the_reason() {
        let mut g = Graph::new();
        let cam = g.add(library::build("camera-in").unwrap());
        let scale = g.add(library::build("scale").unwrap());
        let err = g.connect((cam, 0), (scale, 0)).unwrap_err();
        assert_eq!(
            err,
            ConnectError::TypeMismatch {
                from: PortType::Field,
                to: PortType::Signal
            }
        );
    }

    #[test]
    fn convertible_cables_get_a_visible_adapter() {
        let mut g = Graph::new();
        let beat = g.add(library::build("beat").unwrap());
        let scale = g.add(library::build("scale").unwrap());
        // gate → signal: allowed, with an adapter bead
        let adapter = g.connect((beat, 0), (scale, 0)).unwrap();
        assert_eq!(adapter, Some(crate::value::AdapterKind::GateToSignal));
    }

    #[test]
    fn cycles_are_refused() {
        let mut g = Graph::new();
        let a = g.add(library::build("scale").unwrap());
        let b = g.add(library::build("scale").unwrap());
        g.connect((a, 0), (b, 0)).unwrap();
        assert_eq!(g.connect((b, 0), (a, 0)).unwrap_err(), ConnectError::Cycle);
    }

    #[test]
    fn knob_feeds_expr_which_is_reprogrammable() {
        let mut g = Graph::new();
        let knob = g.add(library::build("knob").unwrap());
        let expr = g.add(library::build("expr").unwrap());
        g.connect((knob, 0), (expr, 0)).unwrap();
        g.block_mut(knob).unwrap().set_param(0.8);
        g.block_mut(expr).unwrap().set_code("a * 2").unwrap();
        g.tick(&Ctx::default());
        let Some(crate::value::Value::Signal(v)) = g.preview(expr, 0) else {
            panic!("expr must output a signal");
        };
        assert!((v - 1.6).abs() < 1e-6, "knob 0.8 through a*2, got {v}");
        // bad code is refused with an addressed reason, program unchanged
        let err = g.block_mut(expr).unwrap().set_code("a +").unwrap_err();
        assert!(err.contains("char"), "{err}");
        g.tick(&Ctx::default());
        assert!(matches!(
            g.preview(expr, 0),
            Some(crate::value::Value::Signal(_))
        ));
    }

    #[test]
    fn param_out_key_is_repointable() {
        let mut g = Graph::new();
        let sink = g.add(library::build("param-out").unwrap());
        assert_eq!(g.block(sink).unwrap().key(), Some("mind.warp"));
        g.block_mut(sink).unwrap().set_key("mind.hue");
        assert_eq!(g.block(sink).unwrap().key(), Some("mind.hue"));
    }

    #[test]
    fn previews_expose_intermediate_results() {
        let mut g = Graph::new();
        let lfo = g.add(library::build("lfo").unwrap());
        let ctx = Ctx {
            time: 1.0,
            ..Ctx::default()
        };
        g.tick(&ctx);
        assert!(matches!(g.preview(lfo, 0), Some(Value::Signal(_))));
    }
}
