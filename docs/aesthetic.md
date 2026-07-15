# The aesthetic canon — *color is phase*

This is the story every visual decision on phunction.sh derives from. If a
change can't be justified from here, it doesn't ship. (Sibling doc for
voidstar-style band context: we perform alongside voidstar — cosmic, void,
ambient. phunction is the complementary pole: mathematical, luminous,
precise. Don't drift toward their look; contrast is the act.)

## The one rule

**Color is phase.** phunction's world is the complex plane: the DAW is named
for e^{iωt}, the visuals are domain-colored fields where hue literally
encodes arg(z). So the palette is not a set of picked colors — it is the
OKLCH hue wheel, read as phase:

| token | value | role |
|---|---|---|
| `--plane` | `#0a0812` | the background: the plane at \|z\| → ∞. violet-black, never pure black in UI (pure black is reserved for lab fullscreen) |
| `--ink` | `#ece5d8` | chalk-warm paper white. body text, wordmark |
| `--annot` | `#8f8878` | marginalia: captions, labels, hints |
| `--w0` | `oklch(80% 0.15 85)` | **ω⁰ amber** — power, tempo, values, the QED |
| `--w1` | `oklch(78% 0.12 215)` | **ω¹ cyan** — links, the lab, cool/visual things |
| `--w2` | `oklch(72% 0.19 345)` | **ω² magenta** — phazor, hot/audio things, warnings, hover |

The three accents are the cube roots of unity on the hue wheel (85°, 215°,
345° — equally spaced). Need a fourth accent? You don't. Need a continuous
range? Use the wheel itself: `oklch(L C <angle>)` where the angle *means*
something (the phasor hero's hue = its argument; step i of the sequencer =
85° + i·22.5°). Gradients are arcs of the wheel (`in oklch longer hue`),
never arbitrary color pairs.

## Type: the paper and the shell

The project's tension is a mathematician on a stage with a terminal. The
typography stages it:

- **Paper** (KaTeX Main = Computer Modern, Knuth's mathematics face):
  wordmark (*italic*), headings (*italic*), theorem copy, figure names.
  Used with restraint — it is the voice of statements.
- **Iosevka** (the shell): everything else — body, UI chrome, buttons,
  captions, data. Numbers in UI get `font-variant-numeric: tabular-nums`.
- **PaperMath** (KaTeX Math Italic): single math glyphs when they carry
  meaning (the ∄ of the 404).

Never introduce a third family. Never set UI chrome in Paper.

## Voice: theorem register, played straight

Copy states things the way papers do, with stage-dry confidence:
"Theorem (phunction). Any browser is a synthesizer. Proof: press power. ∎"
Content sections are *figures* with `fig. n` labels — because they are
figures. The topbar brand is `~/phunction` — the domain is a shell path.
Glyphs are semantic or absent: ∎ ends proofs, ∄ is the 404, ∿ is phazor,
ℂ is the lab. No decorative emoji, no exclamation marks.

## Signature and restraint

The one bold element is the **live phasor** on the landing page (fig. 0):
rotating unit vector, traced sine, hue = argument. Everything else stays
quiet: hairline borders (`--line`), flat surfaces, no glow effects, no
border-radius (the plane is ruled, not rounded), motion only where it
depicts something (the phasor rotates because phasors rotate; figures don't
bounce). `prefers-reduced-motion` freezes the phasor into a legible still.

The background grid is the graph paper of the old blackboard site — kept
faint (7% alpha). It is substrate, not decoration.

## Hard rules

1. Legible on a 1080p screenshare at the back of a venue. Contrast is a
   feature, not a mood.
2. Dark is canonical (projectors, stages, OLED). A light "whiteboard" theme
   may come later; it must re-derive from the same phase rule.
3. Every hue in the codebase is either a named token or a *stated* phase
   angle. Grep for `oklch(` — each hit should say what angle it is and why.
4. New pages open with their most characteristic element, not a header
   block.
