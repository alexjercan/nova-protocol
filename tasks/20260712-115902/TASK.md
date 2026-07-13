# Fix: ManeuverTelemetry remove races scenario teardown (despawned entity warn)

- STATUS: OPEN
- PRIORITY: 85
- TAGS: v0.5.2,bug,flight,scenario

Observed in the user's 2026-07-12 playtest log (pre-existing, not from
the playtest-fixes branch): during scenario teardown (UnloadScenario
inside OnExit(GameStates)), a queued
`EntityCommands::remove::<ManeuverTelemetry>` hits an entity the unload
already despawned - bevy logs "Encountered an error in command ...
Entity despawned: The entity with ID 1066v0 is invalid". Likely
`remove_maneuver_telemetry` (flight.rs observer) racing the scoped
despawn sweep. Harmless warn today, but it is exactly the class of
teardown race that graduates into a panic when the command grows.

Mechanism (verified in source, 2026-07-13 plan pass): the observer
(crates/nova_gameplay/src/flight.rs:250-254) already guards with
`commands.get_entity(remove.entity)`, but that checks liveness at observer
time; the `remove` it queues applies after the despawn that triggered the
`On<Remove, Autopilot>` completes, so the guard cannot help. The fix is to
queue the fallible variant: `EntityCommands::try_remove` exists in bevy_ecs
0.19 (commands/mod.rs:1827) and is already the house style for the mirror
case (`try_insert` in gravity.rs, asteroid.rs).

## Steps

- [ ] Fail-first regression test: App rig with a ship carrying `Autopilot`
      + `ManeuverTelemetry`, the ECS fallback error handler swapped to
      panic (reuse the pattern from task 20260713-175352's pin,
      examples/13_menu_newgame.rs), despawn the ship as the unload sweep
      does, run an update. Must fail (panic on the queued remove) against
      the current `remove::<ManeuverTelemetry>`; record the failure.
- [ ] Fix `remove_maneuver_telemetry` (crates/nova_gameplay/src/flight.rs:250)
      to queue `try_remove::<ManeuverTelemetry>()`.
- [ ] Sweep the race class (sweep-then-delete): grep for other queued
      `remove::<...>`/`insert` sites reachable while an entity despawns -
      at least flight.rs:1524 (autopilot_system's in-system telemetry
      clear) and every other `On<Remove, ..>` observer - and decide each
      site explicitly in NOTES.md (fix it or record why it cannot race).
- [ ] CHANGELOG.md entry under Unreleased (Fixed).

## Notes

- Full stack in the 2026-07-12 session log (WARN block preceding the
  duplicate-Node panic, which is fixed separately in 20260712-110730).
- PRIORITY bumped 60 -> 85 in the v0.5.2 plan pass: bugs land before the
  examples rework (20260712-211352) so the rework can pin them.
