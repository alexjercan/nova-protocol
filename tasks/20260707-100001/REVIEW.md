# Review: Torpedo bay test range example

- TASK: 20260707-100001
- BRANCH: feature/torpedo-range

## Round 1

- VERDICT: APPROVE

Delivers the Goal: a playable torpedo range that also runs as a headless smoke
test. One player torpedo ship vs. near/mid/far/side/moving asteroid gates, with
guidance gizmos (line-of-sight + armed/un-armed status sphere), fired->armed->
detonated logging, and the autopilot + screenshot harness. Every Step is ticked
and genuinely done.

Verified independently in the worktree:

- `cargo build --example 06_torpedo_range --features debug`: green.
- `cargo build --example 06_torpedo_range` (no `debug`): green - the harness
  (`nova_autopilot`/`nova_screenshot`, `autopilot_fire`) all cfg out.
- `cargo clippy --example 06_torpedo_range --features debug`: clean, after adding
  `[lints] workspace = true` to the root package (the right fix: examples now
  inherit the same `type_complexity` allow every crate already uses; allows-only,
  so it cannot introduce warnings elsewhere).
- `BCS_AUTOPILOT=1 ... --features debug` (Xvfb): `nova harness: reached Playing`,
  3x `torpedo fired`/`armed`/`detonated`, `cycle complete, no panic`, exit 0, no
  real panics and no ERROR-level log spam.
- `BCS_SHOT=1024x768 ...`: 1024x768 PNG (1.1 MB, i.e. a real rendered frame, not
  a black one), exit 0.

The scene is built on `GameAssetsStates::Loaded` (not `OnEnter(Playing)`), which
correctly sidesteps the screenshot preset's forced-Playing double-setup caveat
documented in the harness task. Faithful reuse of the real firing/targeting
pipeline (scenario + player input + section systems) rather than a mock.

No BLOCKER/MAJOR. Two NITs, take-it-or-leave-it.

- [ ] R1.1 (NIT) examples/06_torpedo_range.rs:216 `range_autotarget` - both this
  and the game's own player-targeting assign `TorpedoTargetEntity` to un-targeted
  player torpedoes. It is harmless (both resolve to the gate's rigid-body parent,
  so the same/compatible target), but redundant. Could skip range_autotarget when
  the player-target resource is already set. Left as-is: the range convenience
  guarantees homing hands-off (autopilot), which is the point.
  - Response:
- [ ] R1.2 (NIT) examples/06_torpedo_range.rs:230 `drive_moving_gate` overwrites
  `LinearVelocity` on a `RigidBody::Dynamic` gate every frame. It works (effective
  kinematic drift), but a `RigidBody::Kinematic` gate would model a scripted mover
  more cleanly. Not worth the extra wiring for a test range.
  - Response:
