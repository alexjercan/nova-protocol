# Review: Look-ray + camera-mode infrastructure

- TASK: 20260713-082324
- BRANCH: feature/look-ray-infrastructure

## Round 1

- VERDICT: APPROVE

Scope: camera_controller.rs (derivation + seeding + accessor + raised flag +
tests), targeting.rs (two aim queries -> ActiveLookRay, faithful split-rig
fixtures + ray-liveness regression), TASK.md.

Independent verification (shared-session blind-spot guard):
- Re-derived the "no external mode consumers" claim: grepped
  `SpaceshipCameraControlMode` repo-wide - zero references outside
  camera_controller.rs, so deleting the observers breaks no one.
- The A/B sabotage was run for real (commit-before-sabotage respected):
  seeding source reverted to the normal rig -> `transitions_seed_from_the_
  outgoing_rig` FAILS with the flanker message; restored -> passes.
- The frozen-ray fix is proven at the LIVE app level, not just unit tests:
  12_hud_range's autopilot asserts the lock acquires at +0.5 s, which now
  flows through ActiveLookRay - both autopilots green.
- Decoy discipline: the split-rig fixtures put the dormant turret rig 90 deg
  off, so any picker still reading the turret rig fails BOTH the new
  ray-liveness test and the pre-existing cone tests. Checked that the decoy
  would indeed have acquired the side body in `acquisition_follows_the_live_
  active_ray` (it points at -X, where the body sits) - the delivery-guard
  None assertion doubles as a decoy-reading detector.

- [ ] R1.1 (MINOR) camera_controller.rs `derive_control_mode_and_raised` -
  the "every gameplay consumer of WeaponsRaised is pause-gated itself" doc
  claim is forward-looking: no consumers exist yet. Fine today, but 082330/
  082337 MUST honor it (their radar/safety observers need pause guards) or
  the pause-ungated derivation becomes a hole. Carry the note into those
  tasks rather than changing anything here.
  - Response: Noted in the task Outcome ("Note for 082330/082337") and both
    task plans already specify pause-guarded observers. No code change.

- [ ] R1.2 (NIT) The old `_spaceship: Single<&Transform, ...>` gate on
  sync_spaceship_control_mode keeps mode transitions inert while shipless
  (pre-existing). The derivation still updates the RESOURCE shipless, so on
  ship spawn the marker state converges on the next real transition. No
  action; recorded for the componentization task.
  - Response: acknowledged, no action.

Positives: memoryless derivation is the right shape (the nested-hold matrix
would be unreadable as observer patches); the production-faithful test rigs
(real InputPlugin + EnhancedInput + production action shape, finish/cleanup
sequencing) exercise the actual input pipeline rather than hand-set state;
pause semantics are documented AND pinned with a delivery-guarded test.

Checks: 13 camera + 46 targeting + 147 input tests pass; cargo fmt clean;
cargo check --workspace clean; 12_hud_range + 10_gameplay autopilots PASS
(no-behavior-change contract held). Full suite + clippy run in CI per repo
policy.
