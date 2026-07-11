# Review: Bullets twitch badly at high spaceship velocity

- TASK: 20260710-231930
- BRANCH: fix/bullet-spawn-raw-clock

## Round 1

- VERDICT: APPROVE

Verified against the spec and the spike doc:

- The raw-clock rewrite is correct: fire timing on fixed dt (merged tick
  removes the previously UNORDERED tick-vs-shoot pair in the Update set),
  muzzle pose via `local_pose_in_root` from the root's raw avian pose,
  every velocity term on the same raw state, and the sub-tick lead formula
  `muzzle - muzzle_exit_velocity * lead` whose derivation (ship terms
  cancel) is written down in TASK.md and in code. The reviewer re-derived
  the tick-window algebra and confirms the planned "full velocity *
  overshoot" formula would indeed have leaked ship-velocity scatter; the
  landed formula is exact to first order.
- Once-timer overshoot bookkeeping checked against edge cases by hand:
  idle-then-arm fires immediately at tick start (excess clamps to dt);
  continuous fire preserves phase across the reset-and-advance loop;
  fire intervals shorter than a tick multi-fire correctly (including two
  shots in the arming tick, which is the physically right cadence);
  MAX_SHOTS_PER_TICK backstops runaway configs and is documented, not
  silent.
- Tests are behavioral and A/B-proven: the stream regression fails with
  lead compensation disabled AND against the pre-fix Update-schedule
  path; the delivery guard pins the stride so a degenerate all-one-point
  "stream" cannot pass. The cadence regression pins the restored
  100 rounds/s (the old path silently capped at frame rate - good catch,
  documented in the spike record).
- Reviewer ran: full nova_gameplay lib suite 354/354; cargo check
  --workspace --all-targets (examples compile - the real risk of a
  schedule move); consumer grep for the deleted system and the moved
  system found no external references. fmt clean. No tests weakened;
  rig updates make the allegiance tests MORE production-shaped (muzzle in
  the ship tree, raw pose components).
- TASK.md is honest, including the uncommitted-A/B git-checkout blunder
  and its recovery.

- [x] R1.1 (MINOR) crates/nova_gameplay/src/sections/torpedo_section/mod.rs:305-308
  - the torpedo section's own `shoot_spawn_projectile` still spawns in
  Update from TransformHelper (eased pose): the same clock mix this branch
  fixes for bullets, at lower severity (single guided launches, guidance
  absorbs the offset, no stream to scatter). Out of this branch's scope
  per review rules; file a tatr follow-up task so the family umbrella can
  reference it instead of silently leaving the known sibling unfixed.
  - Response: agreed - filed as tatr task 20260711-114640 (torpedo launch
    pose on the raw clock), tagged v0.5.0/physics/torpedo, priority 55,
    linked to the spike doc. No code change on this branch.
