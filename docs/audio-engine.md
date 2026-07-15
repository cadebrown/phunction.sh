# phazor — the audio engine

## Signal path (v0)

```
StepSequencer ──(frame-offset events)──▶ 16× Voice ──Σ──▶ master gain ──▶ tanh ──▶ out
                                            │
Voice = Phasor ×3 (1f, 2f, 3f phase-locked) → blend(brightness) → SVF lowpass → ADSR × velocity
```

- **Phasor** (`math.rs`): unit complex number rotated once per sample;
  sin/cos for free, one Newton renormalization step per sample keeps |z|=1
  forever. This is the sonic identity of the instrument — extend timbres by
  *stacking/detuning/FM-ing phasors* before reaching for wavetables.
- **SVF** (`voice.rs`): Simper trapezoidal state-variable filter; stable
  under modulation. Retune at block rate.
- **ADSR**: exponential-approach one-poles per stage; click-free by
  construction. Kill = hard stop (voice steal / panic).
- **Params** (`event.rs::ParamId`): flat exhaustive enum; every continuous
  param goes through a `Smoothed` one-pole (block-rate, 15 ms). The
  inspector enumerates `ParamId::ALL` — keep it exhaustive.

## The rules on the audio thread

`Engine::process` and below: **no alloc, no locks, no strings, no
unbounded work**. Sequencer event scratch is a `heapless::Vec` sized by
`MAX_EVENTS_PER_BLOCK`. If you need more headroom, change the constant and
justify it in the PR — don't switch to `Vec`.

Commands are applied at block start (live feel ≪ 3 ms). If an event must
land *within* the block (sequencer notes), it gets a frame offset and the
render loop splits at it. That's the pattern for everything future:
timestamped commands, not "apply when you see it".

## Threading bootstrap

See `docs/architecture.md` + `phazor-web/src/worklet.rs` (heavily
commented). The three traps, so you don't rediscover them:

1. **TextDecoder/TextEncoder don't exist in worklets.** The glue stubs
   them; our polyfill module must be imported *before* the glue (static
   imports evaluate depth-first) and its `decode()` must accept zero args
   (the glue does a warm-up call — this one cost an hour).
2. **Worklet errors are silent.** registration + constructor are wrapped in
   try/catch + `console.error`. Keep it that way.
3. **Trunk hashes the glue JS filename** — resolve it at runtime from the
   `link[rel=modulepreload]` tag; never hardcode paths into the blob.

## Where this goes next (agreed direction, not vaporware)

- Per-step note/velocity editing (UI) — engine already supports it.
- More voices/timbres: phasor FM, detuned supersaw-of-phasors; keep the
  phasor as the atom.
- WAV export (hound) in a worker; OPFS project storage; Web MIDI in
  (midir/web-sys — Safari has none, keyboard fallback stays).
- If DSP outgrows one thread: firewheel-web-audio-style multithreading is
  compatible with the current shared-memory design.
