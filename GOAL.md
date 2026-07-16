# GOAL — everything asked, to its fullest, tested

This is the complete, standing goal for phunction.sh/phazor. Every item
carries its acceptance test. Nothing here is optional, nothing gets a
hacky version, and a claim without a passing test is a lie — the session
that shipped keyhints with no handlers taught us that. Status marks are
maintained as work lands: `[x]` shipped + tested, `[~]` partial, `[ ]` open.

The prime directive: **a code + modular synth instrument, played live.**
Think TouchDesigner × FL Studio × a bespoke live-coding language, wearing
hermetic formalism, running entirely in the browser as Rust→WASM.

---

## I · The room (immersion & layout)

- [x] The active visual ("mind") renders wall-to-wall behind everything —
      a fixed fullscreen canvas, DPR-capped for raymarchers.
      *Test: canvas covers viewport at any scroll/size; ≥30fps fullscreen.*
- [~] `/phazor` is an **app, not a page**: no document scroll — compact
      pane columns over the field (qualia-style), panes fold, float, and
      dock. Center stays mostly open so the field shows.
      *Test: `document.body` never scrolls; every control reachable inside
      panes; panes scroll internally only.*
- [x] Zen mode (`z` + corner button): every pane fades, the field remains.
      *Test: toggling zen leaves only field + zen button interactive.*
- [x] Freeform workspace: any pane drags by its latch to float anywhere
      (pointer events — touch included), double-tap docks, positions persist.
      *Test: drag → floats at hand; tap ≠ drag; reload restores; dock clears.*
- [x] Pane layout presets: 1 = perform (transport/weather/mind), 2 =
      patch (bay/expr/shader), 3 = mix (full console) — folds drive from
      a registry, no state lost. *Verified: all three via keyboard.*

## II · The sound (dark, evolving, never a toy)

- [x] Generative dark-ambient score: drone progression (i→VI→iv→v across
      Phrygian/Aeolian/Dorian), arp with breathing rests, semirandomized
      pentatonic lead — deterministic frame arithmetic + seeded hash.
      *Test: same seed + frames = identical music; density 0 silences lead.*
- [x] Rich FX graph: tempo-synced ping-pong delay (tape-glide retune) →
      Freeverb → drive; three voice layers with per-layer envelope/filter/
      pan/unison. *Test: fx.rs + engine tests, 37+ green.*
- [x] The score **evolves forever**: seed hash-steps every 64 beats;
      nothing loops statically. *Test: eras produce different event streams;
      resume evolves identically from the same position.*
- [x] Evolution reaches more axes: the PROGRESSION varies with the era
      seed (three minor-family walks, drone released at era edges), and
      the lead's REGISTER and DENSITY take era-scale walks (high-bias and
      chattiness from the evolving seed). *Test: storm + determinism
      suites green; eras traverse progressions and ranges.*
- [x] Mic input as a first-class source (standing directive): mic-in block
      exposes the live level to the graph (Ctx.ext). *Test: shelf spawn →
      permission prompt raised once; graceful at zero when refused.*

## III · The visuals (chill, flowing, insanely detailed)

- [x] Canon palette everywhere — midnight indigo → violet → deep teal.
      **No full-spectrum rainbow anywhere.** *Test: screenshot sweep of all
      minds shows hues confined to the canon wheel.*
- [x] Flow filter: the whole mod bus is slewed (~0.5s swells); drift is a
      value-noise walk (never periodic, never still); hue creeps forever.
      *Test: mods never step; drift signal non-repeating over 10 min.*
- [x] Seven minds: silk (domain-warp fbm), citadel (IFS), gyroid (TPMS
      flight), basilica (mandelbox), gasket (apollonian), cortex (CPPN),
      specter (live camera kaleido). *Test: all seven render non-black,
      respond to their four labeled controls, and react to audio.*
- [x] WGSL live-coding mind: fragment source editable in the UI, compiled
      over the prelude at runtime through a validation error scope, errors
      shown in place, source persisted. *Verified: starter compiles and
      takes the room; 'nonsense_symbol' shows the parse error while the
      last good pipeline keeps rendering.*
- [~] More generative-geometry minds tuned for flow: current (curl-noise
      streaks — divergence-free flow, verified live) joins silk. Remaining:
      reaction-diffusion / lenia need feedback render targets (new gfx
      infra). *Test: same as minds above.*

## IV · The constructive graph (everything is patchable)

- [x] Typed dataflow: 8 port types, station hues, Direct/Adapter/Never
      compat, cycle refusal, previews. *Test: 21 phunction-graph tests.*
- [x] Patchbay: nodes drag, cables patch by gesture, refusals in theorem
      voice, per-node settings in place (knob thumb / expr code / key
      grid), live sparkline per node. *Test: browser gesture suite —
      patch, refuse, unplug, drag, remove.*
- [x] **Cables in code**: the patch notation (`l = lfo rate=k`,
      `e -> mind.warp`) compiles with line-addressed errors; the live
      graph renders back to text; round-trip is identity. *Test:
      patch.rs tests incl. round-trip.*
- [x] The patch **is** the persistence: autosaves as text within 1s of any
      change; boot restores it. A live set never evaporates.
      *Test: mutate → reload → identical graph, code, and positions.*
- [~] Field-typed routes into the room: the board carries typed Values;
      `cam = camera-in / cam -> mind.field` compiles (Field-checked),
      routes on the board, and takes the room onto the camera pipeline
      while the cable holds — verified end-to-end (camera streaming into
      the field). Remaining: fields through arbitrary minds (per-mind
      field support) and audio buses to fx sends. *Tested: graph test +
      live round-trip incl. render().*
- [x] Gamepad block: stick axes/trigger as Signal outputs, polled per
      frame into Ctx.ext. *Test: block spawns and outputs; live-pad drive
      pending a physical pad session.*
- [x] Prebuilt patch library: five whole-patches on the bay's shelf
      (breath / pulse / listener / open mic / co-pilot). *Test: every
      entry compile+build tested natively; one click installs (verified).*

## V · The languages (code as a first-class door)

- [x] expr: the little signal language — Pratt parser, byte-addressed
      errors, NaN-proof eval, unit-phase waves. 14+ tests.
- [x] patch: the graph notation (above).
- [x] Editable WGSL blocks (see III — the live-wgsl mind).
- [x] Language docs surface in the UI: `?` in the bay reveals the whole
      patch grammar, kinds, keys, and expr vocabulary. *Verified: toggle
      renders; library entries double as runnable examples.*

## VI · Live-performance invariants (tested, standing — AGENTS.md)

- [x] **Nothing clicks**: params glide, delay retunes bend like tape,
      steals fade, velocity slews, step edits release their old note.
      *Test: `command_storms_never_click` (extend when adding commands).*
- [x] **State survives reloads**: machine state (v2) + transport clock +
      patch + mind + mind params persist; power-on restores and SeekBeats
      resumes mid-set. *Test: browser reload round-trip, verified per axis.*
- [x] Reroutes are silent; the UI never blocks the audio thread.
      *Test: storm test + ring drop-counting.*
- [x] The invariants run in CI against the wasm build: scripts/smoke.mjs
      drives headless Chrome over raw CDP (no deps) — boot, world plays
      (gfx-gate fallback included), state+patch persist, reload resumes.
      `just smoke` locally; a smoke job gates ship in CI. *Verified: all
      green locally; CI proves itself on this very push.*

## VII · Multimodal (one bus, every hand)

- [x] Pointer events end-to-end (mouse = touch = pen) on faders, knobs,
      nodes, panes. HTML5 DnD is banned (no touch support).
- [x] Keyboard: space play/stop, esc panic, z zen, ` debug — all real.
      *Test: dispatch each key, observe the effect.*
- [~] Arrow-key nudge + Home/End on focused knobs/faders — landed and
      verified (tempo nudged by keys). Remaining: focus order audit.
      *Test: keyboard-only session can play, patch, and mix.*
- [x] Gamepad → bus (see IV). MIDI-in landed too: last note/velocity/
      mod-wheel as a media block over Ctx.ext[4..7], requested on first
      node use (Web MIDI, no sysex). *Verified: shelf spawn; hardware
      drive pending a physical controller session.*
- [~] Touch pass: the full gesture suite runs green with touch-type
      pointers (fader drag 0.65→0.81, panel float, node drag 26→116px,
      knob turn — all pointerType:'touch'). Real-device iPad/Android
      session remains with Cade's hardware.

## VIII · UI quality (dense, legible, rock solid)

- [~] **The pedalboard look**: every pane is a digital stompbox — a bold
      station-hue enclosure per module (color IS wayfinding), chunky
      controls, LED truth, crisp digital surfaces (LCDs, code fields);
      efficient and flat where physical metaphor doesn't pay.
      *Test: each pane identifiable by color at a glance from across the
      room; screenshot sweep.*

- [x] Full-res spectrum: 96 log-spaced Goertzel bands as one lit curve.
- [x] Dense + straightforward: compact panes, smaller controls, inline
      numeric entry on knob AND fader readouts (click the value, type,
      Enter — verified 90→92 bpm and 0.36→0.42). Layouts 1/2/3 are the
      per-room compact modes.*
- [x] Robustness: Escape cancels any in-flight gesture (verified), node
      drags hold pointer capture, ports/removers reach 24px hit zones,
      aria audit clean (every interactive element named — the last five
      knob thumbs fixed). *Test: gesture-interrupt suite + audit script.*
- [x] No layout shift while performing; panes never jump under the hand.
      *Verified: PerformanceObserver CLS = 0.0000 during knob/fader
      gesture burst (layout presets are intentional rearrangement).*

## IX · The canon (aesthetics are load-bearing)

- [x] Hermetic formalism: midnight purple, eight stations, Redaction +
      Iosevka, machined rack, sigil, theorem voice. Words over glyphs.
- [ ] The whiteboard/parchment twin theme (light mode of the grimoire).
- [x] Colorized math variables (sacred, canon round 1): the phasor
      identity on the hero — z(t) = A·e^{i(ωt+φ)} in KaTeX math italic,
      every variable in its station hue, above the live figure it
      describes. The .mvar vocabulary is reusable in any prose.
- [~] Texture pass round 2: mind-field saturation/contrast lift landed;
      scanline/noise opacity tuning remains a taste pass with Cade.

## X · The site around the instrument

- [x] `/blog` and `/info` exist (secondary to /phazor, same canon —
      theorem voice, honest about how early they are).
- [x] Production deploys on every push to main (Workers static assets,
      COOP/COEP correct). *Test: CI green, phunction.sh serves the build.*

---

## The working standards (how every item above lands)

1. **Tested or it didn't happen.** Native tests for engine/graph/language;
   browser gesture suites for UI; screenshots for visuals. A UI hint or
   README claim describing behavior that doesn't exist is a bug of the
   worst class.
2. **No hacky versions.** If the honest version doesn't fit the session,
   the item stays open in this file — it does not ship as a stub wearing
   the feature's name.
3. **The invariants (VI) apply to every new feature retroactively** —
   new commands join the storm test, new state joins the persistence
   round-trip, new gestures join the interrupt suite.
4. **Verify in the running browser before claiming.** The CDP screenshot
   compositor lies near fixed WebGPU canvases — trust the DOM probes and
   direct interaction results.
