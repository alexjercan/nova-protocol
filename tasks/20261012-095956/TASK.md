# World first: scenario state in the world save

- STATUS: OPEN
- PRIORITY: 92
- TAGS: v0.16.0,world-first

## User facts
- Epic `20261012-095526`. A harnessed world saves like any world only once scenario state (variables, objectives, timers) enters the save. Until then harnessed scenarios stay session-only, as chapters are today. This task decides and, if approved, lands it.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Agent findings
- `WorldSaveState` holds `format, generation, player, sectors, canister_ids_next, transients` and no scenario state (`crates/nova_world_base/src/save/mod.rs:103-117`); `WORLD_SAVE_FORMAT = 3`, never migrated (`:63`, `:1-21`). `NovaEventWorld` is cleared at teardown (`crates/nova_scenario/src/world.rs:183-200`).

- Survey 2026-10-12: the save is written on every `CurrentSector` change and on leave (`crates/nova_world_base/src/save/session.rs:165-172`, `:323-325`), never timed. Load from the menu runs the bundle's declared start (`crates/nova_menu/src/lib.rs:361-362`), Load last save re-sends `LoadScenario(CurrentScenario)` (`leave.rs:163-165`); OnStart always re-fires (`lifecycle.rs:503`) and the player spawn thaws the `ResumedSpaceship` record instead of doubling (`actions/spawn.rs:289-317`). Scenario state on `NovaEventWorld` (variables, objectives, story log, readouts, `scenario_elapsed`, timer deadlines, sequence runs) is cleared at teardown (`world.rs:526-560`); `once` handlers are retired by despawning the handler entity and rebuilt on every load (`nova_events/src/engine.rs:456-484`, `lifecycle.rs:477-494`). Handler-spawned persistent ships are addressable, so the save refuses the snapshot while they live (`frozen.rs:815-832`): OUT OF SCOPE here, note it.
- Owner decision 2026-10-12: option A. `WORLD_SAVE_FORMAT` 4, no migration. `WorldSaveState.scenario: SavedScenario { scenario_id, elapsed, variables, objectives, story, readouts, timers (remaining seconds), spent_handlers (indexes into scenario.events; stable because the catalog digest pins content), marks: Vec<(GeneratedBodyId, label)> }`. Load loads the saved scenario id, not the declared start. Thaw order: `resume_world` inserts a resumed-scenario record; the loader rebuilds handlers skipping spent indexes, restores the event world (timers rebased on the restored clock) and `GeneratedMarks`, then fires `OnResume` INSTEAD of `OnStart`. New vocabulary event `OnResume`; the open world builder shares one spawn list (player + lights) between OnStart and OnResume; lint requires an OnResume handler on a scenario that can save (needs_session or fixed seed with a save session). A save that lands while a sequence or cinematic runs waits with a bounded settle; mid-beat sequences are not serialised.

- Landed 2026-10-12: format 4 with `WorldSaveState.scenario: SavedScenario` (`crates/nova_scenario/src/resume.rs`); the engine fires `HandlerRetired` before a `once` despawn and `SpentHandlers` records the `ScenarioHandlerIndex`; `open_scenario_run` (`loader/lifecycle.rs`) skips spent handlers, restores the event world, the HUD objectives and the marks and fires `OnResume`; a mismatched resumed id logs an error and fires `OnStart`; an unregistered saved id is `WorldRefusal::Scenario`. The snapshot waits (bounded settle) on a running sequence or cinematic. Lint: `seed: None` needs an `OnResume` spawning one player; an OnStart/OnResume spawner pair is not a duplicate. Proof: `system_world_resume` (three new markers), loader and timer unit tests.

## Delivery gate
- Show the saved shape (variables, objectives with progress, timers with remaining live seconds) and the thaw order relative to the harness `OnStart`. Format bump; no migration. Stop for the owner on whether `OnStart` re-fires on load.

## Verification
- Save mid-objective, load, the objective is still in progress and completes; a timer resumes with the remaining time.

## Done when
- A harnessed world round-trips its scenario state, or the owner records that harnessed scenarios stay session-only and this task closes with that decision.
