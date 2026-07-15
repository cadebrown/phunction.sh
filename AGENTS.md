# AGENTS.md — phunction.sh

**phunction.sh** is an audiovisual instrument shipped as a website: a shader
lab, a browser DAW (**phazor**), and a live-performance surface — head to toe
Rust, compiled to wasm, statically hosted on Cloudflare (Workers static
assets), MIT licensed, and used live on stage where nobody can stop to debug.

This file is the map and the non-negotiables. Detail lives in `docs/`:

- [`docs/architecture.md`](docs/architecture.md) — crates, threading model, realtime budgets
- [`docs/audio-engine.md`](docs/audio-engine.md) — phazor: engine, worklet bootstrap, rings
- [`docs/gfx.md`](docs/gfx.md) — the phunctor harness; how to add a visual
- [`docs/crates-policy.md`](docs/crates-policy.md) — how dependencies get admitted
- [`docs/review-guidelines.md`](docs/review-guidelines.md) — the quality bar for every change
- [`docs/deploy.md`](docs/deploy.md) — CI/CD, Cloudflare, domain runbook

## The one diagram

```
UI thread (Leptos CSR, phunction-app)
  │ Command ring (rtrb, bytes-only)      ▲ MeterFrame ring
  ▼                                      │
audio thread = phazor Engine, run INSIDE the AudioWorklet as a second
thread of the same wasm module over shared memory (nightly, +atomics)
  ▼
speakers        …meanwhile wgpu (phunction-gfx) renders phunctors on canvas
```

## Non-negotiables

1. **Everything builds through `just`.** Threaded wasm needs `-Zbuild-std` +
   atomics rustflags, and the justfile is the single source of truth for
   them (user-level `RUSTFLAGS` env silently overrides `.cargo/config.toml`).
   Never invoke bare `trunk` or `cargo --target wasm32-*`.
2. **`just check` green before any push.** It is exactly what CI runs. CI
   deploys `main` straight to production — there is no staging.
3. **The audio path is sacred.** Inside `Engine::process` and everything it
   calls: no allocation, no locks, no syscalls, no unbounded loops, no
   strings. Commands/telemetry are `Copy` PODs over rtrb rings.
4. **Sample-accurate or don't bother.** Musical time derives from the frame
   counter (`Transport`); events split the render quantum at exact offsets.
   Never schedule audio from wall-clock or UI-side timers.
5. **No strings across the worklet ABI.** wasm-bindgen's worklet glue only
   stubs TextEncoder/Decoder; we polyfill for panics/logs, but the design
   rule stands — engine communication is bytes.
6. **Ship the debugger.** Tracing, meters, dropped-command counters, GPU
   backend readouts stay in production builds. This is open source radical
   art software; debuggability is a feature, not a leak.
7. **Target-agnostic cores.** `phazor-core` (and future DSP/math crates)
   compile natively — that's where tests and criterion benches live. Browser
   glue stays in `*-web` crates and `phunction-app`.
8. **Errors reach humans.** Any failure a visitor can hit (GPU bring-up,
   audio start) is surfaced in the UI with the real message — never a silent
   fallback or a bare spinner.
9. **Live-performance empathy.** phazor must never emit an unbounded sample
   (master tanh ceiling stays), panic must silence voices instantly, and UI
   controls must stay legible on a 1080p screenshare.

## Working here

- Plan first for anything structural; work a small concrete example; then
  generalize (the repo's whole design philosophy — see the phasor voice).
- New crate? New dependency? Read `docs/crates-policy.md` first.
- Match idiom: doc comments state *invariants and why*, not what the next
  line does. `thiserror` for library errors. Params get smoothed. Events get
  frame offsets.
- Commit conventional-commits style; the message explains what surprised you.
- Verify in a real browser (`just dev`, then drive the UI) before calling
  audio/gfx work done. `just check` + a Chrome smoke test is the definition
  of verified. Worklet errors are invisible unless you look at the page
  console — always look.

## Handoff

When you finish a work session, report: what changed, what you *verified in
the browser* (not just compiled), current wasm size (`ls dist/*_bg.wasm`),
and any invariant above you had to bend — with why.
