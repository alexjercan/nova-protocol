# Review: ManeuverTelemetry teardown despawn race

- TASK: 20260712-115902
- BRANCH: fix/telemetry-teardown-race

## Round 1

- VERDICT: APPROVE

Independently re-derived the load-bearing claims before judging:

- (a) Handler flavors - CONFIRMED from bevy_ecs-0.19.0 source.
  `EntityCommands::remove` (commands/mod.rs:1726) is
  `queue_handled(entity_command::remove::<B>(), warn)` and `despawn`
  (:1906) is `queue_handled(entity_command::despawn(), warn)`;
  `queue_handled` wraps the command via `handle_error_with(error_handler)`
  (commands/command.rs:66), which closes over the concrete `warn` fn at
  queue time - a `FallbackErrorHandler` swap cannot reach it. `insert`
  (:1435) goes through `queue` -> `handle_error()` (command.rs:85), which
  reads `world.fallback_error_handler()` at APPLY time, so the swap does
  see it. `try_remove` (:1827) is `queue_silenced` -> `ignore_error()`.
  The premise of follow-up 20260713-203709 is real.
- (b) Flush ordering - CONFIRMED from bevy_ecs-0.19.0
  world/command_queue.rs. Each command's apply closure runs
  `command.apply(world); world.flush();` (push, ~:185-190), and
  `apply_or_drop_queued` (:235) bumps the global cursor to `stop` before
  looping on a `local_cursor`, so a nested flush applies only
  newly-queued (observer) commands - BEFORE the remaining pending
  despawns of the flushing queue. This simultaneously explains the
  same-entity race (the observer's command applies right after its own
  target's despawn completes) and the cross-entity refutation (a
  cross-entity target from a live query/Single is still alive when the
  observer's command applies). The revert of the two try_* hardenings is
  correct, and the pins' premise is sound.
- (c) Sweep mirror - CONFIRMED. `teardown_scenario_entities`
  (nova_scenario/src/loader.rs:396) queues plain
  `commands.entity(entity).despawn()` per scoped entity from an observer
  (`unload_scenario`), i.e. into the world's internal queue - exactly the
  queue the tests drive via `app.world_mut().commands()`. The tests'
  "sweep-style queued despawns" match the production path, including the
  queue-identity subtlety the cursor mechanics depend on.
- (d) Would-they-fail - CONFIRMED. I re-ran the flight sabotage
  (try_remove -> remove): the test goes red with the production warn;
  restored, green. The two pins pass as shipped (plain commands), and
  their delivery guards are non-vacuous: guard 1 proves the capture sees
  the exact baked-warn class ("Entity despawned" from a deliberately
  stale remove), guard 2 proves the observer wiring lands on a live
  target, and `log.clear()` isolates the race assertion. Under a
  hypothetical bevy switch to breadth-first observer commands, the
  pinned remove/insert would land after the pending despawn and produce
  the guarded warn class - the pins can fail exactly when claimed.

Honesty check: TASK.md/NOTES.md match the diff and the commit history
(c686520 ships the three try_* fixes, 10e7184 reverts the two
cross-entity ones and keeps the tests - the refutation story is real,
not retconned). Sweep verdicts spot-checked: the three HUD chip
observers despawn root UI nodes (`screen_indicator_layer()` bundles, no
ChildOf on the root, no ScenarioScopedMarker - the loader's auto-scoping
observers cover only MeshFragment/TurretBullet/Torpedo markers, and no
DespawnOnExit/StateScoped exists under hud/) whose only death path is
the observer; hud/mod.rs:356 and camera_controller.rs:311-317 are
already try_*; editor lib.rs:1455 and beacon_chips.rs:255 are query
mutations only. Changelog matches shipped behavior. Follow-up
20260713-203709 is coherent: examples/13_menu_newgame.rs:52 contains
the FallbackErrorHandler(panic) swap, tests/examples_smoke.rs already
greps stderr, and "Encountered an error in" is the stable handler
prefix (bevy_ecs error/handler.rs:96).

Checks run in the worktree: `cargo fmt --check` clean;
`cargo check --workspace --all-targets` clean (pre-existing
proc-macro-error2 future-incompat note only); the three named tests
pass (`despawning_an_autopiloting_ship_queues_no_stale_telemetry_command`,
`a_wells_death_does_not_race_its_holders_despawn`,
`a_player_ships_despawn_does_not_race_the_cameras`). Full suite and
clippy intentionally not run (repo convention - CI does).

- [x] R1.1 (MINOR) tasks/20260712-115902/NOTES.md:5-10 - the Mechanism
  section still states the REFUTED queue model as fact: "Commands the
  observer queues go to the END of the currently-flushing queue, so they
  apply after the despawns already queued ahead of them". Per the bevy
  source (command_queue.rs: `command.apply(world); world.flush();` with
  the global cursor bumped to `stop`) and per this document's own
  "decisive probe" section, observer commands apply immediately after
  the triggering command and BEFORE the remaining pending despawns; only
  the same-entity clause ("including the despawn of their own target")
  survives. The doc's opening paragraph contradicts its own refutation
  section and would re-teach the wrong model to future readers. Suggest
  rewriting the paragraph to the verified mechanism: observer-queued
  commands flush right after the triggering command completes - after
  its own target's despawn, before the queue's remaining despawns.
  - Response: fixed - the Mechanism section now states the confirmed
    per-command flush model (observer commands apply immediately, before
    remaining pending despawns) and derives the same-entity-only race from
    it.

- [x] R1.2 (MINOR) tasks/20260712-115902/NOTES.md:85-92 (HUD chip
  caveat) - the recorded remedy is the wrong knob. If the chips ever
  become sweep-scoped, the warn does NOT come from the chip observer's
  despawn: in chip-before-beacon order the dead chip drops out of the
  observer's query (nothing queued); in beacon-before-chip order the
  observer's despawn applies to a still-live chip (per the verified
  ordering) and it is the SWEEP's own pending plain `despawn` that then
  hits the already-dead chip and warns. "The try_despawn swap should
  ride along with any such change" (in the observer) therefore cannot
  fix the race it predicts - the fix would have to live in
  `teardown_scenario_entities` (or the scoping change itself). Suggest
  correcting the caveat to point at the sweep-side double-despawn.
  - Response: fixed - the caveat now names the sweep's own pending
    despawn as the racing command and prescribes dropping the observer's
    redundant despawn or fallible sweep despawns, not an observer-side
    try_despawn.

- [x] R1.3 (MINOR) tasks/20260712-115902/NOTES.md:82-85 (flight.rs:1524
  SAFE verdict) - the stated reason ("its commands apply at the system's
  own sync point before any other system (or state transition) can
  despawn the ship. No cross-flush window") is not a mechanism
  guarantee. Sync points are shared: an earlier-ordered system in the
  same FixedUpdate interval that queues a ship-root despawn would have
  its buffer applied BEFORE autopilot_system's at the same sync point,
  landing the plain `remove::<ManeuverTelemetry>` on a dead ship. The
  verdict holds today only by schedule topology: ship-root despawns
  originate in Update-side observer flushes (integrity glue runs
  `aggregate_ship_health` in Update; explode-chain despawns are
  observer-driven and flush within their sync point) and the scenario
  sweep, all of which fully apply before autopilot_system next runs and
  its query re-matches. Suggest recording that topology argument as the
  actual reason (or swapping the site to try_remove for symmetry with
  the observer fix).
  - Response: fixed - the verdict now reads safe-by-topology (ship-root
    despawns originate in Update-side observer flushes; autopilot is
    FixedUpdate), names the shared-sync-point limitation, and asks for a
    re-audit if a FixedUpdate ship-despawn path appears.