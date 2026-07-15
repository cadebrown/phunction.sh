# VISION.md — the complete goals of phunction

> **Theorem (phunction).** *Any browser is a synthesizer; any screen, a
> canvas; any device in the room, part of the instrument.*
> **Proof.** Under construction, in public, forever. ∎

This file is the north star: the full, deliberately lofty statement of what
phunction.sh is becoming. Day-to-day maps live in `AGENTS.md` and the task
docs; this is the thing they all point at. When a decision is hard, pick
the branch that moves toward this document.

---

## 0 · The thesis

phunction is not a website about an artist. **The website is the artwork
and the instrument.** One URL that boots, on anything with a browser, into
a complete audiovisual performance environment — a DAW, a shader
laboratory, a 3D world, a typography playground, a VJ rig, a live-coding
surface — all one design language, all one control system, all one
mathematical story (*color is phase*), written head to toe in Rust and
given away whole. TouchDesigner's depth, FL Studio's joy, a grimoire's
soul, a proof's spine.

The measure of success: **someone walks on stage with nothing but a URL
and plays a full set** — sound, visuals, and lights — while the audience
opens the same URL and becomes part of the rig.

---

## I · phazor — the instrument (audio)

The hand-rolled browser DAW grows from a 16-step toy into a serious,
giggable instrument:

- **A real modular engine.** The patch cables stop being decoration: a
  modulation matrix where any source (LFOs, envelopes, sequencer lanes,
  audio analysis, pose, gamepad axes, other people's phones) patches into
  any destination — with the cables drawn, draggable, and saved. The rack
  UI *is* the signal graph.
- **Voices worth performing with.** Phasor-bank synthesis as the house
  timbre (stacked, detuned, FM'd, phase-distorted rotors — the oscillator
  is the brand), plus sampler voices, a drum rack, and per-voice effect
  inserts (delay, reverb, drive, the SVF family).
- **Sequencing that breathes.** Per-step note/velocity/gate/probability/
  ratchet editing, polymetric lanes, song sections, live loop capture, and
  swing that actually swings — all sample-accurate, all cycle-locked.
- **Projects as artifacts.** Full project save/load in OPFS; export stems
  and mixdowns (WAV free, Opus/AAC via WebCodecs); **a patch is a URL** —
  every instrument state shareable as a link, every link a playable thing.
- **The browser is not an excuse.** Latency budgeted like hardware
  (2.67 ms quanta, never missed), a wasm bundle a phone can love, and the
  engine testable/benchable natively down to the sample.

## II · The lab — the eye (visuals)

- **A phunctor for every mathematical obsession**: domain colorings,
  flows, reaction-diffusion, strange attractors, lattices, ray-marched
  3D worlds, particle fields, cymatics — each one URL, chromeless,
  projector-ready, mathematically honest enough to caption.
- **Everything audio-reactive.** The engine's analysis (bands, beats,
  envelopes) is a first-class modulation source for every visual: press
  play in phazor and the entire site dances — substrate included.
- **Live-codable shaders.** In-browser WGSL editing with naga validation,
  hot-swap without dropping a frame, errors as legible as the art. The
  shader editor is a stage instrument, not a dev tool.
- **3D worlds.** A wgpu scene module — models, cameras, lights, fog —
  driven by the same bus, so a scene can be *played*: orbit with a
  gamepad stick, strobe the sun with a kick drum.
- **Text as material.** Redaction and Computer Modern as instruments:
  scramblers, phase-colored glyph fields, kinetic theorem-setting, lyrics
  engines — typography that performs.
- **Video as clay.** Clips and camera feeds pulled through the shader
  pipeline as textures — feedback, datamosh-adjacent treatments, all
  recordable back out.

## III · The toolkit — one language for all of it

- **phunction-ui**: our own component system, machined and gritty (the
  rack), where *everything is compact, collapsible, dockable, and wired
  for real* — knobs, faders, XY pads, LED ladders, LCD strips, jacks,
  cables, node canvases, timeline strips, tab docks. No component that
  merely depicts; every control carries signal.
- **The network view.** A TouchDesigner-style patch canvas as an
  alternative projection of the same state the rack shows — two views,
  one truth, switchable mid-performance.
- **/studio**: the workspace where all module families dock into one
  performance surface — layout savable, shareable, foldable to exactly
  what tonight's set needs.
- **Playground-driven development.** Every primitive earns its place on a
  public playground page, screenshot-iterated against the canon before it
  ships. The design lab (/design) stays public: rejected candidates are
  part of the record.

## IV · Control — every device in the room

- **Total input parity**: keyboard/trackpad, touch (iPad, Android Chrome,
  iOS as it allows), **gamepads and controllers**, MIDI hardware
  (Safari excepted, gracefully), and eventually OSC bridges — all routed
  through one control bus where *a gamepad axis, a finger, and a knob are
  the same signal*.
- **Mappable everything.** MIDI-learn-style assignment for any control
  from any device, saved with the project. Key hints visible everywhere;
  the whole rig drivable eyes-closed from muscle memory.
- **The audience as controller** (the entanglement arc): phones join a
  set via QR, and their touches, tilts, and taps become sanctioned,
  rate-limited modulation sources. A crowd is a MIDI controller with a
  thousand knobs.

## V · The stage — performance is the product

- **Screenshare-native**: every surface legible at 1080p from the back of
  a venue; zen modes everywhere; blackout that keeps audio running.
- **Duet-ready**: built to gig alongside voidstar.sh — complementary
  aesthetics, compatible clocks; someday, synced transports across
  machines (two browsers, one beat).
- **Record the ritual**: capture performances (canvas + engine audio) to
  files worth keeping; export loops, clips, and posters of the night.
- **Set management**: setlists, scene snapshots, and one-tap transitions
  between full workspace states, mid-song, without fear.

## VI · The codebase — radical art software

- **Source as exhibit.** The repository reads like the site feels:
  documented invariants, theorem-style comments, benchmarks as proofs,
  the design canon versioned beside the code. Reading it should teach
  someone real DSP, real graphics, and real Rust.
- **Ship the debugger, always**: tracing consoles, param inspectors, perf
  HUDs, and diagnostics live in production, because the audience deserves
  the wiring diagram.
- **Agent-native forever**: any capable human or AI collaborator can open
  AGENTS.md and contribute within the canon on the first try.
- **Quality as identity**: `just check` green is the floor; native tests
  and benches guard the sample-accurate core; every visual change faces a
  screenshot critique before it ships.

## VII · The moonshots (the ∞-horizon)

- **phunction OS**: the whole site as one bootable performance
  environment — workspace manager, patch storage, controller registry —
  the feeling of a hardware instrument that happens to be a URL.
- **The networked ensemble**: multiple performers, multiple cities, one
  transport — browsers phase-locked over the network, latency treated as
  a compositional parameter instead of an enemy.
- **The teaching instrument**: every module doubles as a lesson — hover a
  filter and see its transfer function; the DAW as the world's most
  playable DSP textbook.
- **A physical echo**: the rack design realized as an actual Eurorack
  faceplate; the site's controls and a hardware module answering to the
  same patch file.
- **One thousand phunctors.** Not a metaphor. A living catalogue of
  mathematical visuals, each one honest, each one performable, built over
  years, numbered like figures in the longest paper ever written.

---

*Written under a waxing moon. Every hue states its angle. The demo never
ends.* ⌬
