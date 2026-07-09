# Review: Camera twitch when flying (fixed-tick stair-step)

- TASK: 20260709-160753
- BRANCH: fix/camera-twitch-interpolation

## Round 1

- VERDICT: APPROVE (no BLOCKER/MAJOR; the MINORs were addressed on the branch
  before merge - see Responses)

Independent pass verified the physics story against both crate sources: the
interpolation plugin is in PhysicsPlugins by default with per-body opt-in
(the diagnosis); easing runs in RunFixedMainLoop before Update on every
render frame; FixedFirst snap-back prevents any eased-pose leak into the
simulation (no double-write fight with avian's writeback, no spawn-frame
artifact); the camera chain reads `&Transform` in Update (fresh eased pose,
not stale GlobalTransform), so the "no frame lag" correction is genuinely
verified; flight/autopilot read raw avian Position/Rotation in FixedUpdate;
integrity graph, blast overlap, HUD projections and both examples'
assertions are unaffected or consistently eased.

- [x] R1.1 (MINOR) docs/2026-07-09-camera-twitch-interpolation.md - "Physics
  is untouched" oversells: projectile spawn points (both
  shoot_spawn_projectile systems compute the muzzle from current Transforms
  in Update) and the torpedo arming/fuze/PN loop now operate on the eased
  pose, up to one tick behind raw physics. Internally consistent and arguably
  better (matches what the player sees), but it deserves a sentence.
  - Response: fixed - the doc now states the eased-pose consumers explicitly
    and why that is intentional.
- [x] R1.2 (MINOR) tasks/20260709-160753 step 3 promised an app-level test
  that the Transform advances on render frames between fixed ticks; only
  presence checks were delivered, and the behavioral test is feasible (manual
  4ms frames, count distinct translations across consecutive updates). An
  avian upgrade changing the interpolation default would keep the presence
  checks green while the twitch returns.
  - Response: fixed - `scenario_bodies_move_between_fixed_ticks` (actions.rs)
    steps 4ms render frames against the 15.6ms fixed tick and asserts the
    translation advances on every frame (4 distinct positions across 4
    frames), which only easing can produce.
- [x] R1.3 (NIT) actions.rs comment hardcodes "64 Hz"; true only by default.
  - Response: fixed - phrased as "the fixed timestep (64 Hz by default)".
- [ ] R1.4 (NIT) eased bodies mark Transform changed every frame even at
  rest, so propagation re-runs per scenario-body subtree; negligible at
  current scene sizes, escape hatch (per-entity opt-out) already documented.
  - Response: acknowledged, no change - revisit only if asteroid fields grow
    by orders of magnitude.
- [ ] R1.5 (NIT) the bundle presence test and the smoke assertion are
  redundant with each other.
  - Response: keeping both - they guard different seams (the shared bundle
    vs the live spawned ship), and the new behavioral test carries the real
    weight.

## Round 2

- VERDICT: APPROVE

Verified the fixes: the behavioral test was proven against the bug - with
`TransformInterpolation` commented out of the bundle it fails with the exact
stair-step ([0.3125, 0.3125, 0.46875, 0.46875]: repeated poses on non-tick
frames, one full 15.625 ms tick of motion per jump), and passes with it
restored; the doc sentence matches the Update-schedule reader list from
round 1; the comment phrasing is corrected. No new findings.
