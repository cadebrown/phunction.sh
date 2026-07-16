//! The phunctor contract: a self-contained visual that the harness can host.
//!
//! *phunctor* (n.) — phunction ∘ functor: a mapping from (time, params,
//! audio) into light. Keep them pure in spirit: all state derives from the
//! inputs, so a phunctor can be hot-swapped, rewound, or run at any
//! resolution without ceremony.

use crate::context::GfxContext;

/// Registry metadata for a phunctor.
#[derive(Debug, Clone, Copy)]
pub struct PhunctorMeta {
    /// URL slug (`/lab/{id}`).
    pub id: &'static str,
    /// Display name.
    pub name: &'static str,
    /// A single glyph for the index tile.
    pub glyph: &'static str,
    /// One-line description for the index.
    pub tagline: &'static str,
}

/// Per-frame inputs. The modulation bus: UI knobs now, audio analysis and
/// MIDI later — phunctors never know the difference.
#[derive(Debug, Clone, Copy)]
pub struct FrameInput {
    /// Seconds since the phunctor started.
    pub time: f32,
    /// Output aspect ratio (w/h).
    pub aspect: f32,
    /// Eight general-purpose modulation channels in `0..=1` — knobs, pads,
    /// and the spectrum's coarse bands. Meaning is phunctor-local; the
    /// harness just routes them.
    pub mods: [f32; 8],
}

/// A hosted visual module.
pub trait Phunctor {
    /// Render one frame to `view`.
    fn frame(&mut self, gfx: &GfxContext, view: &wgpu::TextureView, input: &FrameInput);
}

/// A registry entry: metadata + constructor.
pub struct PhunctorDef {
    /// Metadata for the index and router.
    pub meta: PhunctorMeta,
    /// Build a fresh instance on the given context.
    pub create: fn(&GfxContext) -> Box<dyn Phunctor>,
}
