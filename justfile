# phunction.sh task runner. `just` with no args lists targets.
#
# Threaded wasm needs (a) atomics rustflags and (b) std rebuilt with them.
# Both are set HERE, explicitly, because a user-level `RUSTFLAGS` env var
# (common in dotfiles) silently overrides `.cargo/config.toml` target
# rustflags — we found this out the hard way. Drive all wasm builds through
# these targets, never bare `trunk`/`cargo --target wasm32-*`.

WASM_RUSTFLAGS := "-C target-feature=+atomics,+bulk-memory,+mutable-globals -C link-arg=--shared-memory -C link-arg=--export=__heap_base -C link-arg=--export=__wasm_init_tls -C link-arg=--export=__tls_size -C link-arg=--export=__tls_align -C link-arg=--export=__tls_base -C link-arg=--import-memory -C link-arg=--max-memory=1073741824"
BUILD_STD := "std,panic_abort"

default:
    @just --list

# Dev server with COOP/COEP headers (audio threads work locally).
dev:
    RUSTFLAGS='{{WASM_RUSTFLAGS}}' CARGO_UNSTABLE_BUILD_STD={{BUILD_STD}} trunk serve --open

# Production build into dist/.
build:
    RUSTFLAGS='{{WASM_RUSTFLAGS}}' CARGO_UNSTABLE_BUILD_STD={{BUILD_STD}} trunk build --release

# Native tests (fast path — DSP/logic crates).
test:
    cargo nextest run

# Everything CI runs, in CI's order. Green here = green there.
check: fmt-check clippy clippy-wasm test

# DSP throughput benchmarks.
bench:
    cargo bench -p phazor-core

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all --check

clippy:
    cargo clippy --workspace --all-targets -- -D warnings

# Clippy for the real target: catches wasm-only breakage natively missed.
clippy-wasm:
    RUSTFLAGS='{{WASM_RUSTFLAGS}}' CARGO_UNSTABLE_BUILD_STD={{BUILD_STD}} cargo clippy --workspace --target wasm32-unknown-unknown -- -D warnings

# Deploy to Cloudflare (Workers static assets). CI does this on main; manual
# use is for emergencies.
deploy: build
    npx wrangler@4 deploy

# Serve the production build locally exactly as Cloudflare will.
preview: build
    npx wrangler@4 dev
