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
- [ ] Pane layout presets ("perform", "patch", "mix") — one key each.
      *Test: switching layouts rearranges panes without losing state.*

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
- [ ] Evolution reaches more axes: chord-progression variation between
      eras, register/density long-walks, world-morphing over minutes.
      *Test: 10-minute render shows measurable event-distribution drift.*
- [ ] Mic input as a first-class source (standing directive): mic-in block
      exposes live rms/bands to the graph. *Test: mic-in block outputs
      nonzero under signal; permission requested once, gracefully denied.*

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
- [ ] WGSL live-coding mind: fragment source editable in the UI, compiled
      over the prelude at runtime, errors shown in place, source persisted.
      *Test: edit → visual changes < 1s; bad WGSL shows the error, keeps
      the last good pipeline running.*
- [ ] More generative-geometry minds tuned for flow (curl-noise advection,
      reaction-diffusion, lenia-like). *Test: same as minds above.*

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
- [ ] Field/Audio-typed routes into the room: camera/wgsl fields patchable
      to the mind slot; audio buses patchable to fx sends.
      *Test: camera-in → mind.field renders the camera through any mind.*
- [ ] Gamepad block: stick axes/triggers as Signal outputs, polled per
      frame. *Test: with a pad connected, axes drive a patched target.*
- [ ] Prebuilt patch library: a shelf of whole-patches (worlds for the
      graph). *Test: one click installs; all compile.*

## V · The languages (code as a first-class door)

- [x] expr: the little signal language — Pratt parser, byte-addressed
      errors, NaN-proof eval, unit-phase waves. 14+ tests.
- [x] patch: the graph notation (above).
- [ ] Editable WGSL blocks (see III).
- [ ] Language docs surface in the UI (`?` reveals vars/functions/grammar
      in each code field). *Test: help renders, examples run.*

## VI · Live-performance invariants (tested, standing — AGENTS.md)

- [x] **Nothing clicks**: params glide, delay retunes bend like tape,
      steals fade, velocity slews, step edits release their old note.
      *Test: `command_storms_never_click` (extend when adding commands).*
- [x] **State survives reloads**: machine state (v2) + transport clock +
      patch + mind + mind params persist; power-on restores and SeekBeats
      resumes mid-set. *Test: browser reload round-trip, verified per axis.*
- [x] Reroutes are silent; the UI never blocks the audio thread.
      *Test: storm test + ring drop-counting.*
- [ ] The invariants run in CI against the wasm build too (headless
      chrome smoke: boot, patch, reload-resume). *Test: CI job green.*

## VII · Multimodal (one bus, every hand)

- [x] Pointer events end-to-end (mouse = touch = pen) on faders, knobs,
      nodes, panes. HTML5 DnD is banned (no touch support).
- [x] Keyboard: space play/stop, esc panic, z zen, ` debug — all real.
      *Test: dispatch each key, observe the effect.*
- [ ] Arrow-key nudge + Home/End on focused knobs/faders; visible focus
      order that follows the pane layout. *Test: keyboard-only session can
      play, patch, and mix.*
- [ ] Gamepad → bus (see IV). MIDI-in as a stretch.
- [ ] Real-device touch pass on iPad + Android Chromium.
      *Test: all gesture suites pass on touch emulation at minimum.*

## VIII · UI quality (dense, legible, rock solid)

- [~] **The pedalboard look**: every pane is a digital stompbox — a bold
      station-hue enclosure per module (color IS wayfinding), chunky
      controls, LED truth, crisp digital surfaces (LCDs, code fields);
      efficient and flat where physical metaphor doesn't pay.
      *Test: each pane identifiable by color at a glance from across the
      room; screenshot sweep.*

- [x] Full-res spectrum: 96 log-spaced Goertzel bands as one lit curve.
- [~] Dense + straightforward: compact panes, smaller controls, everything
      visible without hunting. *Remaining: inline numeric entry on every
      control; per-pane compact/expanded modes; clearer grouping labels.*
- [ ] Robustness: pointer capture on ALL drags, Escape cancels any
      in-flight gesture, hit targets ≥ 24px, aria labels audited.
      *Test: gesture-interrupt suite (drag + Escape, drag off-window).*
- [ ] No layout shift while performing; panes never jump under the hand.
      *Test: CLS ≈ 0 during interaction recording.*

## IX · The canon (aesthetics are load-bearing)

- [x] Hermetic formalism: midnight purple, eight stations, Redaction +
      Iosevka, machined rack, sigil, theorem voice. Words over glyphs.
- [ ] The whiteboard/parchment twin theme (light mode of the grimoire).
- [ ] Colorized math variables in prose (sacred, from canon round 1).
- [ ] Texture pass round 2 (noise/scanline/vignette tuning — "we will
      come back to these").

## X · The site around the instrument

- [ ] `/blog` and `/info` exist (secondary to /phazor, same canon).
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
