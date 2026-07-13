# Fix: ManeuverTelemetry remove races scenario teardown (despawned entity warn)

- STATUS: CLOSED
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

- [x] Fail-first regression test: written BEFORE the fix and red against
      it with the exact production warn ("Encountered an error in command
      ...: Entity despawned: The entity with ID 16v0 is invalid; its index
      now has generation 1"). PLAN CORRECTION, discovered building the
      rig: the 175352 fallback-to-panic pattern CANNOT see this warn -
      `remove`/`despawn` bake in the WARN handler at queue time - so the
      test asserts on captured tracing output instead
      (`test_log::CapturedLog`), with two in-test delivery guards (a
      deliberate stale command must warn; a live disengage must land). The
      race also needs a QUEUED despawn - direct `World::despawn` does not
      reproduce it. Details in NOTES.md; pin gap filed as 20260713-203709.
- [x] Fix `remove_maneuver_telemetry` (crates/nova_gameplay/src/flight.rs:250)
      to queue `try_remove::<ManeuverTelemetry>()`.
- [x] Sweep the race class: every production `On<Remove, ..>` observer
      audited with a per-site verdict in NOTES.md. Two suspected
      cross-entity sites (nova_scenario's camera handover, gravity's
      dominant-well strip) were implemented as try_* fixes, then
      sabotage-A/B'd - the plain variants produced NO warn, REFUTING the
      cross-entity race: bevy applies observer commands before the
      remaining pending despawns. The try_* hardening was reverted (loud
      errors stay loud) and the two tests kept as ordering pins that fail
      exactly if bevy's ordering changes. Only the same-entity shape
      races; flight.rs:1524 and the HUD chip observers recorded SAFE.
- [x] CHANGELOG.md entry under Unreleased (Fixed).

## Notes

- Full stack in the 2026-07-12 session log (WARN block preceding the
  duplicate-Node panic, which is fixed separately in 20260712-110730).
- PRIORITY bumped 60 -> 85 in the v0.5.2 plan pass: bugs land before the
  examples rework (20260712-211352) so the rework can pin them.

## Record (2026-07-13)

What changed: `remove_maneuver_telemetry` (flight.rs) queues
`try_remove::<ManeuverTelemetry>` - the same-entity despawn race, proven
red-green by a test written before the fix. The two suspected
cross-entity races (nova_scenario's player-death camera handover,
gravity's dominant-well strip) were implemented, tested, sabotage-A/B'd,
REFUTED, and reverted to the plain commands; their tests remain as
ordering pins. Three App-level tests total, asserting on captured tracing
output (new shared helper `nova_gameplay::test_log::CapturedLog`), each
with in-test delivery guards on the capture and the observer wiring.
Mechanism, the two-flavor error-handling discovery (handled warn vs
unhandled fallback), and per-site sweep verdicts in NOTES.md. Follow-up
task 20260713-203709 (pin gap) filed.

Difficulties: the first rig was DOUBLY vacuous - a direct World::despawn
does not reproduce the race (observer queues nothing), and the
FallbackErrorHandler(panic) swap cannot see remove-warns at all (baked
handler, found in the bevy source). Then the sabotage A/B on the sweep
fixes refused to go red, which refuted the cross-entity mechanism
entirely - the audit agent's RACY verdicts there were reasoned from
breadth-first queue semantics that bevy does not have. Every one of these
was caught because a verification was required to be able to fail before
being trusted.

Self-reflection: the plan step prescribed the 175352 pin pattern from its
task record without verifying WHICH command flavors that pattern
escalates; the bevy source check that found queue_handled(_, warn) took
two minutes and should have happened at plan time (verify-first). Same
for the cross-entity fixes: they were written from an assumed queue
ordering before probing it - the A/B was the probe, and cheaper ordering
probes could have led. The wrong assumptions still paid: a real coverage
gap in the existing pin (20260713-203709) and two durable ordering pins
came out of them.
