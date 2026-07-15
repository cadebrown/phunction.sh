# Review guidelines — the quality bar

Applies to every change, human- or agent-authored. `just check` green is
the *floor*, not the bar.

## Always

- **No silenced signals.** Failing test → fix the code. Clippy finding →
  fix or `allow` *with a written justification at the allow site* (see
  workspace lints for the pattern). Never `--no-verify`, never blanket
  `#[allow]`.
- **Invariants in doc comments.** Public items carry docs
  (`missing_docs = warn` is on) that state contracts and *why*, not
  narration of the code.
- **Errors are values.** `thiserror` enums in library crates; every
  user-reachable failure has a UI surface. `unwrap`/`expect` only where
  impossibility is provable — and the `expect` message says why.
- **Tests pin behavior, not lines.** DSP/logic changes come with native
  tests that assert observable behavior (sample-accurate offsets, bounded
  output, envelope termination — see phazor-core for the house style).
  Declarative view code doesn't need unit tests; a browser smoke test
  covers it.

## Audio-path changes (extra bar)

- Prove no-alloc/no-lock on the hot path (by construction, not assertion).
- Run `just bench` before/after; a regression needs a stated reason.
- Verify by ear or by meter telemetry in a real browser — "compiles" is
  not "works" across a worklet boundary.

## Gfx changes

- Test on both backends if the change touches the harness (Chrome =
  WebGPU; force GL by flipping the backends line locally if needed).
- Shader changes: screenshot before/after in the PR/commit description.

## Agent-specific

- Read `AGENTS.md` non-negotiables before structural work.
- Verify wgpu/leptos API shapes against the actual dependency source or
  docs MCP — both crates break API every release; training memory is stale.
- End with the Handoff report (AGENTS.md): changed / verified-in-browser /
  wasm size / bent invariants.
