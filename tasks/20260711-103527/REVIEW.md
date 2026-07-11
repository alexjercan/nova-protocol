# Review: Thruster impulses push from the stale eased pose

- TASK: 20260711-103527
- BRANCH: fix/thruster-impulse-raw-pose

## Round 1

- VERDICT: APPROVE

Verified against the spec (TASK.md Goal + Steps) and the spike doc:

- The core fix is correct and minimal: `thruster_impulse_system` composes
  application point AND direction from the root's raw `Position`/`Rotation`
  via the `Forces` item accessors plus the local mount Transform - byte-for-
  byte the balancer's lever-arm math (flight.rs), so the claimed invariant
  now holds by construction. The old code's internal clock mix (raw child
  Rotation direction, eased GlobalTransform point) is gone.
- The direct-child assumption was independently verified in both production
  spawn paths: ship sections (nova_scenario/objects/spaceship.rs,
  `with_children` off the root) and the torpedo thruster
  (torpedo_section/mod.rs, `children![...]` on the root). The balancer and
  manual_burn_system already carried the same assumption.
- The regression test is behavioral and was proven against the bug
  (7.1 rad/s spin pre-fix, 0 post-fix), uses a production-faithful rig
  (TransformInterpolation on the hull - the double-tick-only pitfall the
  TASK.md documents), and asserts an invariant (zero-torque line through
  the COM) that cannot pass accidentally. The diagnostic was deleted in
  the same branch per the retro convention, with its trace recorded in
  TASK.md.
- Checks: full nova_gameplay lib suite run by the reviewer - 351 passed,
  0 failed (includes high_speed_stop_settles_without_tumbling and the
  ORBIT/GOTO maneuvers exercising the q_target change). cargo check and
  fmt clean. No tests weakened or deleted.
- Audit table in TASK.md spot-checked: sync_controller_section_forces,
  manual_burn_system, gravity_well_system and the bcs PD path confirmed
  clean; AI Update-schedule reads correctly scoped out to the HUD tasks.

- [ ] R1.1 (MINOR) crates/nova_gameplay/src/flight.rs:1397-1400 - the GOTO
  raw-Position preference has no dedicated regression; a future refactor
  could silently reorder the `(Option<&Position>, &GlobalTransform, ...)`
  preference. A behavioral pin would need to detect ~2 ticks of target
  motion in the goal position, which is brittle; if no clean assertion
  exists, a one-line note in the existing goto test naming the preference
  is acceptable. Left to the implementer's discretion.
  - Response: Accepted as-is with reasoning: a behavioral pin would
    assert sub-unit differences in desired velocity against a moving
    target (brittle), and a presence-style component check is exactly the
    weak-test antipattern the 20260709-160753 retro flagged. The
    preference is documented at the query site with the task ID; the
    umbrella's combined verification re-tests GOTO behavior end to end.
