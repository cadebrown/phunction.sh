# phunction.sh

> psychedelic code futurism · mathematical intuition made audible and visible
> · open source radical art software

The public instrument of **phunction** — an audiovisual artist project. This
is not a portfolio site; it is the artwork: a shader lab, a browser DAW
(**phazor**), and a live-performance surface, implemented head to toe in
Rust, compiled to WebAssembly, and shipped as static files that run on
anything with a browser — laptop, phone, projector, whatever's lying around.

Sibling project: [voidstar.sh](https://github.com/randbrown/voidstar.sh) —
we perform together; the instruments are different on purpose.

## What's inside

| thing | what it is | where |
|---|---|---|
| **phazor** | hand-rolled DAW engine — the Rust engine runs as a *thread inside the AudioWorklet* over shared wasm memory; sample-accurate sequencer, phasor-bank voices | `crates/phazor-core`, `crates/phazor-web` |
| **the lab** | fullscreen shader-art experiments ("phunctors") on a wgpu WebGPU/WebGL2 harness | `crates/phunction-gfx` |
| **the shell** | Leptos (CSR) site: routing, live panels, debug HUD — screenshare-ready, keyboard-first | `crates/phunction-app` |

Everything debuggable ships to production: tracing console, param inspector,
perf HUD. Radical art software means you get the wiring diagram.

## Run it

```sh
rustup toolchain install          # picks up rust-toolchain.toml (nightly)
cargo binstall trunk just         # build tools
just dev                          # → http://localhost:8080, threads enabled
just test                         # native tests (DSP core runs anywhere)
just bench                        # criterion DSP throughput benches
just check                        # exactly what CI runs
```

**Always build through `just`** — threaded wasm needs `-Zbuild-std` plus
atomics rustflags, and a `RUSTFLAGS` in your dotfiles would silently eat the
repo's config otherwise (ask us how we know).

## For agents & contributors

Read [`AGENTS.md`](AGENTS.md) first — it is the map and the non-negotiables.
Architecture, crate policy, and review bar live in [`docs/`](docs/).

## Deploy

Push to `main` → GitHub Actions → `just build` → Cloudflare Worker (static
assets) → [phunction.sh](https://phunction.sh). No staging. The lab *is* the
stage.

## License

[MIT](LICENSE). Take it, fork it, make weirder art with it.
