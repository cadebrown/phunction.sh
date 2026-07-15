# Crate policy — how a dependency earns its place

Every dependency is code we ship to every visitor's browser and audit
forever. The bar:

1. **Search the workspace first.** The helper may exist (`rg` before you
   write or import).
2. **Prefer the blessed set.** blessed.rs picks, or crates already in
   `[workspace.dependencies]`. We deliberately use: leptos, wgpu,
   wasm-bindgen/web-sys/js-sys, rtrb, heapless, thiserror, tracing, serde,
   glam, bytemuck, web-time, gloo (when needed), fundsp (when the DSP
   library moment comes), criterion/proptest/approx for dev.
3. **Verify before adding** (don't trust memory — the ecosystem moves):
   - current version + release recency (crates MCP `get_crate_info` /
     `get_crate_health`),
   - maintenance status (rustwasm-org-style sunsets happen),
   - wasm32 compatibility (check features; `wasm_js`-style flags),
   - transitive weight (`get_dependency_tree`; then twiggy the bundle).
4. **Realtime crates** (anything the audio thread touches) must be
   allocation-free on the hot path and ideally `no_std`-capable. rtrb and
   heapless set the standard.
5. **Add at the workspace level** (`[workspace.dependencies]`), reference
   with `dep.workspace = true`. One version per crate across the workspace.
6. **Known dead ends** (do not re-add): `wee_alloc` (unmaintained, leaks),
   `tracing-wasm` (abandoned — use tracing with a custom console layer),
   `wasm-pack` for app builds (it's for libraries; Trunk owns the app
   pipeline).

Version pins: minor-level (`"0.8"`, `"30"`) — Cargo.lock is committed, CI
builds from the lock, upgrades are deliberate commits.
