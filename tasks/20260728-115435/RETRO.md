# Retro: NOVA OS ship app - clearer section rendering

- TASK: 20260728-115435
- BRANCH: feature/ship-section-legibility
- REVIEW ROUNDS: 1 (APPROVE, 2 NITs both addressed)

See TASK.md Work Log for what changed and DECISION.md for the colour-channel
choice; this file is process only.

## What went well

- Surfaced the real fork BEFORE building: the task said "encode kind by hue
  and/or icon" while also "keep the phosphor palette", but colour already meant
  status - a monochrome palette cannot carry both. Put that constraint to the
  owner (AskUserQuestion) instead of guessing, got "keep it green, move status
  elsewhere", and recorded it in DECISION.md. Exactly the load-bearing-look fork
  the flow skill warns about.
- Verified the one genuinely unproven thing on a real GPU. `LineList` +
  `StandardMaterial` had no prior use in this repo; a headless entity-tree test
  proves the mesh/entities exist but NOT that lines draw. Running the existing
  `screenshot_nova_os` harness (exit 0, visible wireframe) retired that risk and
  doubled as the "green blob -> separated boxes" acceptance shot.
- Test-first on the pure helpers (`kind_glyph`, `bar_fraction`, `ammo_pips`) plus
  a live-tree test for the blip subtree caught wiring at the right altitude; the
  uniform-green regression pin genuinely fails on revert (block used to carry the
  status material) and used an off-origin fixture per
  `spatial-fixture-off-the-trivial-point`.

## What went wrong

- The screenshot verification range (`nova_os_range` in the harness) has only
  controller/hull/thruster - no weapon section - so the ammo pips were verified
  only by unit test + ECS tree, never on screen. Root cause: reached for the
  nearest existing harness fixture without checking it exercises every new visual
  variant. The pips are the one new element with no pixel-level confirmation.
- Minor churn: wrote an intra-doc link `[cuboid_edges]` in the module header
  (R1.1 fix) to a PRIVATE fn, immediately catching myself via
  `rustdoc-no-public-to-private-intra-doc-link` and downgrading to a code span.
  Cost nothing here but is the same trap that has cost a `cargo doc` warning
  twice before.

## What to improve next time

- When a change adds a rendering primitive/material combo the repo has not used
  before, plan the GPU screenshot as part of the /work verify step from the
  start, not as an afterthought - and pick or extend a capture fixture that
  contains one of every new visual variant (here: an armed ship, so the pips get
  a pixel check too).

## Action items

- [x] Recorded the load-bearing colour-channel choice in DECISION.md.
- [x] Ledger: added `new-render-primitive-verify-on-gpu`.
- Manual acceptance (owner playtest) still open - listed in TASK.md Work Log and
  REVIEW.md; the armed-ship pip visual is the notable gap. No new tatr task filed
  yet: the sibling inspector-panel task (20260728-115430) and the owner's next
  playtest will exercise armed ships; file a follow-up only if the pips read
  wrong then.
