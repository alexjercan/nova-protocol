# Fix: ManeuverTelemetry remove races scenario teardown (despawned entity warn)

- STATUS: OPEN
- PRIORITY: 60
- TAGS: bug,flight,scenario

Observed in the user's 2026-07-12 playtest log (pre-existing, not from
the playtest-fixes branch): during scenario teardown (UnloadScenario
inside OnExit(GameStates)), a queued
`EntityCommands::remove::<ManeuverTelemetry>` hits an entity the unload
already despawned - bevy logs "Encountered an error in command ...
Entity despawned: The entity with ID 1066v0 is invalid". Likely
`remove_maneuver_telemetry` (flight.rs observer) racing the scoped
despawn sweep. Harmless warn today, but it is exactly the class of
teardown race that graduates into a panic when the command grows.
Fix direction: `try_remove`/queue_silenced in the observer, or gate it
on the entity still existing.

Notes:
- Full stack in the 2026-07-12 session log (WARN block preceding the
  duplicate-Node panic, which is fixed separately in 20260712-110730).
