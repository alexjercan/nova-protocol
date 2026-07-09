# AI aim and turret lock-on anchor at the ship root origin, not the live structure

- STATUS: OPEN
- PRIORITY: 60
- TAGS: v0.4.0,bug,ai,turret

Found during review of 20260709-140620 (chase camera phantom-pivot fix). The
camera was moved to the live center of mass, but other consumers still target
the ship ROOT ORIGIN - the build position of the first sections, which is empty
space once those sections are destroyed:

- AI ships aim and fly at `player_transform.translation`
  (crates/nova_gameplay/src/input/ai.rs:83/112/141/166): after the player loses
  the front sections, enemy fire converges on the empty build-spot instead of
  the surviving hull.
- The player's turret aim point and lock-on cone are origin-anchored
  (`transform.translation + forward * 100`, input/player.rs:117-145, and the
  origin of `pick_target` around input/player.rs:217): small parallax against
  the COM-anchored camera crosshair after losing sections.

## Steps

- [ ] Add a shared `live_structure_anchor(transform, Option<&ComputedCenterOfMass>)
      -> Vec3` helper in `crates/nova_gameplay/src/sections/mod.rs` (next to
      `SpaceshipRootMarker`): live COM lifted with rotation + translation only
      (the exact math of camera_controller.rs:297-300, which must not scale),
      falling back to the root translation for marker-only roots. Unit tests:
      lift matches rotation*com+translation, fallback without COM.
- [ ] Refactor `update_chase_camera_input` (camera_controller.rs) onto the
      helper so the COM-anchor math lives once; its existing anchor tests
      must stay green (run them - they are touched code).
- [ ] AI (input/ai.rs): all four player-position reads (rotation target,
      thruster alignment, turret target input, projectile fire alignment)
      query `Option<&ComputedCenterOfMass>` alongside the player transform
      and aim at the helper's anchor instead of `transform.translation`.
- [ ] Player (input/player.rs): the turret camera-ray base and the
      `pick_target` cone origin use the ship's anchor instead of the root
      translation (kills the parallax vs the COM-anchored crosshair).
- [ ] Behavioral tests: with a shifted COM on the world, AI turret input and
      the cone origin equal the anchor, not the origin (player.rs/ai.rs test
      modules, existing world-test patterns).
- [ ] Verify: cargo fmt, cargo check --workspace, run the new tests plus the
      touched camera-anchor tests (skip full suite per user instruction;
      report skips).

## Notes

- Origin consumers that are fine as-is: autopilot GOTO/navigation (meter-scale
  offsets do not matter) and HUD projections (they project target entities).
- Related: 20260709-140620 (camera anchored on live COM, this bug's sibling).

Update (20260709): this bug seeded the component-lock arc
(docs/spikes/20260709-192358-component-lock-vats-lite.md). The live-structure
anchor helper decided in Steps is now also the pre-focus turret aim anchor and
the AI aim anchor for that arc - land this task first; it fixes the bug on its
own even if the rest of the arc waits.
