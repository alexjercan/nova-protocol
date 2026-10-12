# World first: scenario state in the world save

- STATUS: OPEN
- PRIORITY: 92
- TAGS: v0.16.0,world-first

## User facts
- Epic `20261012-095526`. A harnessed world saves like any world only once scenario state (variables, objectives, timers) enters the save. Until then harnessed scenarios stay session-only, as chapters are today. This task decides and, if approved, lands it.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Agent findings
- `WorldSaveState` holds `format, generation, player, sectors, canister_ids_next, transients` and no scenario state (`crates/nova_world_base/src/save/mod.rs:103-117`); `WORLD_SAVE_FORMAT = 3`, never migrated (`:63`, `:1-21`). `NovaEventWorld` is cleared at teardown (`crates/nova_scenario/src/world.rs:183-200`).

## Delivery gate
- Show the saved shape (variables, objectives with progress, timers with remaining live seconds) and the thaw order relative to the harness `OnStart`. Format bump; no migration. Stop for the owner on whether `OnStart` re-fires on load.

## Verification
- Save mid-objective, load, the objective is still in progress and completes; a timer resumes with the remaining time.

## Done when
- A harnessed world round-trips its scenario state, or the owner records that harnessed scenarios stay session-only and this task closes with that decision.
