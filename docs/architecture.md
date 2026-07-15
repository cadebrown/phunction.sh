# Architecture

## Crates

| crate | what | target |
|---|---|---|
| `phazor-core` | DSP + musical time: `Transport`, `Phasor` oscillator, Simper SVF, ADSR, `StepSequencer`, `Engine`. Zero I/O, near-zero deps. | native + wasm |
| `phazor-web` | Makes the engine audible: AudioWorklet thread bootstrap, rtrb ring endpoints, `AudioContext` glue. | wasm (empty shell natively) |
| `phunction-gfx` | wgpu harness: `GfxContext` (WebGPU→WebGL2 fallback), `Phunctor` trait + registry, `ShaderPhunctor` WGSL host. | native-checkable, runs on wasm |
| `phunction-app` | The site: Leptos CSR shell, routes, phazor panel, lab viewer. Wiring only — heavy machinery stays in the engine crates. | wasm |

Dependency direction: `app → {phazor-web, phunction-gfx} → phazor-core`.
`phazor-core` depends on almost nothing (heapless) — that is load-bearing:
it's what makes the DSP natively testable and benchable.

## Threading model (the deliberate hard part)

The whole app is **one wasm module with shared memory** (nightly,
`+atomics,+bulk-memory,+mutable-globals`, `-Zbuild-std=std,panic_abort`).
The AudioWorklet re-instantiates the module over the same
`WebAssembly.Memory`, making the audio thread a true second thread: rtrb
SPSC rings and atomics work exactly as they do natively.

Bootstrap sequence (`phazor-web/src/worklet.rs`):
1. UI thread constructs `Engine` + both rings, wraps them in
   `PhazorProcessor`, leaks it to a `usize` handle.
2. `audioWorklet.addModule(blobUrl)` — the blob imports a TextCodec
   polyfill module *first*, then the wasm-bindgen glue (resolved at runtime
   from Trunk's modulepreload link, because Trunk hashes filenames).
3. `AudioWorkletNode` gets `[module, memory, handle]` via processorOptions;
   the worklet `initSync`s and unpacks the processor.
4. `process()` = drain command ring → `Engine::process(left, right)` → push
   `MeterFrame`.

Consequences you must respect:
- COOP/COEP headers are required everywhere (dev: Trunk.toml; prod:
  `public/_headers`). `require-corp`, not `credentialless` (Safari).
- The exact rustflags live in the **justfile only** (env RUSTFLAGS
  overrides config-file rustflags — found out the hard way).
- wasm-opt runs with `--all-features` (externref + atomics).

## Realtime budgets

- 128-frame quantum @ 48 kHz = **2.67 ms** hard deadline on the audio
  thread, on a phone, in wasm. Keep native `just bench` ≥ 50× real-time as
  the proxy margin.
- Meter/telemetry drain runs on rAF; it must never block (latest-wins).
- First paint budget: keep the wasm ≤ ~500 KB post-wasm-opt; profile with
  twiggy before adding UI-side dependencies.

## Musical time

`Transport` owns a frame counter advanced only by `Engine::process`. Beats
are *derived* (`frames → beats` at current tempo), tempo changes rebase the
counter so position is continuous. The sequencer converts step boundaries to
frame offsets within each block; the engine splits the buffer at offsets.
There is no other clock. Anything that needs musical time asks the engine.
