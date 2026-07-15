# phunctors — the visual system

*phunctor* (n.): phunction ∘ functor. A mapping from (time, params, later:
audio analysis) into light. Hosted by the wgpu harness in `phunction-gfx`.

## Adding a visual (the whole ceremony)

1. Write `crates/phunction-gfx/shaders/<id>.wgsl` containing an `fs_main`.
   The prelude (`shaders/prelude.wgsl`) is prepended automatically and
   provides: uniforms `u` (time, aspect, mod0..3), fullscreen-triangle
   `vs_main`, `cmul`, IQ `palette`, `hash21`, `TAU`.
2. Add one `PhunctorDef` entry to `REGISTRY` in `src/lib.rs` (id, name,
   glyph, tagline, `create` closure).
3. `just dev` → `/lab/<id>`. The index tile appears automatically.

Design bar for shipped phunctors: full-viewport, no dead center, readable
motion at projector distance, and *mathematically honest* — the tagline
should be a true statement about what's on screen.

## Harness contract

- `GfxContext::from_canvas` picks WebGPU when genuinely available, else
  WebGL2 (`new_instance_with_webgpu_detection`); `backend()` reports which —
  surface it in any debug UI.
- The render loop lives in the *host* (`phunction-app/src/lab.rs`): resize
  by client-size × devicePixelRatio, handle `CurrentSurfaceTexture`
  variants, stop when the canvas leaves the DOM.
- The modulation bus (`FrameInput.mods`, 4× `0..=1`) is the only channel
  from the world into a phunctor. Pointer feeds it today; audio analysis
  and MIDI feed it tomorrow. Phunctors must not read inputs any other way —
  that's what keeps them swappable and audio-reactive for free later.

## Uniform layout

Rust `Uniforms` (shader_phunctor.rs) mirrors WGSL `struct U` (prelude) by
hand. If you change one, change the other in the same commit. A naga-based
layout assertion is planned alongside the live shader editor.

## Live-coding direction

naga ships in the bundle already (wgpu dep) — the shader editor will
validate user WGSL client-side before pipeline creation, hot-swapping
`ShaderPhunctor`s. Design new harness features with "user-authored shader at
runtime" in mind.
