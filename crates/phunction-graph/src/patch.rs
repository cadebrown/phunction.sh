//! patch — the graph as text. Cables in code, code into cables.
//!
//! One statement per line:
//!
//! ```text
//! # a comment
//! k = knob 0.8                 # a node; knobs take a bare value
//! l = lfo rate=k depth=0.6     # inputs by name: node refs or literals
//! e = expr "a*2 + sin(t*0.2)" a=l
//! e -> mind.warp               # route a node's first output to a board key
//! b = beat
//! m = mix a=l b=e t=b.phase    # multi-output refs: name.portname
//! ```
//!
//! Literal inputs become anonymous knob nodes — nothing is privileged, the
//! text just writes the same graph your hands would. `compile` validates
//! statically (kinds, port names, types) with **line-addressed** errors;
//! applying the plan runs through [`Graph::connect`], so cycles and type
//! rules hold no matter which door you came in.

use crate::graph::{Graph, NodeId};
use crate::library;
use crate::value::{compat, Compat};

/// A compile failure, addressed to its line (1-based).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchError {
    /// 1-based source line.
    pub line: usize,
    /// The reason, in words.
    pub msg: String,
}

impl core::fmt::Display for PatchError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "line {}: {}", self.line, self.msg)
    }
}

/// One planned node.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanNode {
    /// Patch-scope name (anonymous literals get `_litN`).
    pub name: String,
    /// Library kind id.
    pub kind: &'static str,
    /// Knob value, if given.
    pub param: Option<f32>,
    /// Expr source, if given.
    pub code: Option<String>,
}

/// One planned cable: (source node index, out port) → (dest node index, in port).
pub type PlanLink = ((usize, usize), (usize, usize));

/// A validated patch, ready to build a [`Graph`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Plan {
    /// Nodes in declaration order.
    pub nodes: Vec<PlanNode>,
    /// Cables.
    pub links: Vec<PlanLink>,
    /// Board routes: (source node index, out port, board key).
    pub routes: Vec<(usize, usize, &'static str)>,
}

/// Compile patch text against the block library and the host's board keys.
///
/// # Errors
/// The first problem found, addressed to its line.
#[allow(clippy::too_many_lines)]
pub fn compile(src: &str, keys: &[&'static str]) -> Result<Plan, PatchError> {
    let mut plan = Plan::default();
    let mut names: Vec<String> = Vec::new();
    let mut lit_count = 0usize;

    let err = |line: usize, msg: String| PatchError { line, msg };
    let find_meta = |kind: &str| library::SHELF.iter().find(|m| m.id == kind).copied();

    for (ix, raw) in src.lines().enumerate() {
        let line_no = ix + 1;
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }

        if let Some((lhs, rhs)) = line.split_once("->") {
            // route: `name -> board.key`. Signals go to parameter keys;
            // a Field goes to the room itself via `mind.field`.
            let name = lhs.trim();
            let key_txt = rhs.trim();
            let (src_ix, out_port) = resolve_ref(name, &names, &plan, line_no)?;
            let from_ty = node_meta(&plan.nodes[src_ix]).outputs[out_port].ty;
            if key_txt == "mind.field" {
                if from_ty != crate::value::PortType::Field {
                    return Err(err(
                        line_no,
                        format!("mind.field takes a Field, not {from_ty:?}"),
                    ));
                }
                plan.routes.push((src_ix, out_port, "mind.field"));
                continue;
            }
            let Some(key) = keys.iter().find(|k| **k == key_txt) else {
                return Err(err(
                    line_no,
                    format!(
                        "unknown key `{key_txt}` (have: {}, mind.field)",
                        keys.join(", ")
                    ),
                ));
            };
            plan.routes.push((src_ix, out_port, key));
            continue;
        }

        let Some((name, def)) = line.split_once('=') else {
            return Err(err(
                line_no,
                format!("expected `name = kind …` or `name -> key`, got `{line}`"),
            ));
        };
        let name = name.trim();
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(err(line_no, format!("bad name `{name}`")));
        }
        if names.iter().any(|n| n == name) {
            return Err(err(line_no, format!("`{name}` is already defined")));
        }

        // tokenize the definition, honoring quoted expr code
        let (code, rest) = extract_quoted(def.trim());
        let mut toks = rest.split_whitespace();
        let Some(kind_txt) = toks.next() else {
            return Err(err(line_no, "missing block kind".into()));
        };
        let Some(meta) = find_meta(kind_txt) else {
            let kinds: Vec<&str> = library::SHELF.iter().map(|m| m.id).collect();
            return Err(err(
                line_no,
                format!("unknown block `{kind_txt}` (have: {})", kinds.join(", ")),
            ));
        };
        let kind = meta.id;

        let mut param = None;
        // dst index is patched after the node lands: literal knobs are
        // appended while args parse, so the node's slot moves under us
        let mut pending_links: Vec<PlanLink> = Vec::new();
        for tok in toks {
            if let Some((port_txt, val_txt)) = tok.split_once('=') {
                let Some(port) = meta.inputs.iter().position(|p| p.name == port_txt) else {
                    let have: Vec<&str> = meta.inputs.iter().map(|p| p.name).collect();
                    return Err(err(
                        line_no,
                        format!(
                            "`{kind}` has no input `{port_txt}` (have: {})",
                            have.join(", ")
                        ),
                    ));
                };
                let (src_ix, out_port) = if let Ok(v) = val_txt.parse::<f32>() {
                    // literal: an anonymous knob
                    lit_count += 1;
                    let lit_name = format!("_lit{lit_count}");
                    plan.nodes.push(PlanNode {
                        name: lit_name.clone(),
                        kind: "knob",
                        param: Some(v),
                        code: None,
                    });
                    names.push(lit_name);
                    (plan.nodes.len() - 1, 0)
                } else {
                    resolve_ref(val_txt, &names, &plan, line_no)?
                };
                // static type check so errors land on this line, not at apply
                let from_ty = node_meta(&plan.nodes[src_ix]).outputs[out_port].ty;
                let to_ty = meta.inputs[port].ty;
                if matches!(compat(from_ty, to_ty), Compat::Never) {
                    return Err(err(
                        line_no,
                        format!("{from_ty:?} does not speak {to_ty:?} (input `{port_txt}`)"),
                    ));
                }
                pending_links.push(((src_ix, out_port), (usize::MAX, port)));
            } else if tok.parse::<f32>().is_ok() && kind == "knob" {
                param = tok.parse().ok();
            } else {
                return Err(err(line_no, format!("stray token `{tok}`")));
            }
        }
        if kind == "expr" && code.is_none() {
            return Err(err(line_no, "expr needs its program in quotes".into()));
        }
        if let Some(c) = &code {
            if kind != "expr" {
                return Err(err(
                    line_no,
                    format!("`{kind}` does not take code (`\"{c}\"`)"),
                ));
            }
            if let Err(e) = crate::expr::parse(c, &library::ExprBlock::VARS) {
                return Err(err(
                    line_no,
                    format!("expr: at char {}: {}", e.pos + 1, e.msg),
                ));
            }
        }
        plan.nodes.push(PlanNode {
            name: name.to_string(),
            kind,
            param,
            code,
        });
        names.push(name.to_string());
        let this_ix = plan.nodes.len() - 1;
        for ((s_ix, s_port), (_, d_port)) in pending_links.drain(..) {
            plan.links.push(((s_ix, s_port), (this_ix, d_port)));
        }
    }
    Ok(plan)
}

/// Meta for a planned node (kind always exists — compile checked).
fn node_meta(node: &PlanNode) -> &'static library::BlockMeta {
    library::SHELF
        .iter()
        .find(|m| m.id == node.kind)
        .copied()
        .expect("plan kinds come from the shelf")
}

/// Resolve `name` or `name.portname` to (node index, out port).
fn resolve_ref(
    txt: &str,
    names: &[String],
    plan: &Plan,
    line: usize,
) -> Result<(usize, usize), PatchError> {
    let (name, port_txt) = txt
        .split_once('.')
        .map_or((txt, None), |(n, p)| (n, Some(p)));
    let Some(ix) = names.iter().position(|n| n == name) else {
        return Err(PatchError {
            line,
            msg: format!("unknown node `{name}`"),
        });
    };
    let meta = node_meta(&plan.nodes[ix]);
    let port = match port_txt {
        None => 0,
        Some(p) => meta
            .outputs
            .iter()
            .position(|o| o.name == p)
            .ok_or_else(|| {
                let have: Vec<&str> = meta.outputs.iter().map(|o| o.name).collect();
                PatchError {
                    line,
                    msg: format!("`{name}` has no output `{p}` (have: {})", have.join(", ")),
                }
            })?,
    };
    if meta.outputs.is_empty() {
        return Err(PatchError {
            line,
            msg: format!("`{name}` has no outputs to patch from"),
        });
    }
    Ok((ix, port))
}

/// Pull one quoted string out of a definition, returning (code, rest).
fn extract_quoted(s: &str) -> (Option<String>, String) {
    let Some(start) = s.find('"') else {
        return (None, s.to_string());
    };
    let Some(len) = s[start + 1..].find('"') else {
        return (None, s.to_string());
    };
    let code = s[start + 1..start + 1 + len].to_string();
    let mut rest = String::with_capacity(s.len());
    rest.push_str(&s[..start]);
    rest.push(' ');
    rest.push_str(&s[start + len + 2..]);
    (Some(code), rest)
}

/// Build a fresh graph from a plan. Returns the graph plus, per plan node,
/// its [`NodeId`] (for the UI's layout table). Connection failures can only
/// be cycles — types were compile-checked — and abort with a message.
///
/// # Errors
/// A human-readable reason (the offending cable's endpoints).
pub fn build(plan: &Plan) -> Result<(Graph, Vec<NodeId>), String> {
    let mut g = Graph::new();
    let mut ids = Vec::with_capacity(plan.nodes.len());
    for node in &plan.nodes {
        let mut block =
            library::build(node.kind).ok_or_else(|| format!("no block `{}`", node.kind))?;
        if let Some(v) = node.param {
            block.set_param(v);
        }
        if let Some(c) = &node.code {
            block
                .set_code(c)
                .map_err(|e| format!("`{}`: {e}", node.name))?;
        }
        ids.push(g.add(block));
    }
    for ((si, sp), (di, dp)) in &plan.links {
        g.connect((ids[*si], *sp), (ids[*di], *dp)).map_err(|e| {
            format!(
                "cable {} → {}: {e:?}",
                plan.nodes[*si].name, plan.nodes[*di].name
            )
        })?;
    }
    for (si, sp, key) in &plan.routes {
        let sink = if *key == "mind.field" {
            g.add(Box::new(library::FieldOut))
        } else {
            g.add(Box::new(library::ParamOut { key }))
        };
        ids.push(sink);
        g.connect((ids[*si], *sp), (sink, 0))
            .map_err(|e| format!("route {} -> {key}: {e:?}", plan.nodes[*si].name))?;
    }
    Ok((g, ids))
}

/// Render a graph description back to patch text — the inverse door.
/// `nodes` is (id, kind) in display order; sinks become `->` lines.
#[must_use]
pub fn render(graph: &Graph, nodes: &[(NodeId, String)]) -> String {
    use core::fmt::Write;
    // stable names: kind + ordinal
    let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    let mut names: std::collections::HashMap<usize, String> = std::collections::HashMap::new();
    for (id, kind) in nodes {
        if kind == "param-out" || kind == "field-out" {
            continue;
        }
        let n = counts.entry(kind.as_str()).or_insert(0);
        *n += 1;
        names.insert(id.0, format!("{}{}", kind.replace('-', "_"), n));
    }
    let feeder = |id: NodeId, port: usize| -> Option<String> {
        graph
            .cables()
            .find(|(_, to, _)| *to == (id, port))
            .map(|(from, _, _)| {
                let name = names.get(&from.0 .0).cloned().unwrap_or_else(|| "?".into());
                let meta = graph.block(from.0).map(crate::library::Block::meta);
                let port_name = meta.and_then(|m| m.outputs.get(from.1)).map(|p| p.name);
                match (from.1, port_name) {
                    (0, _) => name,
                    (_, Some(p)) => format!("{name}.{p}"),
                    _ => name,
                }
            })
    };
    let mut out = String::new();
    for (id, kind) in nodes {
        if kind == "param-out" {
            continue;
        }
        let Some(block) = graph.block(*id) else {
            continue;
        };
        let name = &names[&id.0];
        let _ = write!(out, "{name} = {kind}");
        if let Some(v) = block.param() {
            let _ = write!(out, " {v:.2}");
        }
        if let Some(c) = block.code() {
            let _ = write!(out, " \"{c}\"");
        }
        for (port, spec) in block.meta().inputs.iter().enumerate() {
            if let Some(src) = feeder(*id, port) {
                let _ = write!(out, " {}={src}", spec.name);
            }
        }
        out.push('\n');
    }
    for (id, kind) in nodes {
        if kind != "param-out" {
            continue;
        }
        let Some(block) = graph.block(*id) else {
            continue;
        };
        if let (Some(key), Some(src)) = (block.key(), feeder(*id, 0)) {
            let _ = writeln!(out, "{src} -> {key}");
        }
    }
    for (id, kind) in nodes {
        if kind != "field-out" {
            continue;
        }
        if let Some(src) = feeder(*id, 0) {
            let _ = writeln!(out, "{src} -> mind.field");
        }
    }
    out
}

/// The shipped patch library — whole-patches, worlds for the graph.
/// Every entry is compile-tested; the UI installs them by name.
pub static LIBRARY: &[(&str, &str)] = &[
    (
        "breath",
        "k = knob 0.25\nl = lfo rate=k depth=0.4\ns = slew in=l amount=0.85\ns -> mind.warp\nl2 = lfo rate=0.1 depth=0.3\nl2 -> mind.hue",
    ),
    (
        "pulse",
        "b = beat\ng = slew in=b amount=0.92\ng -> mind.scale\ne = expr \"0.3*tri(b*0.25)\" b=b.phase\ne -> mind.dolly",
    ),
    (
        "listener",
        "a = audio-in\ns = slew in=a amount=0.88\ns -> mind.warp\ne = expr \"a*0.5 + 0.15*sin(t*0.07)\" a=a\ne -> mind.hue\na2 = audio-in\na2 -> fx.wash",
    ),
    (
        "open mic",
        "m = mic-in\ns = slew in=m amount=0.9\ns -> mind.warp\ne = expr \"a*0.8\" a=m\ne -> fx.echo",
    ),
    (
        "co-pilot",
        "p = pads\np -> mind.hue\ne = expr \"abs(a)\" a=p.y\ne -> mind.dolly\ne2 = expr \"a*0.7\" a=p.trig\ne2 -> mind.warp",
    ),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::Ctx;

    const KEYS: &[&str] = &["mind.warp", "mind.hue"];

    #[test]
    fn a_patch_compiles_builds_and_sounds() {
        let src = "
# the starter weather
k = knob 0.8
l = lfo rate=k depth=0.5
e = expr \"a*0.5 + 0.25\" a=l
e -> mind.warp
";
        let plan = compile(src, KEYS).unwrap();
        assert_eq!(plan.routes.len(), 1);
        let (mut g, _ids) = build(&plan).unwrap();
        let ctx = Ctx {
            time: 1.0,
            ..Ctx::default()
        };
        g.tick(&ctx);
        let board = ctx.board.borrow();
        assert_eq!(board.len(), 1);
        assert_eq!(board[0].0, "mind.warp");
        let crate::value::Value::Signal(v) = board[0].1 else {
            panic!("routes write signals");
        };
        assert!(v.is_finite());
    }

    #[test]
    fn literals_become_knobs() {
        let plan = compile("l = lfo rate=0.3", KEYS).unwrap();
        assert_eq!(plan.nodes.len(), 2);
        assert_eq!(plan.nodes[0].kind, "knob");
        assert_eq!(plan.nodes[0].param, Some(0.3));
        assert_eq!(plan.links.len(), 1);
    }

    #[test]
    fn errors_are_line_addressed() {
        let e = compile("k = knob 0.5\nx = warble", KEYS).unwrap_err();
        assert_eq!(e.line, 2);
        assert!(e.msg.contains("warble"));

        let e = compile("k = knob 0.5\nk -> mind.wat", KEYS).unwrap_err();
        assert_eq!(e.line, 2);
        assert!(e.msg.contains("mind.wat"));

        let e = compile("c = camera-in\ns = scale in=c", KEYS).unwrap_err();
        assert_eq!(e.line, 2);
        assert!(e.msg.contains("does not speak"));

        let e = compile("e = expr \"a +\"", KEYS).unwrap_err();
        assert_eq!(e.line, 1);
        assert!(e.msg.contains("char"));
    }

    #[test]
    fn multi_output_refs_use_port_names() {
        let plan = compile("b = beat\ns = scale in=b.phase", KEYS).unwrap();
        assert_eq!(plan.links[0].0, (0, 1), "phase is beat's second output");
    }

    #[test]
    fn the_shipped_library_compiles_and_builds() {
        let keys = [
            "mind.scale",
            "mind.warp",
            "mind.hue",
            "mind.dolly",
            "voice.cutoff",
            "fx.echo",
            "fx.wash",
        ];
        for (name, text) in LIBRARY {
            let plan =
                compile(text, &keys).unwrap_or_else(|e| panic!("library patch `{name}`: {e}"));
            build(&plan).unwrap_or_else(|e| panic!("library patch `{name}` build: {e}"));
        }
    }

    #[test]
    fn field_routes_compile_and_reach_the_board() {
        let plan = compile("cam = camera-in\ncam -> mind.field", KEYS).unwrap();
        let (mut g, _ids) = build(&plan).unwrap();
        let ctx = Ctx {
            camera: crate::value::FieldId(1),
            ..Ctx::default()
        };
        g.tick(&ctx);
        let board = ctx.board.borrow();
        assert_eq!(board.len(), 1);
        assert_eq!(board[0].0, "mind.field");
        assert!(matches!(board[0].1, crate::value::Value::Field(f) if f.0 == 1));

        let e = compile("k = knob 0.5\nk -> mind.field", KEYS).unwrap_err();
        assert!(e.msg.contains("takes a Field"), "{}", e.msg);
    }

    #[test]
    fn round_trip_survives() {
        let src = "k = knob 0.75\nl = lfo rate=k\nl -> mind.hue\n";
        let plan = compile(src, KEYS).unwrap();
        let (g, ids) = build(&plan).unwrap();
        let mut listed: Vec<(crate::graph::NodeId, String)> = plan
            .nodes
            .iter()
            .zip(&ids)
            .map(|(n, id)| (*id, n.kind.to_string()))
            .collect();
        // build() appends route sinks after plan nodes
        for id in &ids[plan.nodes.len()..] {
            listed.push((*id, "param-out".to_string()));
        }
        let text = render(&g, &listed);
        let plan2 = compile(&text, KEYS).unwrap();
        let (g2, _) = build(&plan2).unwrap();
        let ctx = Ctx {
            time: 2.0,
            ..Ctx::default()
        };
        let mut g = g;
        let mut g2 = g2;
        g.tick(&ctx);
        let first = ctx.board.borrow().clone();
        ctx.board.borrow_mut().clear();
        g2.tick(&ctx);
        let second = ctx.board.borrow().clone();
        assert_eq!(first, second, "text → graph → text → graph is identity");
    }
}
