// UI-functionality suite config. `just ui-test` locally, the `smoke` CI job
// runs it against the same release dist/. Chromium only: the app's contract
// is Chromium-class engines first (WebGPU, threaded wasm).
import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "tests/ui",
  timeout: 60_000,
  retries: process.env.CI ? 1 : 0,
  workers: 1, // one AudioContext-owning page at a time keeps timing honest
  use: {
    baseURL: "http://127.0.0.1:4173",
    trace: "retain-on-failure",
    launchOptions: {
      args: [
        "--autoplay-policy=no-user-gesture-required",
        "--enable-unsafe-webgpu",
      ],
    },
  },
  webServer: {
    command: "node scripts/serve-dist.mjs 4173",
    port: 4173,
    reuseExistingServer: !process.env.CI,
  },
});
