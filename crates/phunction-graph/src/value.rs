//! Port types, wire values, and the conversion rules between them.

/// Opaque handle to a GPU field (texture) owned by the gfx runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FieldId(pub u32);
/// Opaque handle to an audio bus owned by the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AudioId(pub u32);
/// Opaque handle to geometry owned by the gfx runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GeoId(pub u32);

/// What a wire carries. The set is deliberately rich (Cade's directive) —
/// but every member earns its place by having real producers and consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PortType {
    /// Continuous control value, nominally `0..=1` (CV).
    Signal,
    /// Boolean event level (triggers, mutes).
    Gate,
    /// Wrapped angle `0..=1` ≙ `0..τ` (the house currency).
    Phase,
    /// Two signals moving together (XY pads, positions).
    Vec2,
    /// A color as a phase-wheel hue in degrees.
    Color,
    /// An audio bus (handle).
    Audio,
    /// A GPU field / texture (handle) — video, shader output, camera.
    Field,
    /// Geometry (handle) — meshes, point sets.
    Geometry,
}

impl PortType {
    /// Station hue for cables/ports of this type (canon: every hue states
    /// its angle).
    #[must_use]
    pub fn hue(self) -> f32 {
        match self {
            Self::Signal => 100.0,   // phosphor — control voltage
            Self::Gate => 10.0,      // blood — events hit
            Self::Phase => 55.0,     // rust — the wheel itself
            Self::Vec2 => 190.0,     // verdigris
            Self::Color => 145.0,    // ichor
            Self::Audio => 325.0,    // philtre — hot signal
            Self::Field => 235.0,    // aether — light
            Self::Geometry => 280.0, // sigil — structure
        }
    }

    /// Port glyph for compact node rendering.
    #[must_use]
    pub fn glyph(self) -> &'static str {
        match self {
            Self::Signal => "∿",
            Self::Gate => "⊓",
            Self::Phase => "φ",
            Self::Vec2 => "⇉",
            Self::Color => "◉",
            Self::Audio => "♪",
            Self::Field => "▦",
            Self::Geometry => "◬",
        }
    }
}

/// A value on a wire.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value {
    /// See [`PortType::Signal`].
    Signal(f32),
    /// See [`PortType::Gate`].
    Gate(bool),
    /// See [`PortType::Phase`].
    Phase(f32),
    /// See [`PortType::Vec2`].
    Vec2([f32; 2]),
    /// See [`PortType::Color`].
    Color(f32),
    /// See [`PortType::Audio`].
    Audio(AudioId),
    /// See [`PortType::Field`].
    Field(FieldId),
    /// See [`PortType::Geometry`].
    Geometry(GeoId),
}

impl Value {
    /// The type this value inhabits.
    #[must_use]
    pub fn ty(&self) -> PortType {
        match self {
            Self::Signal(_) => PortType::Signal,
            Self::Gate(_) => PortType::Gate,
            Self::Phase(_) => PortType::Phase,
            Self::Vec2(_) => PortType::Vec2,
            Self::Color(_) => PortType::Color,
            Self::Audio(_) => PortType::Audio,
            Self::Field(_) => PortType::Field,
            Self::Geometry(_) => PortType::Geometry,
        }
    }

    /// The neutral value a disconnected input of type `ty` reads.
    #[must_use]
    pub fn default_for(ty: PortType) -> Self {
        match ty {
            PortType::Signal => Self::Signal(0.0),
            PortType::Gate => Self::Gate(false),
            PortType::Phase => Self::Phase(0.0),
            PortType::Vec2 => Self::Vec2([0.5, 0.5]),
            PortType::Color => Self::Color(10.0),
            PortType::Audio => Self::Audio(AudioId(0)),
            PortType::Field => Self::Field(FieldId(0)),
            PortType::Geometry => Self::Geometry(GeoId(0)),
        }
    }
}

/// A known conversion between wire types — realized as an auto-insertable
/// utility block in the patchbay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterKind {
    /// Gate → Signal: false=0, true=1 (with UI-side slew later).
    GateToSignal,
    /// Signal → Gate: threshold at 0.5.
    SignalToGate,
    /// Signal → Phase: wrap into `0..1`.
    SignalToPhase,
    /// Phase → Signal: identity carry.
    PhaseToSignal,
    /// Phase → Color: the house rule, hue = phase.
    PhaseToColor,
    /// Signal → Color: scale `0..1` onto the wheel.
    SignalToColor,
    /// Vec2 → Signal: take x (a Split block offers both lanes).
    Vec2ToSignal,
}

/// Result of asking whether `from` may feed `to`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compat {
    /// Same type: plug it in.
    Direct,
    /// Different types with a blessed conversion: patchbay inserts the
    /// adapter block automatically (and shows it — no silent coercion).
    Adapter(AdapterKind),
    /// No meaning: the patchbay refuses the cable.
    Never,
}

/// The conversion table. Deliberately explicit — every allowed coercion is
/// a design decision, not a lattice.
#[must_use]
pub fn compat(from: PortType, to: PortType) -> Compat {
    use PortType as P;
    if from == to {
        return Compat::Direct;
    }
    match (from, to) {
        (P::Gate, P::Signal) => Compat::Adapter(AdapterKind::GateToSignal),
        (P::Signal, P::Gate) => Compat::Adapter(AdapterKind::SignalToGate),
        (P::Signal, P::Phase) => Compat::Adapter(AdapterKind::SignalToPhase),
        (P::Phase, P::Signal) => Compat::Adapter(AdapterKind::PhaseToSignal),
        (P::Phase, P::Color) => Compat::Adapter(AdapterKind::PhaseToColor),
        (P::Signal, P::Color) => Compat::Adapter(AdapterKind::SignalToColor),
        (P::Vec2, P::Signal) => Compat::Adapter(AdapterKind::Vec2ToSignal),
        _ => Compat::Never,
    }
}

/// Apply an adapter to a value (used by auto-inserted conversion blocks).
#[must_use]
pub fn adapt(kind: AdapterKind, v: Value) -> Value {
    match (kind, v) {
        (AdapterKind::GateToSignal, Value::Gate(g)) => Value::Signal(f32::from(u8::from(g))),
        (AdapterKind::SignalToGate, Value::Signal(s)) => Value::Gate(s > 0.5),
        (AdapterKind::SignalToPhase, Value::Signal(s)) => Value::Phase(s.rem_euclid(1.0)),
        (AdapterKind::PhaseToSignal, Value::Phase(p)) => Value::Signal(p),
        (AdapterKind::PhaseToColor, Value::Phase(p)) => Value::Color(10.0 + p * 360.0),
        (AdapterKind::SignalToColor, Value::Signal(s)) => {
            Value::Color(10.0 + s.clamp(0.0, 1.0) * 360.0)
        }
        (AdapterKind::Vec2ToSignal, Value::Vec2([x, _])) => Value::Signal(x),
        // wrong input type for the adapter: pass a neutral value rather
        // than poisoning the graph
        (k, _) => Value::default_for(adapter_output(k)),
    }
}

/// Output type of an adapter.
#[must_use]
pub fn adapter_output(kind: AdapterKind) -> PortType {
    match kind {
        AdapterKind::GateToSignal | AdapterKind::PhaseToSignal | AdapterKind::Vec2ToSignal => {
            PortType::Signal
        }
        AdapterKind::SignalToGate => PortType::Gate,
        AdapterKind::SignalToPhase => PortType::Phase,
        AdapterKind::PhaseToColor | AdapterKind::SignalToColor => PortType::Color,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_type_states_a_hue_and_glyph() {
        for ty in [
            PortType::Signal,
            PortType::Gate,
            PortType::Phase,
            PortType::Vec2,
            PortType::Color,
            PortType::Audio,
            PortType::Field,
            PortType::Geometry,
        ] {
            assert!(ty.hue() >= 0.0 && ty.hue() < 360.0);
            assert!(!ty.glyph().is_empty());
        }
    }

    #[test]
    fn same_type_is_direct_and_media_never_coerce() {
        assert_eq!(compat(PortType::Audio, PortType::Audio), Compat::Direct);
        assert_eq!(compat(PortType::Audio, PortType::Field), Compat::Never);
        assert_eq!(compat(PortType::Field, PortType::Signal), Compat::Never);
    }

    #[test]
    fn adapters_convert_correctly() {
        assert_eq!(
            adapt(AdapterKind::GateToSignal, Value::Gate(true)),
            Value::Signal(1.0)
        );
        assert_eq!(
            adapt(AdapterKind::SignalToGate, Value::Signal(0.7)),
            Value::Gate(true)
        );
        assert_eq!(
            adapt(AdapterKind::SignalToPhase, Value::Signal(1.25)),
            Value::Phase(0.25)
        );
        match adapt(AdapterKind::PhaseToColor, Value::Phase(0.5)) {
            Value::Color(h) => assert!((h - 190.0).abs() < 1e-4),
            other => panic!("wrong type: {other:?}"),
        }
    }
}
