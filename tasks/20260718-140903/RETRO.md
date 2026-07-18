# Retro: live LOW<->HIGH switching rendered at stale resolution

- TASK: 20260718-140903
- BRANCH: fix/render-scale-switch (squash-landed as master 099f14e8)
- REVIEW ROUNDS: 1 (APPROVE)

## What went well

- **Reproduced before fixing.** Added a `NOVA_SWITCH_QUALITY` example mode that
  flips the preset mid-run and captured the post-switch frame. The before shots
  showed the exact inversion the user described; the after shots proved the fix.
  Reproducing first turned a confusing "it's laggy/inverted" report into a
  concrete, checkable artifact.
- **Root-caused in bevy source, not by guessing.** Read `camera_system`
  (bevy_render `camera.rs`) and found the recompute condition ignores
  `RenderTarget`-component swaps - so the one-line-ish fix (`projection.set_changed()`
  on every switch) was targeted, not a shotgun.
- **A regression test that pins the mechanism** (projection marked changed on a
  switch, untouched on a steady frame), so this can't silently regress.

## What went wrong

- **I verified fresh-start states, never transitions.** Both render-scale bugs
  the user hit (clicks, then switching) were in state I never exercised: my
  screenshots always booted directly into Low or High, so a fresh Low looked
  perfect while a Low reached by *switching* was broken. Root cause: I treated
  "captures at a preset look right" as "the preset works", when the preset is a
  live setting whose whole point is being toggled at runtime. The camera
  target-swap bug was invisible to a fresh-start capture because fresh start
  hits bevy's `is_added()` recompute path by luck.

## What to improve next time

- For a runtime-toggleable feature, test the TRANSITION (A->B and B->A at
  runtime), not just each fresh state. Fresh-start and switched-into states can
  differ (here, whether bevy re-derives the camera target).
- Treat "bevy reacts to component X changing" as a claim to verify in source,
  not assume - `camera_system` reacts to `Projection`/`is_added`/target-content,
  but NOT to a `RenderTarget` swap.

## Action items

- [x] Regression test + a reusable `NOVA_SWITCH_QUALITY` repro mode in the
  example.
- [x] Recorded the bevy gotcha in the design log and the lessons ledger.
