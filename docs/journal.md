# Engineering journal

A running prose record of how phunction.sh/phazor actually got built —
the narrative layer above the git log. Newest entries first. Candidate
seed material for /blog posts about the process.

---

## 2026-07-17 · The review round, and the fold bug that hid in plain sight

A quality review turned up the best kind of bug: the fold gate
(`.rack-body.hidden { display: none }`) had been silently defeated for
days by a later equal-specificity rule (`.panel .rack-body
{ display: grid }`). Every "fresh boot" since the DAW-grid landed had
rendered every panel open — which is exactly why the field felt buried.
The test suite masked it by guarding on element *count* (folded bodies
are rendered-but-hidden). Lessons now encoded: display gates get
`!important`; tests assert visibility, not existence; debug probes get
unique Chrome profile dirs (a `pkill` without `-f` left a locked
browser lying to us for an hour).

Same round: every pane now boots folded (field-first), and the chrome
became real modules — `topbar.rs`, `presets.rs` — instead of a
1,200-line panel monolith.

## 2026-07-16 · Three research visuals, verified before rendered

The goal: three world-class background visuals from real mathematics.
The discipline that worked: verify every formula natively before writing
a line of WGSL. The Kleinian limit set (Maskit slice, Jos Leys's fast
algorithm — extracted from his paper's PDF streams) was checked as
thin-and-present on a native raymarch; the Hopf fibration's projected
fibers were proven exact circles (equidistant + coplanar to 1e-6); and
Lenia's parameters were *swept for morphology* — the stable band looks
like a labyrinth (indistinguishable from the existing Gray-Scott mind),
so a run-length metric found the spot regime where discrete soft
amoebas live. Then each was iterated from actual screenshots: hopf went
from sparse dots to marched glow; lenia from grain to fat ciliated
cells (kernel *spacing*, not radius, sets creature size); indra got
rotated horizontal because widescreen wants its necklace flat.

## 2026-07-16 · The music learned weather; pushes learned to check on themselves

Every 64-beat era now rolls modal interchange, ±4% tempo breathing, and
timbre/space walks — all pure functions of the era seed, gliding so
nothing steps. And after a push failed silently in CI (a smoke-test
race that only fresh CI profiles could reproduce), `just ci-watch`
became part of the push protocol: a push isn't done until the ship run
is green.

## 2026-07-16 · The sequencer grew up; the UI became a DAW station

The engine had spoken `Step { note, vel }` all along — only the UI was
a toy. Steps now tap to toggle, drag vertically through *scale degrees*
(snapped, so edits can't leave the harmony), shift-drag for velocity.
The chrome went glass-over-field with a strict control grid, ornament
(screws, rails, fake jacks) deleted rather than styled.

## Earlier

The deep history — engine invariants (the storm test that caught four
real click bugs), the pane era, the patch language, live WGSL, the
canon palette — is in the git log and `GOAL.md`'s acceptance trail.
This journal starts where it starts; the log knows the rest.
