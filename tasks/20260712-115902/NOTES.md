# Notes: teardown despawn races (20260712-115902)

## Mechanism

An `On<Remove, X>` observer fires while its entity is being DESPAWNED by a
queued command (the UnloadScenario sweep, ship death). Bevy applies each
command's queued follow-ups immediately - `command.apply(world);
world.flush()` per command, confirmed in bevy_ecs world/command_queue.rs
(review round 1) - so an observer's commands run right after the despawn
that triggered it, BEFORE the queue's remaining pending despawns. The race
therefore exists only when the observer's command targets the entity whose
own despawn triggered it: that entity is already gone by the time even an
immediately-applied command lands, and a `get_entity` guard in the observer
only proves liveness at observer time.

Two flavors, discovered while building the regression rig:

- `remove`/`despawn` bake in the WARN handler AT QUEUE TIME
  (bevy_ecs-0.19 commands/mod.rs: `queue_handled(_, warn)`) - they log
  "Encountered an error in command ...: Entity despawned" and are INVISIBLE
  to a `FallbackErrorHandler` swap. This is exactly the playtest warn, and
  it means the 20260713-175352 fallback-to-panic pin does NOT catch this
  class - filed as task 20260713-203709.
- `insert` queues UNHANDLED - it goes through the fallback handler (warn by
  default, panic under the pin's swap).

A direct `World::despawn` does NOT reproduce the race: the entity is
already gone when the observer queues, `get_entity` bails, nothing lands.
The rig must queue the despawn as a command, like the sweep does.

## The decisive probe: only the SAME-ENTITY shape races

The audit initially suspected cross-entity races too (camera handover,
dominant-well strip). Both were implemented as try_* fixes with
deterministic sweep-order tests, then sabotage-A/B'd (plain variants
restored, commit c686520 first) - and BOTH tests stayed green under the
plain commands: no warn, race refuted. Bevy applies an observer's queued
commands BEFORE the remaining pending despawns in the flushing queue, so
an observer whose target comes from a live query/`Single` cannot hit a
dead entity. The try_* hardening was therefore REVERTED (loud errors are
worth keeping for real future bugs), and the two tests were kept as
ORDERING PINS: they fail with "Entity despawned" exactly if bevy ever
moves observer commands behind the pending queue, which is when the try_*
variants would become necessary.

The only shape that races - confirmed red-green - is the SAME-ENTITY one:
a command targeting the entity whose own despawn triggered the observer
(it is already gone when even an immediately-applied command lands). Both
production-observed warns have this shape: remove_maneuver_telemetry
(this task) and remove_screen_indicator_camera (hud/mod.rs:356, the
asteroid_next precedent - its comment's "same command flush" is the
same-entity case).

## Sweep verdicts (every production `On<Remove, ..>` observer)

Fixed (racy - the command targets the despawning entity itself):

- flight.rs:250 `remove_maneuver_telemetry` - the reported bug; queued
  `remove::<ManeuverTelemetry>` on the despawning ship. -> `try_remove`.

Probed and refuted (plain commands kept, ordering pinned by test):

- nova_scenario loader.rs:573 `on_player_spaceship_destroyed` - queued
  remove+insert on the scenario camera; the observer's commands apply
  before the camera's pending despawn, and a camera that despawned first
  fails the `Single`, skipping the observer. Pin:
  `a_player_ships_despawn_does_not_race_the_cameras`.
- gravity.rs:237 `remove_dominant_well_on_well_removed` - queued
  `remove::<DominantWell>` on holder ships; same ordering argument, query
  yields only live holders. Pin:
  `a_wells_death_does_not_race_its_holders_despawn`.

Safe (decided per site):

- flight.rs:1829 `on_autopilot_removed_cool_engines` - direct query
  mutations, no commands.
- flight.rs:~1524 autopilot_system's in-system telemetry clear - safe by
  schedule TOPOLOGY, not by a sync-point guarantee (review R1.3): sync
  points are shared, so an earlier-ordered system's buffered ship despawn
  would apply first - but no such producer exists: ship-root despawns
  originate in Update-side observer flushes (unload sweep, integrity
  death), while autopilot_system runs in FixedUpdate. Re-audit this site
  if a FixedUpdate-scheduled ship-despawn path ever appears.
- loader.rs:557 `on_player_spaceship_spawned` - fires on player ADD (load
  path); nothing despawns the camera in that flush.
- hud/beacon_chips.rs:170, hud/objective_markers.rs:212,
  hud/item_highlights.rs:134 - each despawns its chip/layer entity, which
  is a ROOT UI node (screen_indicator_layer, no ChildOf, no
  DespawnOnExit/scenario scoping) whose ONLY death path is this observer -
  the target is alive at apply time. Caveat (corrected by review R1.2): if
  chips ever become sweep/state-scoped, the resulting warn would come from
  the SWEEP's own pending plain `despawn` hitting the chip the observer
  already despawned (beacon-first order) - an observer-side try_despawn
  cannot prevent that; the right change would be dropping the observer's
  then-redundant despawn or making the sweep's despawns fallible.
- hud/mod.rs:356 `remove_screen_indicator_camera` - already `try_remove`
  (the precedent fix, playtest 2026-07-13 asteroid_next).
- hud/mod.rs PlayerSpaceshipMarker observers (413-829) - despawn non-scoped
  HUD widgets whose only death path is those observers.
- input/player.rs:711, camera_controller.rs:295 - already try_*.
- editor lib.rs:1456, beacon_chips.rs:256 - query mutations only.

## Regression tests

Three, all on the same pattern: capture the thread's tracing output
(`nova_gameplay::test_log::CapturedLog`; duplicated in nova_scenario's
loader tests - one 20-line helper does not justify a shared test-util
crate), drive sweep-style queued despawns in a forcing order, and assert
no "Entity despawned" in the log. Each carries two in-test delivery
guards: a deliberate stale command that MUST warn (proves the capture sees
the class) and a live-path assertion that the observer's command really
lands (proves the wiring).

Evidence trail:

- Flight (the fix): written BEFORE the fix, failed red with the exact
  production warn ("Encountered an error in command ...: Entity despawned:
  The entity with ID 16v0 is invalid; its index now has generation 1"),
  green with `try_remove`.
- Loader + gravity (the pins): written for the suspected cross-entity
  races, they stayed GREEN under sabotage back to the plain commands -
  which refuted those races and reclassified the tests as ordering pins
  (they can still fail: a bevy ordering change to breadth-first would
  produce the warn).

## Alternatives considered

- Gating the observer on "entity still exists" cannot work: liveness at
  observer time does not survive to apply time (that guard was already
  there and the warn shipped anyway).
- A global "silence all command errors during teardown" switch would hide
  real bugs; per-site fallible commands keep every other error loud.
