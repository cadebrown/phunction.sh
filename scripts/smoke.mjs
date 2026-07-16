// Browser smoke: the live-performance invariants against the real wasm
// build, in a real Chrome. Boot → power on → the machine plays → state
// persists → reload resumes. Runs locally (`node scripts/smoke.mjs`) and
// in CI; needs only Node ≥ 20 and a Chrome binary (CHROME_PATH or PATH).
//
// WebGPU may be absent (CI): the viewport error path + the gfx-gate
// fallback timer must still start the world — that is itself part of the
// contract being smoked.

import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { extname, join } from "node:path";
import { execFileSync, spawn } from "node:child_process";

const DIST = new URL("../dist", import.meta.url).pathname;
const MIME = {
  ".html": "text/html",
  ".js": "text/javascript",
  ".wasm": "application/wasm",
  ".css": "text/css",
  ".woff2": "font/woff2",
  ".svg": "image/svg+xml",
};

// -- a dist server with the COOP/COEP headers threaded wasm needs
const server = createServer(async (req, res) => {
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
});
await new Promise((r) => server.listen(0, r));
const port = server.address().port;
const base = `http://127.0.0.1:${port}`;

// -- drive Chrome over CDP, no puppeteer dependency
const chrome =
  process.env.CHROME_PATH ??
  ["google-chrome", "chromium-browser", "chromium", "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"].find(
    (c) => {
      try {
        execFileSync(c, ["--version"], { stdio: "ignore" });
        return true;
      } catch {
        return false;
      }
    },
  );
if (!chrome) {
  console.error("smoke: no chrome found (set CHROME_PATH)");
  process.exit(2);
}

const proc = spawn(chrome, [
  "--headless=new",
  "--remote-debugging-port=0",
  "--no-sandbox",
  "--disable-gpu-sandbox",
  "--autoplay-policy=no-user-gesture-required",
  "--user-data-dir=/tmp/phunction-smoke-profile",
  "about:blank",
]);
const wsUrl = await new Promise((resolve, reject) => {
  let buf = "";
  proc.stderr.on("data", (d) => {
    buf += d;
    const m = buf.match(/DevTools listening on (ws:\/\/\S+)/);
    if (m) resolve(m[1]);
  });
  setTimeout(() => reject(new Error("chrome did not start")), 20000);
});

const ws = new WebSocket(wsUrl);
await new Promise((r) => (ws.onopen = r));
let msgId = 0;
const pending = new Map();
ws.onmessage = (ev) => {
  const msg = JSON.parse(ev.data);
  if (msg.id && pending.has(msg.id)) {
    pending.get(msg.id)(msg);
    pending.delete(msg.id);
  }
};
const send = (method, params = {}, sessionId) =>
  new Promise((resolve) => {
    const id = ++msgId;
    pending.set(id, resolve);
    ws.send(JSON.stringify({ id, method, params, sessionId }));
  });

const { result: { targetId } } = await send("Target.createTarget", { url: `${base}/phazor` });
const { result: { sessionId } } = await send("Target.attachToTarget", { targetId, flatten: true });
await send("Runtime.enable", {}, sessionId);

const evaluate = async (expression) => {
  const msg = await send(
    "Runtime.evaluate",
    { expression, awaitPromise: true, returnByValue: true },
    sessionId,
  );
  if (msg.error) throw new Error(`cdp error: ${JSON.stringify(msg.error)}`);
  const result = msg.result;
  if (result?.exceptionDetails) {
    throw new Error(
      `page threw: ${result.exceptionDetails.exception?.description ?? result.exceptionDetails.text}`,
    );
  }
  return result?.result?.value;
};

// wait for the document to exist before asking it questions
for (let i = 0; i < 50; i++) {
  try {
    if ((await evaluate("document.readyState")) === "complete") break;
  } catch {
    // session warming up
  }
  await new Promise((r) => setTimeout(r, 200));
}

const fail = (msg) => {
  console.error(`smoke ✗ ${msg}`);
  proc.kill();
  process.exit(1);
};

// 1. boot: the power button must exist
const booted = await evaluate(`(async () => {
  for (let i = 0; i < 100; i++) {
    if ([...document.querySelectorAll('button')].some(b => b.innerText.includes('power on'))) return true;
    await new Promise(r => setTimeout(r, 200));
  }
  return false;
})()`);
if (!booted) fail("power button never appeared (wasm boot failed)");
console.log("smoke ✓ boot");

// 2. power on → the machine must start playing (gfx-gate fallback included)
const playing = await evaluate(`(async () => {
  [...document.querySelectorAll('button')].find(b => b.innerText.includes('power on')).click();
  const beat = () => parseFloat(document.body.innerText.match(/beat\\s+([\\d.]+)/)?.[1] ?? '-1');
  for (let i = 0; i < 80; i++) {
    await new Promise(r => setTimeout(r, 250));
    if (beat() > 0.5) return true;
  }
  return false;
})()`);
if (!playing) fail("the world never started playing after power on");
console.log("smoke ✓ plays");

// 3. state persists — eventually-consistent by design (patch autosave is
// a ~1s rev-diff, the clock stamps every 2s), so poll instead of racing it
const persisted = await evaluate(`(async () => {
  for (let i = 0; i < 60; i++) {
    if (localStorage.getItem('phazor:state') && localStorage.getItem('phazor:patch')) return true;
    await new Promise(r => setTimeout(r, 250));
  }
  return false;
})()`);
if (!persisted) fail("machine state / patch not persisted within 15s");
console.log("smoke ✓ persists");

// 4. reload resumes: the reload comes from CDP (an in-page reload would
// kill its own evaluation context), then a fresh context checks resume
const beatBefore = await evaluate(
  `parseFloat(document.body.innerText.match(/beat\\s+([\\d.]+)/)?.[1] ?? '0')`,
);
await send("Page.enable", {}, sessionId);
await send("Page.reload", { ignoreCache: false }, sessionId);
await new Promise((r) => setTimeout(r, 3000));
const resumed = await evaluate(`const BEAT_BEFORE = ${beatBefore};
(async () => {
  for (let i = 0; i < 100; i++) {
    const btn = [...document.querySelectorAll('button')].find(b => b.innerText.includes('power on'));
    if (btn) { btn.click(); break; }
    await new Promise(r => setTimeout(r, 200));
  }
  for (let i = 0; i < 80; i++) {
    await new Promise(r => setTimeout(r, 250));
    const beat = parseFloat(document.body.innerText.match(/beat\\s+([\\d.]+)/)?.[1] ?? '-1');
    if (beat >= Math.max(0.5, BEAT_BEFORE - 8)) return true;
  }
  return false;
})()`);
if (!resumed) fail("reload did not resume the set");
console.log("smoke ✓ resumes (from beat " + beatBefore.toFixed(1) + ")");

console.log("smoke: all green");
proc.kill();
server.close();
process.exit(0);
