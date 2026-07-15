//! `AudioWorklet` thread bootstrap.
//!
//! The trick (from wasm-bindgen's `wasm-audio-worklet` example, adapted for
//! Trunk): the engine is constructed on the UI thread, boxed, and *leaked to
//! a raw pointer*; the pointer rides `processorOptions` into the worklet
//! together with this module's `WebAssembly.Module` and shared memory. The
//! worklet re-instantiates the same module over the same memory (becoming a
//! second thread of this program) and unpacks the pointer. From then on the
//! rtrb rings inside [`PhazorProcessor`] are ordinary shared-memory SPSC
//! queues between the two threads.
//!
//! String hygiene: wasm-bindgen's glue only *stubs* TextEncoder/TextDecoder
//! inside worklets, so the worklet module first imports a tiny real UTF-8
//! polyfill — that is what makes panic messages and tracing from the audio
//! thread legible instead of a cryptic throw.

use phazor_core::{Command, Engine, MeterFrame};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{AudioContext, AudioWorkletNode, AudioWorkletNodeOptions};

/// The audio-thread half of phazor: engine + ring endpoints.
///
/// Exposed to wasm-bindgen so the worklet-side glue can call `process` on
/// the unpacked instance every render quantum.
#[wasm_bindgen]
pub struct PhazorProcessor {
    engine: Engine,
    commands: rtrb::Consumer<Command>,
    meters: rtrb::Producer<MeterFrame>,
}

#[wasm_bindgen]
impl PhazorProcessor {
    /// Render one quantum. `left`/`right` are the worklet output channels.
    /// Returns `true` to keep the node alive.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32]) -> bool {
        // Drain UI commands at block start — bounded by ring capacity, so
        // this loop cannot stall the audio thread.
        while let Ok(cmd) = self.commands.pop() {
            self.engine.apply(cmd);
        }
        let meter = self.engine.process(left, right);
        // Telemetry is best-effort: if the UI is behind, drop frames rather
        // than block.
        let _ = self.meters.push(meter);
        true
    }

    /// Leak self to a pointer-sized handle that can cross `processorOptions`.
    pub fn pack(self) -> usize {
        Box::into_raw(Box::new(self)) as usize
    }

    /// Reclaim a handle produced by [`PhazorProcessor::pack`].
    ///
    /// # Safety
    /// `handle` must be exactly the value returned by `pack`, passed once —
    /// double-unpack is a double-free.
    #[allow(clippy::missing_safety_doc)] // documented above; wasm_bindgen strips the section
    pub unsafe fn unpack(handle: usize) -> Self {
        unsafe { *Box::from_raw(handle as *mut Self) }
    }
}

/// Minimal-but-correct UTF-8 TextEncoder/TextDecoder for worklet scope.
/// Installed only if the real ones are absent.
const TEXT_CODEC_POLYFILL: &str = r"
if (typeof globalThis.TextDecoder === 'undefined') {
  globalThis.TextDecoder = class TextDecoder {
    decode(buf) {
      // wasm-bindgen's glue calls decode() with no args once as a warm-up.
      if (buf === undefined || buf === null) return '';
      const b = new Uint8Array(buf.buffer ? buf.buffer : buf, buf.byteOffset ?? 0, buf.byteLength);
      let s = '', i = 0;
      while (i < b.length) {
        const c = b[i++];
        if (c < 0x80) s += String.fromCharCode(c);
        else if (c < 0xe0) s += String.fromCharCode(((c & 0x1f) << 6) | (b[i++] & 0x3f));
        else if (c < 0xf0) s += String.fromCharCode(((c & 0x0f) << 12) | ((b[i++] & 0x3f) << 6) | (b[i++] & 0x3f));
        else {
          const cp = (((c & 0x07) << 18) | ((b[i++] & 0x3f) << 12) | ((b[i++] & 0x3f) << 6) | (b[i++] & 0x3f)) - 0x10000;
          s += String.fromCharCode(0xd800 + (cp >> 10), 0xdc00 + (cp & 0x3ff));
        }
      }
      return s;
    }
  };
}
if (typeof globalThis.TextEncoder === 'undefined') {
  globalThis.TextEncoder = class TextEncoder {
    encode(s) {
      const out = [];
      for (const ch of s) {
        const cp = ch.codePointAt(0);
        if (cp < 0x80) out.push(cp);
        else if (cp < 0x800) out.push(0xc0 | (cp >> 6), 0x80 | (cp & 0x3f));
        else if (cp < 0x10000) out.push(0xe0 | (cp >> 12), 0x80 | ((cp >> 6) & 0x3f), 0x80 | (cp & 0x3f));
        else out.push(0xf0 | (cp >> 18), 0x80 | ((cp >> 12) & 0x3f), 0x80 | ((cp >> 6) & 0x3f), 0x80 | (cp & 0x3f));
      }
      return new Uint8Array(out);
    }
    encodeInto(s, dst) {
      const src = this.encode(s);
      const n = Math.min(src.length, dst.length);
      dst.set(src.subarray(0, n));
      return { read: s.length, written: n };
    }
  };
}
";

#[wasm_bindgen(inline_js = r#"
export function createPhazorWorkletUrl(glueUrl, polyfillSource) {
    // Two-stage blob: the polyfill must be a *separate module imported
    // first*, because static imports evaluate depth-first — putting the
    // polyfill inline after the glue import would run it too late.
    const polyfillUrl = URL.createObjectURL(
        new Blob([polyfillSource], { type: 'text/javascript' }));
    const worklet = `
        import '${polyfillUrl}';
        import * as bindgen from '${glueUrl}';
        // Worklet-side failures are eaten by the browser unless surfaced by
        // hand; console.error from a worklet does reach the page console.
        try {
            registerProcessor('phazor', class extends AudioWorkletProcessor {
                constructor(options) {
                    super();
                    try {
                        const [module, memory, handle] = options.processorOptions;
                        bindgen.initSync({ module, memory });
                        this.proc = bindgen.PhazorProcessor.unpack(handle);
                    } catch (e) {
                        console.error('[phazor worklet] init failed:', e);
                        throw e;
                    }
                }
                process(inputs, outputs) {
                    const out = outputs[0];
                    return this.proc.process(out[0], out[1] ?? out[0]);
                }
            });
        } catch (e) {
            console.error('[phazor worklet] registration failed:', e);
            throw e;
        }
    `;
    return URL.createObjectURL(new Blob([worklet], { type: 'text/javascript' }));
}

export function mainGlueUrl() {
    // Trunk hashes the glue filename, so discover it from the modulepreload
    // hint Trunk injects into index.html. Blob-URL worklet modules cannot
    // use relative imports, so this must be absolute.
    const link = document.querySelector("link[rel=modulepreload][href*='phunction-app']");
    if (!link) throw new Error('phunction-app modulepreload link not found');
    return link.href;
}
"#)]
extern "C" {
    fn createPhazorWorkletUrl(glue_url: &str, polyfill_source: &str) -> String;
    fn mainGlueUrl() -> String;
}

/// UI-thread handle to the running engine.
pub struct Phazor {
    /// Send commands to the engine (lock-free, never blocks).
    pub commands: rtrb::Producer<Command>,
    /// Drain engine telemetry (lock-free; latest-wins consumption is fine).
    pub meters: rtrb::Consumer<MeterFrame>,
    /// The audio context (exposed for resume/suspend on user gesture).
    pub ctx: AudioContext,
    /// The worklet node (kept alive; disconnect to stop).
    pub node: AudioWorkletNode,
}

/// Boot the engine into an `AudioWorklet` thread. Call from a user gesture —
/// browsers refuse to start audio otherwise.
pub async fn start() -> Result<Phazor, JsValue> {
    let ctx = AudioContext::new()?;
    let worklet_url = createPhazorWorkletUrl(&mainGlueUrl(), TEXT_CODEC_POLYFILL);
    JsFuture::from(ctx.audio_worklet()?.add_module(&worklet_url)?).await?;

    let (cmd_tx, cmd_rx) = rtrb::RingBuffer::new(crate::COMMAND_RING_CAPACITY);
    let (meter_tx, meter_rx) = rtrb::RingBuffer::new(crate::METER_RING_CAPACITY);

    #[allow(clippy::cast_possible_truncation)]
    let engine = Engine::new(ctx.sample_rate());
    let handle = PhazorProcessor {
        engine,
        commands: cmd_rx,
        meters: meter_tx,
    }
    .pack();

    let options = AudioWorkletNodeOptions::new();
    options.set_output_channel_count(&js_sys::Array::of1(&2u32.into()));
    options.set_processor_options(Some(&js_sys::Array::of3(
        &wasm_bindgen::module(),
        &wasm_bindgen::memory(),
        &handle.into(),
    )));
    let node = AudioWorkletNode::new_with_options(&ctx, "phazor", &options)?;
    node.connect_with_audio_node(&ctx.destination())?;

    Ok(Phazor {
        commands: cmd_tx,
        meters: meter_rx,
        ctx,
        node,
    })
}
