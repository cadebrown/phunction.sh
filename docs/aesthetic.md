# The aesthetic canon — HERMETIC FORMALISM

Decided with Cade in a five-round process (2026-07-15); the live record of
rejected candidates is `/design`. If a change can't be justified from this
document, it doesn't ship. Sibling context: we perform alongside voidstar
(cosmic, void, ambient); phunction is the complementary pole — mathematical,
luminous, precise. Contrast is the act.

## Character (round 1)

**The professor IS the wizard.** Claims are made in theorem register and
meant rigorously; the artifacts carrying them are ritual objects. Neither
mode is a costume for the other.

## The one color rule (round 2)

**Color is phase** (hue = arg(z), domain-coloring convention), and the wheel
has **eight named stations** — the 8th roots of unity rotated 10°:

| station | angle | owns |
|---|---|---|
| `--blood` | 10° | danger, panic, peak clipping, warnings |
| `--rust` | 55° | — (reserved) |
| `--phosphor` | 100° | power, primary action, values, the QED ∎ |
| `--ichor` | 145° | success/confirmation, power hover |
| `--verdigris` | 190° | meters (rms base) |
| `--aether` | 235° | links, the lab, cool/visual things |
| `--sigil` | 280° | the mark, identity accents |
| `--philtre` | 325° | phazor, hot/audio things, hover shift |

Any other hue must be a *stated* angle (sequencer step i = 10° + i·22.5°;
the phasor hero's hue = its argument + 10°). Grep `oklch(` — every hit
says its angle. Ground: **midnight purple** `#1a1428` / `#221830` /
`#382e4a`, ink `#f4f2fb`, annotation `#b3accc`, faint `#6e6790`.

## Type (round 3)

- **Redaction 20** — display: wordmark, page titles, fig names. The
  deteriorating print IS the brand texture.
- **Redaction 50** — statements & prose (theorem blocks, descriptions).
- **Iosevka** — the shell: all UI chrome, buttons, captions, data
  (`tabular-nums` for numbers).
- **Computer Modern (KaTeX cuts)** — *rendered mathematics only*. The
  wizard writes; the math typesets itself. Never for UI or prose.
- IM Fell English stays in the stack as fallback ink. Never introduce
  another family.

## Voice

Theorem register, played straight: "Theorem (phunction). … Proof. Press
power. ∎" Sections are captioned figures (`fig. n`). Brand is `~/phunction`.
Semantic glyphs only (∎ proof-end, ∄ 404, ∿ phazor, ℂ lab, φ the heart).
No decorative emoji, no exclamation marks.

## Texture (round 2 — intensities provisional, tuning round reserved)

Noise (fractal grain, 10%, overlay), scanlines (13%), vignette (radial to
78% at edges) — on everything **except** `/lab/:id`: the lab viewer sits
above the texture layers (z-order) so shaders arrive unfiltered.

## Signature & motion (round 4)

- **fig. 0, the phasor hero** owns the landing page: rotating unit vector
  tracing its sine, hue = argument.
- **The sigil is the mark**: 64-tick graduation ring, wonky heptagram
  {7/3}, eight station dots at their own angles, φ still at the heart
  (`sigil.rs`). Topbar, favicon, 404, future loading states. Layers
  counter-rotate on a very slow clock.
- Motion budget: instruments move + ambient life (wordmark breathes at
  11s, cards lift-and-tilt on hover, sigil spins). Nothing bounces.
  `prefers-reduced-motion` stills everything.

## Planned, decided, not yet built

- **Whiteboard twin**: warm parchment grimoire (round-2 decision) — its own
  round, re-derived from the same phase rule.
- **Colorized math variables** (sacred anchor): symbols sample the eight
  stations. Lands with real math content/KaTeX rendering.
- Texture tuning pass (Cade: "more extreme, and we will come back").

## Hard rules

1. Legible on a 1080p screenshare at the back of a venue.
2. Every hue is a named station or a stated angle.
3. New pages open with their most characteristic element.
4. The lab's fullscreen shaders are never post-processed by site chrome.

## The rack (component language — added with Cade 2026-07-15)

UI controls are **machined modular-synth hardware, wired for real**:
skeuomorphism is only allowed when the control genuinely does the thing it
depicts. A `Knob` sends `Command::SetParam` down the ring; an `LedMeter`
renders `MeterFrame` telemetry; a `Led` is a signal, never a decoration.
Components live in `crates/phunction-app/src/rack.rs`.

Vocabulary: rack panels (brushed, engraved Iosevka titles, corner screws
with mismatched slot angles), ±135° knobs with station-hued value arcs,
LED ladders (ichor floor → phosphor shoulder → blood ceiling), machined
buttons with 2px travel, LCD readouts (phosphor on inset dark), key hints
as keycaps. Interaction: drag knobs vertically, shift = fine, double-click
= reset, wheel = nudge.

Stolen from voidstar's qualia with love: control density, visible
keyboard hints, the CLIP lamp, fps/beat/τ-style readouts. Deliberately
NOT stolen: their flat terminal chrome — our hardware is machined, theirs
is printed.
