# CLAUDE.md

Agent guidance lives in **[`AGENTS.md`](AGENTS.md)** — read it first. It is
the map (crate layout, threading diagram) and the non-negotiables (realtime
rules, `just`-only builds, ship-the-debugger ethos).

Quick pointers:

- Build/dev/test: `just dev` · `just check` · `just test` · `just bench`
- Architecture → [`docs/architecture.md`](docs/architecture.md)
- Audio engine (phazor) → [`docs/audio-engine.md`](docs/audio-engine.md)
- Visuals (phunctors) → [`docs/gfx.md`](docs/gfx.md)
- Adding dependencies → [`docs/crates-policy.md`](docs/crates-policy.md)
- Review bar → [`docs/review-guidelines.md`](docs/review-guidelines.md)
- Deploy/Cloudflare → [`docs/deploy.md`](docs/deploy.md)

Useful MCP servers when working here: `crates` (verify versions/health
before adding deps), `rust-docs`/`context7` (API surface — wgpu and leptos
move fast; don't guess), `cloudflare` (deploy/DNS state).
