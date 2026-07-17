# 0001 · Frontend design direction

- **Date**: 2026-07-17
- **Status**: proposed (mockups under review)

## Context

The /phazor chrome evolved by accretion across rapid build rounds:
CRT-terminal v0 → pedalboard skeuomorphism → qualia glass panes →
DAW-station grid → field-first folded boot. Each step was right for its
round, but there is no single stated ethos a new change can be tested
against. Cade asked to rethink the ethos deliberately, with multiple
directions to choose from, explored as mockups before touching the app.

## Options considered

Four directions were drafted; three were selected for mockups (The Score
was cut at the selection step):

1. **Hermetic Formalism (deepened)** — continuity with the existing
   canon: notation-as-interface (λ/Q/β control labels, theorem-voice
   status line), engraved hairlines, sigils, phase-wheel color law.
   Lowest risk, most "us".
2. **Blueprint** — the chrome as a monochrome engineering drawing: title
   blocks, dimension ticks, drafting annotations, ink hairlines — the
   mind field is deliberately the only color on screen.
3. **Lab Hardware** — machined bezels, VFD segment-glow readouts, LED
   lamps, one amber accent. Most tactile; partially fights the
   field-first glass direction.
4. ~~The Score~~ — typographic minimal, controls as inline draggable
   numerals. Rejected at selection (most radical, highest execution
   risk).

Mockups live as private Claude artifacts (linked in the session record);
the process decision was **artifacts to choose, in-repo to maintain** —
no external design-system mirror.

## Decision

Pending Cade's review of the three mockups. The chosen direction will be
codified in `docs/design/ethos.md` and executed as a deliberate
restyling pass, not another accretion step.

## Consequences

- Until decided, no further ad-hoc aesthetic drift: UI changes stay
  functional.
- The rejected directions stay recorded here — they are legitimate
  future pivots, and the reasoning that cut them will matter then.
