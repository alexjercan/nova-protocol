# Torpedo bay test range example (playable + gates + autopilot)

- STATUS: OPEN
- PRIORITY: 90
- TAGS: v0.4.0,example,torpedo

The torpedo bay is the clunkiest section: torpedoes sometimes spawn too close and
die instantly, the controls feel weird, and the homing is weak. Build a dedicated
playable test range for it (mirrors the PDC turret range task) so torpedo behavior
is easy to observe, tune, and regression-test.

Goal: a scene where you can fire torpedoes at targets at varied ranges and clearly
see whether they arm, home, and detonate correctly.

## Steps

- [x] Add `examples/06_torpedo_range.rs` with a single player ship carrying one torpedo
      bay section. Camera is the scenario system's chase camera (auto-attached to the
      player ship), so no bespoke camera was needed.
- [x] Place **target gates** at a spread of distances: near (-30), mid (-70), far (-120),
      one to the side (-25 x), and one drifting laterally (the moving gate). All are
      asteroid objects (proven damageable + explodable). Lifecycle logging traces
      `torpedo fired -> armed -> detonated`.
- [x] Visualize the guidance: gizmos draw the torpedo -> target line-of-sight, a sphere
      at the target point, and a status sphere on the torpedo (yellow un-armed, green
      armed - so the arming delay is visible).
- [x] Wire the BCS autopilot + screenshot harness (scene built on
      `GameAssetsStates::Loaded` so the screenshot's forced Playing does not re-run
      setup). Headless autopilot run: reached Playing, 3 fired, 3 armed, 3 detonated,
      cycle complete, no panic, exit 0. Screenshot run: 1024x768 PNG, exit 0.
- [x] Use the range to validate the arming fix (`20260707-100003`): armed transition is
      logged and shown by the gizmo colour; no self-detonation on spawn. Target-loss
      (`20260707-100004`) and PN guidance (`20260525-133021`) are still open and can now
      be checked/tuned here. A range convenience `range_autotarget` assigns the nearest
      gate so homing is exercised hands-off (the game uses mouse-aim targeting instead).
- [x] Ran headless via `BCS_AUTOPILOT=1` under Xvfb; smoke output as above.

## Resolution

Added `examples/06_torpedo_range.rs`: a player torpedo ship vs. near/mid/far/side/moving
asteroid gates, with guidance gizmos, arm->home->hit logging, a drifting target, and the
autopilot + screenshot harness so it doubles as a headless smoke test. Also added
`[lints] workspace = true` to the root package so examples inherit the workspace clippy
allows (fixes `type_complexity` on the example queries, matching the crates' policy).
`cargo clippy --features debug`, the no-debug build (harness cfg's out), and the headless
autopilot + screenshot runs are all green.

## Notes

Torpedo logic: `crates/nova_gameplay/src/sections/torpedo_section.rs`
- spawn: `shoot_spawn_projectile` (spawns at `spawner` transform, ~0.01 ahead).
- detonation: `torpedo_detonate_system` (fires when within `BLAST_RADIUS*0.5` of the
  target position, no arming gate).
- guidance: `torpedo_sync_system` + `torpedo_thrust_system` (ad-hoc pursuit feeding an
  absolute quaternion into the PD controller).
This range is the harness for tasks 20260707-100003 (arming), 20260707-100004 (target
loss), 20260525-133021 (PN guidance), and 20260706-162913 (torpedo module extraction).
