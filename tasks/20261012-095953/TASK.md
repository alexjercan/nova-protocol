# World first: bounds area on a harness

- STATUS: CLOSED
- PRIORITY: 95
- TAGS: v0.16.0, world-first

## User facts
- Epic `20261012-095526`. A harness may declare an optional bounds area on a generated world. Leaving it raises the existing area `OnExit`, so the outcome is authored, never hard-coded.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Agent findings
- Owner pick 2026-10-12 (research note `tasks/20261012-095949/RESEARCH.md`): shape 1. Types: `ScenarioConfig.world: ScenarioWorldType` with `serde(default, skip_serializing_if = is_void)`; `ScenarioWorldType::{Void, Generated(GeneratedWorldConfig)}`; `GeneratedWorldConfig { seed: Option<u32>, bounds: Option<ScenarioBoundsConfig> }`; `ScenarioBoundsConfig { position: Meters3, radius: Meters }`. `ScenarioRole::OpenWorld` is deleted. `ScenarioConfig::picker_lists()` = `role == Chapter && !world.needs_session()`, so fixed-seed chapters are picker rows. `OpenWorldSession` keeps its name.
- Areas and `OnEnter`/`OnExit` exist (`crates/nova_scenario/src/events.rs:76-160`; `CreateScenarioArea` in `crates/nova_scenario/src/actions/mod.rs:124-481`). The active window is a 160 km cube (`crates/nova_world_base/src/lib.rs:139-145`).

- Landed with `20261012-095950` (commit bd1838e75): `ScenarioBoundsConfig { position, radius }` exists on `GeneratedWorldConfig.bounds`, and lint already refuses a radius past `nova_world_base::OPEN_WORLD_BOUNDS_RADIUS_MAX` (`crates/nova_authoring/src/lint_walk.rs:315-368`). `Void` cannot carry bounds structurally. Remaining here: spawn the sphere as an addressable area with id `bounds` at load so `OnExit` fires, plus the example-mod bounded scenario.

- Owner decision 2026-10-12: a bounded harness keeps streaming sectors outside the bounds (visible past the edge, the player is only subject to the authored `OnExit`); no streaming-window clamp. The radius lint (half the active window edge) keeps the whole sphere inside streamed cells.

- Landed 2026-10-12 in commit 105c84214: `configure_scenario_shape` (`crates/nova_scenario/src/loader/wake.rs:46-66`) queues `ScenarioAreaConfig::bounds` under `BOUNDS_AREA_ID` before OnStart; lint Error for a handler spawning `bounds`, Warn for a `bounds` no OnExit reads; example mod 1.5.0 ships `example_bounds` (seed 1701, 2 km, Defeat on exit); `system_world_bounds` proved one OnExit, the outcome, and 125 sectors still standing. The bounds-on-Void rule is moot (structurally impossible).

## Delivery gate
- Reuse the area type for bounds; lint refuses bounds larger than the active window and bounds on a `Void` scenario. Decide with the owner whether a bounded harness streams sectors outside the bounds (visible but unreachable) or clamps the window; show both consequences first.

## Verification
- A probe scenario with bounds on a generated world: fly out, `OnExit` fires once, the authored outcome shows; lint tests for both refusals.

## Done when
- One bounded harness ships as an asserted example and the lint rules are in the content lint.
