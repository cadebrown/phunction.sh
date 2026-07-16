//! The block contract and the prebuilt library — the shipped "effect
//! racks" (Cade's directive). Nothing here is privileged: user-built
//! blocks implement the same trait.

use crate::graph::Ctx;
use crate::value::{PortType, Value};

/// One port on a block.
#[derive(Debug, Clone, Copy)]
pub struct PortSpec {
    /// Engraved port name.
    pub name: &'static str,
    /// Wire type.
    pub ty: PortType,
}

/// Static metadata for a block (the patchbay's catalogue entry).
#[derive(Debug, Clone, Copy)]
pub struct BlockMeta {
    /// Library id (`lfo`, `scale`, `camera-in`, …).
    pub id: &'static str,
    /// Display name.
    pub name: &'static str,
    /// Library shelf (`source` / `math` / `adapter` / `media` / `sink`).
    pub category: &'static str,
    /// Input ports.
    pub inputs: &'static [PortSpec],
    /// Output ports.
    pub outputs: &'static [PortSpec],
}

/// A live block instance: metadata plus per-instance state and an eval.
///
/// The `set_*`/getter pairs are the patchbay's settings surface: every
/// capability is opt-in with a no-op default, so plain DSP blocks stay
/// two-method simple while programmable blocks expose their innards.
pub trait Block {
    /// Catalogue entry.
    fn meta(&self) -> &'static BlockMeta;
    /// Produce one tick of outputs from inputs. `inputs` and `outputs`
    /// lengths always match `meta()`; the graph guarantees types.
    fn eval(&mut self, ctx: &Ctx, inputs: &[Value], outputs: &mut Vec<Value>);
    /// A directly-settable scalar (knob blocks), if this block has one.
    fn param(&self) -> Option<f32> {
        None
    }
    /// Set the scalar. No-op unless [`Block::param`] is `Some`.
    fn set_param(&mut self, _value: f32) {}
    /// Editable source code (expr blocks), if this block is programmable.
    fn code(&self) -> Option<String> {
        None
    }
    /// Replace the code. `Err(message)` shows verbatim in the patchbay.
    ///
    /// # Errors
    /// A human-readable reason (parse errors are byte-addressed).
    fn set_code(&mut self, _source: &str) -> Result<(), String> {
        Err("this block is not programmable".into())
    }
    /// The output-board key (param-out sinks), if this block routes one.
    fn key(&self) -> Option<&'static str> {
        None
    }
    /// Point the sink at a different board key.
    fn set_key(&mut self, _key: &'static str) {}
}

// ---- the prebuilt library ----

const SIG: PortType = PortType::Signal;

macro_rules! block {
    ($struct_:ident, $meta:ident, $id:literal, $name:literal, $cat:literal,
     ins: [$($iname:literal : $ity:expr),*], outs: [$($oname:literal : $oty:expr),*],
     |$self_:ident, $ctx:ident, $ins:ident, $outs:ident| $body:block) => {
        /// Library block (see `meta` fields for its contract).
        #[derive(Debug, Default)]
        pub struct $struct_ {
            /// Per-instance scratch (block-defined meaning).
            pub state: f32,
        }
        static $meta: BlockMeta = BlockMeta {
            id: $id,
            name: $name,
            category: $cat,
            inputs: &[$(PortSpec { name: $iname, ty: $ity }),*],
            outputs: &[$(PortSpec { name: $oname, ty: $oty }),*],
        };
        impl Block for $struct_ {
            fn meta(&self) -> &'static BlockMeta { &$meta }
            #[allow(unused_variables)]
            fn eval(&mut $self_, $ctx: &Ctx, $ins: &[Value], $outs: &mut Vec<Value>) $body
        }
    };
}

fn sig(v: &Value) -> f32 {
    match v {
        Value::Signal(s) | Value::Phase(s) => *s,
        Value::Gate(g) => f32::from(u8::from(*g)),
        _ => 0.0,
    }
}

block!(Lfo, LFO_META, "lfo", "lfo", "source",
ins: ["rate": SIG, "depth": SIG],
outs: ["out": SIG],
|self, ctx, ins, outs| {
    let rate = 0.05 + sig(&ins[0]) * 8.0;
    let depth = if matches!(ins[1], Value::Signal(_)) { sig(&ins[1]) } else { 1.0 };
    outs.push(Value::Signal(
        ((ctx.time * rate).sin() * 0.5 + 0.5) * depth,
    ));
});

block!(BeatClock, BEAT_META, "beat", "beat clock", "source",
ins: [],
outs: ["gate": PortType::Gate, "phase": PortType::Phase],
|self, ctx, ins, outs| {
    let frac = ctx.beats.fract();
    outs.push(Value::Gate(ctx.playing && frac < 0.1));
    #[allow(clippy::cast_possible_truncation)]
    outs.push(Value::Phase(frac as f32));
});

block!(AudioIn, AUDIO_IN_META, "audio-in", "audio in", "media",
ins: [],
outs: ["rms": SIG, "peak": SIG, "bus": PortType::Audio],
|self, ctx, ins, outs| {
    outs.push(Value::Signal(ctx.rms));
    outs.push(Value::Signal(ctx.peak));
    outs.push(Value::Audio(ctx.audio_bus));
});

block!(CameraIn, CAMERA_IN_META, "camera-in", "camera in", "media",
ins: [],
outs: ["field": PortType::Field],
|self, ctx, ins, outs| {
    // the runtime registers the live camera as field handle; 0 = absent
    outs.push(Value::Field(ctx.camera));
});

block!(MicIn, MIC_IN_META, "mic-in", "mic in", "media",
ins: [],
outs: ["level": SIG],
|self, ctx, ins, outs| {
    // ext[0] = live microphone level (the runtime requests permission on
    // first use; 0 until granted — idle must still feel alive upstream)
    outs.push(Value::Signal(ctx.ext[0]));
});

block!(Pads, PADS_META, "pads", "gamepad", "source",
ins: [],
outs: ["x": SIG, "y": SIG, "trig": SIG],
|self, ctx, ins, outs| {
    // ext[1..4] = first gamepad's left stick + trigger, polled per frame
    outs.push(Value::Signal(ctx.ext[1]));
    outs.push(Value::Signal(ctx.ext[2]));
    outs.push(Value::Signal(ctx.ext[3]));
});

block!(Scale, SCALE_META, "scale", "scale·offset", "math",
ins: ["in": SIG, "mul": SIG, "add": SIG],
outs: ["out": SIG],
|self, ctx, ins, outs| {
    outs.push(Value::Signal(sig(&ins[0]).mul_add(sig(&ins[1]), sig(&ins[2]))));
});

block!(Mix, MIX_META, "mix", "mix", "math",
ins: ["a": SIG, "b": SIG, "t": SIG],
outs: ["out": SIG],
|self, ctx, ins, outs| {
    let t = sig(&ins[2]).clamp(0.0, 1.0);
    outs.push(Value::Signal(sig(&ins[0]) * (1.0 - t) + sig(&ins[1]) * t));
});

block!(Slew, SLEW_META, "slew", "slew", "math",
ins: ["in": SIG, "amount": SIG],
outs: ["out": SIG],
|self, ctx, ins, outs| {
    let coeff = 1.0 - sig(&ins[1]).clamp(0.0, 0.995);
    self.state += (sig(&ins[0]) - self.state) * coeff;
    outs.push(Value::Signal(self.state));
});

block!(Split, SPLIT_META, "split", "vec2 split", "adapter",
ins: ["in": PortType::Vec2],
outs: ["x": SIG, "y": SIG],
|self, ctx, ins, outs| {
    let [x, y] = match ins[0] { Value::Vec2(v) => v, _ => [0.5, 0.5] };
    outs.push(Value::Signal(x));
    outs.push(Value::Signal(y));
});

/// A hand on the patch: no inputs, one signal out, set from the node UI.
#[derive(Debug)]
pub struct Knob {
    value: f32,
}
impl Default for Knob {
    fn default() -> Self {
        Self { value: 0.5 }
    }
}
static KNOB_META: BlockMeta = BlockMeta {
    id: "knob",
    name: "knob",
    category: "source",
    inputs: &[],
    outputs: &[PortSpec {
        name: "out",
        ty: SIG,
    }],
};
impl Block for Knob {
    fn meta(&self) -> &'static BlockMeta {
        &KNOB_META
    }
    fn eval(&mut self, _ctx: &Ctx, _inputs: &[Value], outputs: &mut Vec<Value>) {
        outputs.push(Value::Signal(self.value));
    }
    fn param(&self) -> Option<f32> {
        Some(self.value)
    }
    fn set_param(&mut self, value: f32) {
        self.value = value.clamp(0.0, 1.0);
    }
}

/// The programmable block: behavior is a program in the little language.
/// Inputs a/b/c arrive as variables next to t/beat/rms — new nodes from
/// code, exactly as VISION §III demands.
#[derive(Debug)]
pub struct ExprBlock {
    source: String,
    program: Option<crate::expr::Program>,
}
impl ExprBlock {
    /// Variables an expr node sees: its three patched inputs + the world.
    pub const VARS: [&'static str; 6] = ["a", "b", "c", "t", "beat", "rms"];

    /// A node with the shipped starter program.
    #[must_use]
    pub fn starter() -> Self {
        let source = "a + 0.3*sin(t*0.2)";
        Self {
            program: crate::expr::parse(source, &Self::VARS).ok(),
            source: source.into(),
        }
    }
}
static EXPR_META: BlockMeta = BlockMeta {
    id: "expr",
    name: "expr",
    category: "code",
    inputs: &[
        PortSpec { name: "a", ty: SIG },
        PortSpec { name: "b", ty: SIG },
        PortSpec { name: "c", ty: SIG },
    ],
    outputs: &[PortSpec {
        name: "out",
        ty: SIG,
    }],
};
impl Block for ExprBlock {
    fn meta(&self) -> &'static BlockMeta {
        &EXPR_META
    }
    fn eval(&mut self, ctx: &Ctx, inputs: &[Value], outputs: &mut Vec<Value>) {
        #[allow(clippy::cast_possible_truncation)]
        let env = [
            sig(&inputs[0]),
            sig(&inputs[1]),
            sig(&inputs[2]),
            ctx.time,
            ctx.beats as f32,
            ctx.rms,
        ];
        let v = self
            .program
            .as_ref()
            .map_or(0.0, |p| p.eval(&env).clamp(-4.0, 4.0));
        outputs.push(Value::Signal(v));
    }
    fn code(&self) -> Option<String> {
        Some(self.source.clone())
    }
    fn set_code(&mut self, source: &str) -> Result<(), String> {
        match crate::expr::parse(source, &Self::VARS) {
            Ok(p) => {
                self.program = Some(p);
                self.source = source.into();
                Ok(())
            }
            Err(e) => Err(format!("at char {}: {}", e.pos + 1, e.msg)),
        }
    }
}

/// A parameter sink: writes its input into the ctx output board under a
/// fixed key ("citadel.warp", "voice.cutoff", …). The runtime reads the
/// board after each tick and forwards to engines.
#[derive(Debug)]
pub struct ParamOut {
    /// Board key this sink writes.
    pub key: &'static str,
}
static PARAM_OUT_META: BlockMeta = BlockMeta {
    id: "param-out",
    name: "param out",
    category: "sink",
    inputs: &[PortSpec {
        name: "in",
        ty: SIG,
    }],
    outputs: &[],
};
impl Block for ParamOut {
    fn meta(&self) -> &'static BlockMeta {
        &PARAM_OUT_META
    }
    fn eval(&mut self, ctx: &Ctx, inputs: &[Value], _outputs: &mut Vec<Value>) {
        ctx.board.borrow_mut().push((self.key, sig(&inputs[0])));
    }
    fn key(&self) -> Option<&'static str> {
        Some(self.key)
    }
    fn set_key(&mut self, key: &'static str) {
        self.key = key;
    }
}

/// Construct a library block by id (the patchbay's "add node" menu).
#[must_use]
pub fn build(id: &str) -> Option<Box<dyn Block>> {
    Some(match id {
        "lfo" => Box::new(Lfo::default()),
        "beat" => Box::new(BeatClock::default()),
        "audio-in" => Box::new(AudioIn::default()),
        "camera-in" => Box::new(CameraIn::default()),
        "mic-in" => Box::new(MicIn::default()),
        "pads" => Box::new(Pads::default()),
        "scale" => Box::new(Scale::default()),
        "mix" => Box::new(Mix::default()),
        "slew" => Box::new(Slew::default()),
        "split" => Box::new(Split::default()),
        "knob" => Box::new(Knob::default()),
        "expr" => Box::new(ExprBlock::starter()),
        "param-out" => Box::new(ParamOut { key: "mind.warp" }),
        _ => return None,
    })
}

/// The shelf list for catalogue UIs.
pub static SHELF: &[&BlockMeta] = &[
    &KNOB_META,
    &LFO_META,
    &BEAT_META,
    &AUDIO_IN_META,
    &CAMERA_IN_META,
    &MIC_IN_META,
    &PADS_META,
    &EXPR_META,
    &SCALE_META,
    &MIX_META,
    &SLEW_META,
    &SPLIT_META,
    &PARAM_OUT_META,
];
