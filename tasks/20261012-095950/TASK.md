# World first: world block on the scenario and arming from it

- STATUS: CLOSED
- PRIORITY: 97
- TAGS: v0.16.0, world-first

## User facts
- Epic `20261012-095526`. Implements the shape picked in the research note (previous strand task). The research note (previous strand task) fixed the shape; see the pick below.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Agent findings
- Owner pick 2026-10-12 (research note `tasks/20261012-095949/RESEARCH.md`): shape 1. Types: `ScenarioConfig.world: ScenarioWorldType` with `serde(default, skip_serializing_if = is_void)`; `ScenarioWorldType::{Void, Generated(GeneratedWorldConfig)}`; `GeneratedWorldConfig { seed: Option<u32>, bounds: Option<ScenarioBoundsConfig> }`; `ScenarioBoundsConfig { position: Meters3, radius: Meters }`. `ScenarioRole::OpenWorld` is deleted. `ScenarioConfig::picker_lists()` = `role == Chapter && !world.needs_session()`, so fixed-seed chapters are picker rows. `OpenWorldSession` keeps its name.
- `ScenarioRole` lives at `crates/nova_scenario/src/loader/mod.rs:198-210`; `sync_open_world` at `crates/nova_world_base/src/lib.rs:248-352` arms on the role, panics when an OpenWorld scenario has no `OpenWorldSession`, and disarms otherwise.
- Readers to migrate: `crates/nova_scenario/src/lint/scenario.rs:88`, `crates/nova_probe/src/capabilities/snapshot.rs:437`, `:2202`, `crates/nova_world_base/src/tests.rs:348-495`, the base builder `crates/nova_authoring/src/base_content/scenarios/open_world.rs:47` and the generated `assets/base/scenarios/open_world.content.ron:7`.

- Landed 2026-10-12 in commit bd1838e75: types in `crates/nova_scenario/src/loader/mod.rs:242-313`, arming in `crates/nova_world_base/src/lib.rs:273-297`, lint rules in `crates/nova_scenario/src/lint/scenario.rs:67-227` and `crates/nova_authoring/src/lint_walk.rs:315-368`, boot refusal `crates/nova_core/src/lib.rs:581-585`. Deferred to `20261012-095953`: nothing reads `bounds` at runtime yet. Rule 4 (New Game start must need the session) is lint-walk only, no runtime load gate.

## Delivery gate
- Change the owning interface first (the scenario config type), then follow compiler errors through loader, lint, probe, tests and the base builder. Regenerate base RON with the content tool; never hand-edit it.
- Lint: a `Generated` scenario without a player spawn is an error; a `Void` scenario that names a seed is an error.
- What dies: `ScenarioRole::OpenWorld` and every match arm on it. What may break: the Scenarios picker and the `--scenario` flag path (`crates/nova_menu/src/scenarios.rs:31-40`).

## Verification
- World-base tests arm on a `Generated` scenario and disarm on `Void`; lint tests cover both errors; the content lint passes on every repo bundle; `nova-probe` open-world snapshot still reports the world armed.

## Done when
- No code reads `ScenarioRole::OpenWorld`; the open world and a void chapter both load through the same loader; the changelog carries a **(breaking)** entry for the scenario format.
