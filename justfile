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
# Runs forever; each successful rebuild bumps .dev/last-build (Trunk hook).
# Run this in your own terminal; agents watch the stamp, not the logs.
dev:
    RUSTFLAGS='{{WASM_RUSTFLAGS}}' CARGO_UNSTABLE_BUILD_STD={{BUILD_STD}} trunk serve --open

# Same as dev but headless on a fixed port (agents/CI smoke tests).
serve port='8380':
    RUSTFLAGS='{{WASM_RUSTFLAGS}}' CARGO_UNSTABLE_BUILD_STD={{BUILD_STD}} trunk serve --port {{port}}

# Block until the next successful rebuild lands (used by agents).
wait-build stamp='':
    #!/usr/bin/env sh
    old="{{stamp}}"
    [ -z "$old" ] && old=$(cat .dev/last-build 2>/dev/null || echo 0)
    for _ in $(seq 1 120); do
        cur=$(cat .dev/last-build 2>/dev/null || echo 0)
        [ "$cur" != "$old" ] && echo "built: $cur" && exit 0
        sleep 2
    done
    echo "timeout waiting for rebuild" >&2; exit 1

# One-shot dev rebuild into dist/ (~3s warm). The running `just dev` serves
# dist/ from disk, so this lands without its file watcher — use when the
# watcher wedges (it drops events after error bursts) or for deterministic
# agent-driven rebuilds. Reload the browser manually; the auto-reload
# websocket only fires on watcher builds.
rebuild:
    RUSTFLAGS='{{WASM_RUSTFLAGS}}' CARGO_UNSTABLE_BUILD_STD={{BUILD_STD}} trunk build

# Browser smoke against dist/ (build first): boot → play → persist → resume.
smoke:
    node scripts/smoke.mjs

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
