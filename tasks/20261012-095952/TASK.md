# World first: New Game harness takes the wizard seed; chapters declare Void

- STATUS: CLOSED
- PRIORITY: 96
- TAGS: v0.16.0, world-first

## User facts
- Epic `20261012-095526`. After the world block lands, the `open_world` scenario becomes the New Game harness: `Generated { seed: None }` means the seed comes from the New Game modal. Every shipped chapter, backdrop and tutorial declares `Void` and behaves exactly as today.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Agent findings
- Owner pick 2026-10-12 (research note `tasks/20261012-095949/RESEARCH.md`): shape 1. Types: `ScenarioConfig.world: ScenarioWorldType` with `serde(default, skip_serializing_if = is_void)`; `ScenarioWorldType::{Void, Generated(GeneratedWorldConfig)}`; `GeneratedWorldConfig { seed: Option<u32>, bounds: Option<ScenarioBoundsConfig> }`; `ScenarioBoundsConfig { position: Meters3, radius: Meters }`. `ScenarioRole::OpenWorld` is deleted. `ScenarioConfig::picker_lists()` = `role == Chapter && !world.needs_session()`, so fixed-seed chapters are picker rows. `OpenWorldSession` keeps its name.
- New Game inserts `OpenWorldSession { seed }` then enters Playing (`crates/nova_menu/src/world_setup.rs:324-370`); `start_new_game_scenario` picks the base-declared start or a picker override (`crates/nova_menu/src/menu_ui.rs:581-624`).
- `resume_world` re-inserts the session from the save header (`crates/nova_world_base/src/save/session.rs:234-268`); Retry streams the same world again (`crates/nova_world_base/src/lib.rs:150-156`).

- Landed with `20261012-095950` (commit bd1838e75): `open_world` already declares `world: Generated((seed: None, bounds: None))` and every other shipped scenario is `Void` by default; `sync_open_world` takes the seed from `OpenWorldSession` for `seed: None`. Remaining here: confirm the New Game modal path end to end (Create, Retry, Load) and add any runtime load gate the lint-walk rule 4 lacks.

- Landed 2026-10-12 in commit abd14d8f7: `resolve_new_game_scenario` + `NewGameStartFault` in `crates/nova_menu/src/menu_ui.rs` (error log + `AppExit::error()`, fallback chain deleted); lint Error for `seed: None` on a backdrop or lesson in `crates/nova_scenario/src/lint/scenario.rs`; fixed-seed picker chapter is session-only (no `WorldSaveSession`, Retry reloads, Load last save not offered). Live runs under Xvfb/lavapipe: system_open_world (Create, Retry, disarm), system_scenario_picker, system_world_resume all reached Playing.

## Delivery gate
- The harness reads its seed from the session when the block says `None`; a `Generated { seed: Some }` scenario launched from the picker creates no save folder on desktop unless the note decided otherwise. Migrate every base scenario builder to declare its block; regenerate RON; migrate the hand-written example mod.
- What may break: Retry, Load, the leave flow and the web build (no save module). What must fail loudly: a `Generated { seed: None }` scenario started with no session.

## Verification
- New Game, Retry, Load and a picker-launched chapter each reach Playing in a live run under Xvfb; the main-menu backdrop rotation still runs with no `WorldConfig`.

## Done when
- The open world is one more scenario on the harness path, no scenario is special-cased by role, and the example mod lints clean.
