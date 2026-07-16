//! # phunction-graph
//!
//! The typed dataflow core of the phunction workstation (VISION §III, and
//! Cade's directive: *get the typing and interconnects right early*).
//!
//! ## Design invariants
//!
//! 1. **Wires are typed.** Every port carries a [`PortType`]; every type
//!    wears a station hue and a glyph, so the patchbay can color cables by
//!    what flows through them. Connections are validated: [`compat`]
//!    answers `Direct`, `Adapter(kind)` (an auto-insertable conversion
//!    block), or `Never`.
//! 2. **Blocks are constructive.** A block is data (name, ports, category)
//!    plus an eval function; the prebuilt [`library`] is just blocks that
//!    ship with the site. User-constructed blocks (expression nodes, WGSL
//!    field nodes) implement the same [`Block`] trait — nothing about the
//!    graph privileges the built-ins.
//! 3. **Media are handles.** Audio buses, GPU fields (textures), and
//!    geometry flow through the graph as opaque ids; the graph routes
//!    them, the runtime (audio engine, wgpu) owns them. This keeps the
//!    core target-agnostic and natively testable.
//! 4. **Acyclic, deterministic.** Cycles are rejected at connect time;
//!    evaluation is a topological sweep per tick with a [`Ctx`] snapshot
//!    (time, beats, audio telemetry, live-input handles).

pub mod expr;
pub mod graph;
pub mod library;
pub mod patch;
pub mod value;

pub use graph::{ConnectError, Ctx, Graph, NodeId};
pub use library::{Block, BlockMeta, PortSpec};
pub use value::{compat, AdapterKind, Compat, PortType, Value};
