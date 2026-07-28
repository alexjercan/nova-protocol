# RETRO: recenter the ship-app orbit camera on the selected section

## What went well

- The plan was code-accurate before sprouting: `ShipSections::collect()` already
  yielded `view.local.translation`, `ship_input` already funnelled every selection
  path through `runtime.selected`, and `drive_ship_camera` already built the eye
  from `center`. The three-field `ShipOrbit` extension + one reconcile block was
  all the change needed - no restructuring mid-flow.
- The DECISION.md fork (T-reset returns home AND consumes the selection so the
  reframe sticks) was the load-bearing choice, and pinning it BEFORE building
  meant the `centered_on`-gated reconcile fell out cleanly. The reviewer singled
  out the stick behavior as coherent, and the `ship_reset_reframes_whole_ship_and_sticks`
  test proves it (re-runs `ship_input` with the section still selected).
- Tests were written to fail if the wiring reverted: test 1's post-`drive_frames`
  assertions (`center ~ turret_local` AND `distance(centroid) > 1.0`) fail if the
  ease is a no-op; the `center_target` assertion fails if the reconcile is gone.
  This is exactly `test-the-wiring-system-not-just-its-pure-helpers`.

## What went wrong / what to improve

- The off-origin fixture's "world frame" was faked: the root got a Y rotation, but
  each section's `GlobalTransform` was hand-set to `local + constant_offset`, a
  pure translation that ignored the rotation. The local-vs-world distinction the
  test needed was still real (the target lands on the local `(9,2,-1)`), but the
  root rotation was dead dressing - an inconsistent world pose that could mislead
  a future reader into thinking the world transforms were properly composed. The
  reviewer caught it; fixed by composing `root_world * local`. Lesson below.

## Lessons

- `offorigin-fixture-compose-world-not-fake-it` (new): when a fixture spawns a
  root off-origin AND rotated to distinguish the local frame from the world frame,
  set each child's `GlobalTransform` to the genuinely composed `root_world * local`,
  not `local + constant_offset`. A hand-faked additive offset ignores the root's
  rotation, so the "world" pose is just local shifted by a constant and the root
  rotation is dead dressing that misleads. Sharpens
  [[spatial-fixture-off-the-trivial-point]].
