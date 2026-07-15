//! # phunction-gfx
//!
//! The visual half of phunction: a small wgpu harness that owns the surface
//! and frame loop, and hosts **phunctors** — self-contained shader-art
//! modules (phunction ∘ functor) listed in [`REGISTRY`], launched
//! fullscreen, and driven by a shared modulation bus.
//!
//! Adding a phunctor: write `shaders/<id>.wgsl` with an `fs_main` (the
//! prelude provides uniforms + helpers), add a [`PhunctorDef`] to
//! [`REGISTRY`]. That's the whole ceremony — the lab index and router pick
//! it up from there.

pub mod context;
pub mod phunctor;
pub mod shader_phunctor;

pub use context::{GfxContext, GfxError};
pub use phunctor::{FrameInput, Phunctor, PhunctorDef, PhunctorMeta};
pub use shader_phunctor::ShaderPhunctor;
// Re-exported so hosts (the app's render loop) never need their own wgpu
// dependency — one wgpu version, decided here.
pub use wgpu;

/// Every phunctor the lab ships. Order = display order on the index.
pub static REGISTRY: &[PhunctorDef] = &[PhunctorDef {
    meta: PhunctorMeta {
        id: "argand",
        name: "argand",
        glyph: "ℂ",
        tagline: "domain-colored phasor field — three poles orbiting the complex plane",
    },
    create: |gfx| {
        Box::new(ShaderPhunctor::new(
            gfx,
            include_str!("../shaders/argand.wgsl"),
        ))
    },
}];

/// Look up a phunctor by URL slug.
#[must_use]
pub fn find(id: &str) -> Option<&'static PhunctorDef> {
    REGISTRY.iter().find(|d| d.meta.id == id)
}

/// The site substrate: the domain-colored field every page floats on.
/// Not a lab exhibit — it's the weather.
#[must_use]
pub fn substrate(gfx: &GfxContext) -> ShaderPhunctor {
    ShaderPhunctor::new(gfx, include_str!("../shaders/substrate.wgsl"))
}
