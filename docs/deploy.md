# Deploy — CI/CD and Cloudflare

## Pipeline

Push to `main` → GitHub Actions (`.github/workflows/ship.yml`):
1. `check` job: `just check` (fmt, clippy native+wasm, nextest) — identical
   to local.
2. `ship` job (main only): `just build` → `wrangler deploy` (Worker with
   static assets, config in `wrangler.toml`).

Secrets: `CLOUDFLARE_API_TOKEN` (repo secret, already set). Account:
Alkemical Development (`1f6e8a09fec260d326f4af42d7868238`).

No staging, no preview gate — the lab is the stage. `just preview` runs the
production build locally via `wrangler dev` when you want a dress rehearsal.

## Static-asset specifics

- SPA fallback: `not_found_handling = "single-page-application"` in
  `wrangler.toml` (leptos_router owns the URL space).
- `public/_headers` ships COOP/COEP (`same-origin` / `require-corp`) on
  `/*` — **required** for SharedArrayBuffer/threaded audio. If audio works
  in `just dev` but not in prod, this file didn't make it to `dist/`.
- Trunk copies `public/` → `dist/` root; hashed assets are immutable.

## Domain runbook (cutover from the legacy Pages project)

State: zone `phunction.sh` (id `3547514dbfa93ac11859254ce6d7734f`) on the
Alkemical account; the *old* Astro site lives in Pages project
`phunction-sh` which owns the `phunction.sh` custom domain.

Cutover, in order:
1. Deploy this repo's worker; verify on the `workers.dev` URL (audio +
   `/lab/argand` + hard-refresh headers check).
2. Delete the legacy Pages project `phunction-sh` (releases the domain).
3. Uncomment the `routes` block in `wrangler.toml`; redeploy. Workers
   custom domains create the DNS records automatically.
4. Verify `https://phunction.sh` end to end (COOP/COEP present, audio
   starts, phunctor renders); then delete any stale DNS records pointing at
   `*.pages.dev` if the dashboard left them behind.

Rollback: Workers keeps prior versions — `npx wrangler rollback`.
