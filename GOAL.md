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
- [x] `/phazor` is an **app, not a page**: no document scroll — compact
      pane columns over the field (qualia-style), panes fold, float, and
      dock; center stays open so the field shows. *Verified: body never
      scrolls, columns scroll internally, every control reachable; probe
      + screenshots in-session.*
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
- [x] More generative-geometry minds tuned for flow: current (curl-noise
      streaks) and petri (Gray-Scott reaction-diffusion on the NEW
      feedback render-target infra — FeedbackPhunctor ping-pongs two
      Rgba16Float states at 512×288, sim pass + present pass, state-based
      genesis so the dish can never die). Ten minds. *Verified live:
      coral-maze growth wall to wall; lenia proper can now be one more
      sim shader on the same infra.*
- [x] **Shocking geometry** ("all the geometries are too basic — i want
      shocking, crazy"): maw (pseudo-Kleinian cavity, Knighty fold — the
      camera glides INSIDE the set on a closed bounded loop, orbit-trap
      paints the vaulting) and bulb (power-morphing Mandelbulb 6→10 with
      orbit-trap veins, key+rim+AO and cheap soft shadows). Twelve minds.
      *Tested: both DEs verified natively first (scratchpad raymarch
      repro: 13/13 rays hit in ≤73 steps) — then verified live in canon
      palette; screenshots on record. The maw glide is bounded (±1.8
      sine) so hours-deep sessions can't march out of f32.*
- [x] **Three world-class research visuals** ("research really cool
      mathematical ideas and iterate until 3 world-class visuals"):
      **indra** — a Kleinian limit set on the Maskit slice (Jos Leys's
      fast algorithm: wrap+fold+invert; the tangent-circle necklace of
      Indra's Pearls, the parameter t walking Maskit space so the lace
      rewires); **hopf** — the Hopf fibration, S³ as a sphere of disjoint
      circles, each fiber stereographically projected and ray-marched as
      a glowing tube so the nested Villarceau tori braid as the base
      sphere precesses; **lenia** — Bert Chan's continuous cellular
      automaton on the feedback infra, tuned to the SPOT regime (soft
      ciliated amoebas, not petri's connected coral), self-sustaining
      forever. Fifteen minds. *Tested: every formula verified natively
      first (scratchpad: Kleinian set is thin+present at 4% of the strip;
      Hopf fibers are exact circles equidistant+coplanar to 1e-6; Lenia
      morphology swept for an alive spot-producing config — run-length ~2
      vs the labyrinth's long stripes). Then iterated in-browser from
      screenshots (indra reframed to a horizontal necklace, hopf rebuilt
      from sparse dots to marched glow + brightened, lenia moved from
      labyrinth → grain → soft amoebas via kernel spacing). FPS measured:
      indra/lenia 120, hopf 107 (cut to 40 steps × 24 fibers for
      headroom). Playwright guards all three against shader-compile
      breakage.*

- [x] **The weather owns more of the music** ("constantly evolving and
      changing tones, timbre, notes, tempo"): every 64-beat era now sets
      an EraWeather — modal interchange (~1/3 of eras borrow a sibling
      minor mode), tempo breathing (±4% around the user's BPM, stepped at
      era edges where the delay's tape-glide absorbs the retune), and
      timbre/space walks (brightness bias, cutoff/reverb/feedback
      multipliers) that GLIDE over ~4s. All pure functions of the era
      seed; user CVs stay the base truth. *Tested: weather bounded +
      deterministic over 2000 seeds; 12-era traversal changes mode ≥2×
      and spreads timbre; engine test proves effective BPM drifts within
      ±6% and never leaves the band; storm suite still green.*

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
- [x] Field-typed routes into the room: the board carries typed Values;
      `cam = camera-in / cam -> mind.field` compiles (Field-checked),
      routes on the board, and takes the room onto the camera pipeline
      while the cable holds. The FULL fx space is patchable: echo, regen,
      wash, size, cutoff — every send/space parameter is a route target.
      *Tested: graph tests + live round-trip incl. render().*
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

- [x] **Automated UI-functionality testing, CI + local** (stack chosen
      after verifying the 2026 landscape: Playwright — faster and lighter
      in CI than Cypress, first-class traces, honest pointer gestures):
      `tests/ui/phazor.spec.mjs` covers boot+clock, all three step-editor
      gestures, v3 pattern reload, topbar mind lighting + persistence,
      zen enter/exit, and transport stop/start — 6 specs, ~11s, against
      the same COOP/COEP dist server CI uses. `just ui-test` locally; the
      smoke CI job runs it with traces uploaded on failure.
      *The suite IS the test.*

## VII · Multimodal (one bus, every hand)

- [x] Pointer events end-to-end (mouse = touch = pen) on faders, knobs,
      nodes, panes. HTML5 DnD is banned (no touch support).
- [x] Keyboard: space play/stop, esc panic, z zen, ` debug — all real.
      *Test: dispatch each key, observe the effect.*
- [x] Arrow-key nudge + Home/End on focused knobs/faders (verified: tempo
      nudged by keys), and the focus-order audit passes: tab order walks
      nav → left column → mid → right column with zero pane interleaving,
      so a keyboard-only session can play (space/arrows), patch (the code
      drawer + run), and mix (arrows + inline entry). *Verified by
      DOM-order probe.*
- [x] Gamepad → bus (see IV). MIDI-in landed too: last note/velocity/
      mod-wheel as a media block over Ctx.ext[4..7], requested on first
      node use (Web MIDI, no sysex). *Verified: shelf spawn; hardware
      drive pending a physical controller session.*
- [x] Touch pass: the written acceptance bar — "all gesture suites pass
      on touch emulation at minimum" — is met: fader drag 0.65→0.81,
      panel float, node drag 26→116px, knob turn, all pointerType:'touch'.
      (Real-device session: see Awaiting Cade.)

## VIII · UI quality (dense, legible, rock solid)

- [x] **The pedalboard look**: every pane is a digital stompbox — station
      hue enclosure band, tinted title, status lamp per module (color IS
      wayfinding), crisp digital surfaces. *Verified: screenshot sweep
      shows each pane identifiable by color at a glance.*

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
- [x] **The DAW station** ("we want 'daw station UI' not random BS"):
      glass panes — every pane is translucent over the living mind field
      (the background IS the visuals, blur(14px) glass, verified alpha
      0.72 computed live); ornament stripped on /phazor (screws, rack
      rails, CV socket — a workstation, not a prop); every panel body is
      a strict CSS grid (auto-fill 56px cells, controls take cells, wide
      surfaces span — voice panel measured 235→113px). *Test: computed
      panel background alpha < 1 + grid template on .rack-body + height
      measurements on record.*
- [x] **A sequencer for actual music** (engine already spoke note+vel;
      the UI was the toy): tap toggles, vertical drag walks SCALE DEGREES
      (7px per degree, snapped via Scale::snap/degree_step in phazor-core
      — native-tested: snap lands on scale tones only, walks are exact,
      saturates at the musical range), shift-drag writes velocity, steps
      display note names + velocity bars. State v3 persists note:vel per
      step (v2 bit-patterns still restore over the old riff). *Verified
      live: tap→a3 on, 21px drag→d4 (+3 degrees), shift-drag 108→84,
      reload renders the edited pattern back.*
- [x] **Push isn't done until CI is green**: `just ci-watch` blocks on
      the ship run for HEAD and dumps failing logs on red; AGENTS.md
      makes it part of the push protocol. Root-caused the actual CI
      failure it would have caught: smoke's persist check raced the ~1s
      patch autosave on fresh CI profiles — it now polls (the contract
      is eventual persistence, 2s clock stamps by design).
- [x] **The qualia topbar** (total UI rehaul, voidstar-style): one fixed
      dense strip owns the chrome — title row (wordmark + beat/vox/drop
      LCDs + zen corner) over a wrapping controls row (▶ ■ ✕ transport,
      ∓4 bpm steppers with LCD, all twelve minds, all worlds) — replacing
      three whole panels. The transmission marquee yields to the app
      (`body:has(.panel)`), zen slides the strip away qualia-style and
      the floating handle remains as the exit. *Verified live: strip
      renders at 58px, play/mind lit-states track the engine, marquee
      display:none on /phazor; smoke's beat-readout contract still green.*
- [x] **No invisible UI**: restored float positions are clamped into the
      current viewport (a panel dragged wide on a big window can't come
      back off-screen on a laptop), panel glass is near-opaque, titles
      and control labels brightened. Density v2+v3: 30px knobs, 22×68
      faders, slim latches, capped step pads, 252px columns. *Test:
      stored_float clamps to innerWidth/Height margins by construction;
      panel height 232→198px measured.*

- [x] **Field-first boot + modular chrome** ("simplify — brutally
      efficient to use live; I can't see the background"): every pane
      boots FOLDED to a slim latch rail, so a fresh session is the field
      wall-to-wall with one dense topbar — every surface is one tap away
      and 1/2/3 still snap whole layouts. Glass lightened (alpha 0.5,
      blur 18) so open panes read the field through themselves. The
      chrome is now clean modules: topbar.rs (the strip) and presets.rs
      (worlds) extracted from the phazor_panel monolith. Review catches
      fixed: the fold gate (.rack-body.hidden) had been silently
      overridden by the DAW-grid display rule since it landed — every
      fresh boot rendered all panels OPEN; layout presets referenced
      retired panels (transport/mind); the zen keyboard branch duplicated
      toggle_zen inline; a CSS-hidden decorative jack still rendered.
      *Tested: fresh-profile probe shows all panels folded at t=3s AND
      t=8s with the field visible wall-to-wall; the Playwright suite
      (fixed to guard on VISIBILITY, not element count — folded bodies
      are rendered-but-hidden) is 7/7; full check green.*

## IX · The canon (aesthetics are load-bearing)

- [x] Hermetic formalism: midnight purple, eight stations, Redaction +
      Iosevka, machined rack, sigil, theorem voice. Words over glyphs.
- [x] The parchment twin (canon round 1's grimoire light mode): 'ink' in
      the nav flips the prose surfaces to warm parchment — fell serif
      theorem, inked phasor figure, darkened station hues on the identity,
      sepia sigil — persisted, verified both directions. The stage
      (/phazor) stays night by design: lit gear in a dark room.
- [x] Colorized math variables (sacred, canon round 1): the phasor
      identity on the hero — z(t) = A·e^{i(ωt+φ)} in KaTeX math italic,
      every variable in its station hue, above the live figure it
      describes. The .mvar vocabulary is reusable in any prose.
- [x] Texture pass round 2, executed: grain 0.10→0.07 finer, scanlines
      0.09 alpha at 4px pitch, a true corner vignette (parchment gets its
      own sepia variant), mind-field saturation lift. The three dials are
      one CSS block with a comment naming them. (Taste sign-off: see
      Awaiting Cade.)

## X · The site around the instrument

- [x] `/blog` and `/info` exist (secondary to /phazor, same canon —
      theorem voice, honest about how early they are).
- [x] Production deploys on every push to main (Workers static assets,
      COOP/COEP correct). *Test: CI green, phunction.sh serves the build.*

---

## Awaiting Cade (verification sessions only — the work is shipped)

- **Real-device touch session** (iPad + Android Chromium in hand): the
  emulated suite is green; run the same gestures on glass and file
  anything that feels wrong as new checklist items.
- **Texture taste sign-off**: the three dials (grain opacity, scanline
  alpha/pitch, vignette strength) are labeled in one CSS block — nudge
  by eye, or bless as-is.

## Horizon (architecture epics — roadmap, not checklist)

These are real, wanted, and deliberately NOT checklist items, because no
honest acceptance test exists inside the current architecture:

- **Fields through arbitrary minds**: today a routed field takes the room
  via the field pipeline; blending fields INTO citadel/silk/etc. needs a
  compositor pass (every mind samples an optional field texture). Design:
  add FIELD_BINDINGS to the shared prelude path and a null-texture default.
- **Multi-bus audio topology**: the engine mixes three layers into one
  master by design; patchable audio-bus routing (bus → fx send as a
  CABLE) needs the engine to grow addressable buses. Until then, every
  send/space LEVEL is patchable (fx.echo/regen/wash/size), which is the
  musical surface of the same idea.
- **Lenia proper**: one more sim shader on the feedback infra (petri
  proves the plumbing); worth doing with Cade picking growth kernels.

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
