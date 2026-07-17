# Design ethos

> Status: **baseline** — this documents the system as it stands today.
> A deliberate direction decision is in review (see
> `docs/decisions/0001-design-direction.md`); when it lands, this file
> becomes the canonical statement of that direction and every UI change
> is tested against it.

## The one law that already holds

**The field is the subject; the chrome is an instrument panel over it.**
Everything below serves that: panes boot folded to latch rails, glass is
translucent (alpha ~0.5, blur 18), the canvas paints in the
below-content bucket, zen strips everything.

## Current tokens (as built)

- **Palette — color is phase**: the canon wheel is a 3-tone cosine-lobe
  blend of midnight indigo `(0.10, 0.08, 0.26)`, violet
  `(0.46, 0.19, 0.54)`, deep teal `(0.08, 0.42, 0.48)` (see
  `shaders/prelude.wgsl`). No full-spectrum rainbow anywhere. UI hues
  are OKLCH picks off the same wheel; station hue = wayfinding
  (`--enclosure` per panel).
- **Type**: terminal mono (`--term`) for all chrome; KaTeX math italic
  (`.mvar`) for colorized math variables; uppercase + letterspacing for
  latches and titles.
- **Layout**: fixed fullscreen app, no document scroll. One dense topbar
  (transport · tempo · minds · worlds · zen), latch rails left/right
  (252px), sequence + patchbay strips bottom-center. Controls sit on a
  strict grid (span-by-default, controls take cells).
- **Depth**: glass panes over the live canvas — never opaque boxes.
- **Voice**: the machine speaks in theorem voice ("the room obeys");
  errors are addressed, specific, unapologetic.

## Component language

Knob · Fader · RackPanel(latch, lamp, enclosure hue) · Step pad
(note name + velocity bar) · LCD readout · LedMeter · ctrl-btn ·
patchbay node/cable. All pointer-events-first, keyboard-reachable,
44px-class hit targets on touch surfaces.

## Record

UI eras are archived as dated screenshots in `docs/design/gallery/`.
Decisions that changed this file live in `docs/decisions/`.
