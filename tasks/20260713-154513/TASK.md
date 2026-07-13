# Debug-harness scenario assertion crashes on legitimate scenario transitions

- STATUS: CLOSED
- PRIORITY: 55
- TAGS: v0.5.0,bugfix,debug

## Outcome (CLOSED 2026-07-13)

Playtest crash report (user, 2026-07-13, in 03_scenario right after the
asteroid-kill fix WORKED - 6 asteroids destroyed, objective complete,
scenario advancing): two issues in the transition to `asteroid_next`.

- **FATAL (debug builds only)**: nova_debug's smoke assertion
  `assert_scenario_loaded_payload` stayed armed forever - it asserts the
  loaded id equals the boot scenario AND object_count > 0, so the first
  legitimate transition (a different id, and an object-less epilogue
  scenario) panicked the whole app. The player's first SUCCESS was the
  crash trigger. Fixed: the smoke contract covers the FIRST load only
  (`fired` short-circuits later loads, info-log retained); pinned by a
  test that fires a second load with a different id and zero objects.
  NOT a core-game bug - the harness ships behind the debug feature - but
  the user plays with debug on, so it read as one.
- **WARN (core, harmless but noisy)**: `remove_screen_indicator_camera`
  guarded with `commands.get_entity`, which proves existence at QUEUE
  time only - the scenario teardown despawns the camera in the same
  command flush, so the plain `remove` warned "entity despawned" with a
  full backtrace. Fixed with `try_remove` (the queue-time-vs-apply-time
  race is inherent to teardown observers).

Verified: nova_debug tests (incl. the new pin), 471 nova_gameplay tests,
fmt clean.

## Notes

- The log also CONFIRMS the 150343 asteroid-kill fix live:
  asteroids_destroyed reached 6.0 and the objective flipped in real play.
