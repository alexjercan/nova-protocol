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

- [ ] Decide the aim anchor: live COM (matches the camera) or the surviving
      structure's bounds center; probably a shared helper on the ship root.
- [ ] Update AI targeting (input/ai.rs) and player turret aim/lock-on
      (input/player.rs) to use it.
- [ ] Cover: unit test for the helper; check an AI scenario headlessly (enemy
      shots land on a front-stripped ship, not behind it).

## Notes

- Origin consumers that are fine as-is: autopilot GOTO/navigation (meter-scale
  offsets do not matter) and HUD projections (they project target entities).
- Related: 20260709-140620 (camera anchored on live COM, this bug's sibling).

Update (20260709): this bug seeded the component-lock arc
(docs/spikes/20260709-192358-component-lock-vats-lite.md). The live-structure
anchor helper decided in Steps is now also the pre-focus turret aim anchor and
the AI aim anchor for that arc - land this task first; it fixes the bug on its
own even if the rest of the arc waits.
