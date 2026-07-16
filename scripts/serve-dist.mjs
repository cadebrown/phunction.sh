// Static dist/ server with the COOP/COEP headers threaded wasm requires.
// Used by the Playwright suite (playwright.config.mjs webServer); the CDP
// smoke harness embeds its own copy on a random port.
import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { extname, join } from "node:path";

const PORT = Number(process.argv[2] ?? 4173);
const DIST = new URL("../dist", import.meta.url).pathname;
const MIME = {
  ".html": "text/html",
  ".js": "text/javascript",
  ".wasm": "application/wasm",
  ".css": "text/css",
  ".woff2": "font/woff2",
  ".svg": "image/svg+xml",
};

createServer(async (req, res) => {
  const path = req.url.split("?")[0];
  const file = path === "/" || !extname(path) ? "/index.html" : path;
  try {
    const body = await readFile(join(DIST, file));
    res.writeHead(200, {
      "content-type": MIME[extname(file)] ?? "application/octet-stream",
      "cross-origin-opener-policy": "same-origin",
      "cross-origin-embedder-policy": "require-corp",
    });
    res.end(body);
  } catch {
    res.writeHead(404).end();
  }
}).listen(PORT, () => console.log(`dist on :${PORT}`));
