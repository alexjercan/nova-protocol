# NOTES - remove the scatter_density graphics-preset lever

## What changed and why

Removed `scatter_density` from `GraphicsBudget` entirely. The decision (a
stronger form of the task's option (b)): asteroids, rocks and debris are gameplay
content, not a graphics-budget knob, so no quality tier should thin them - not
even the procedural `ScatterObjects` fields the lever actually reached.

Concretely:

- `crates/nova_gameplay/src/settings.rs`
  - Dropped the `scatter_density: f32` field from `GraphicsBudget`; `particles:
    bool` is now the sole lever.
  - `for_quality` keeps only `particles` per tier (High/Medium on, Low off).
  - Deleted `GraphicsBudget::scaled_count`.
  - Updated the module docs, the `GraphicsQuality::Low` doc, and the
    `apply_graphics_quality` doc to stop describing a scatter/density lever, and
    removed the "provisional multipliers pending the baseline" language (there
    are no fractions left to tune).
  - Test `each_quality_tier_maps_to_a_distinct_graphics_budget` ->
    `graphics_budget_gates_particles_only_by_tier`, asserting on `particles`
    only. Deleted `scaled_count_thins_but_never_empties_a_field`.
- `crates/nova_scenario/src/actions.rs`
  - `ScatterObjectsConfig::action` now spawns `self.count` directly instead of
    `world.graphics_budget().scaled_count(self.count)`.
  - Repurposed the old thinning test into a regression,
    `scatter_action_ignores_graphics_budget`: even with the Low budget carried
    in, scatter spawns the full authored count. This pins the removal so a future
    change can't quietly re-introduce thinning.
- `crates/nova_scenario/src/world.rs`
  - Removed the now-dead `graphics_budget` field on `NovaEventWorld`, its
    `graphics_budget()` accessor, and the budget carry in `world_to_state_system`.
    `world_to_state_system` stays as an empty required `EventWorld` method (no
    default in the trait) with a comment explaining why.

Companion task 20260718-004809 (tune the `for_quality` fractions from the
baseline) is closed WON'T DO: with `scatter_density` gone there are no fractions
left to tune. Any future render-scale lever tuning gets its own task.

## Correction to the baseline premise

The task (from the baseline report) claimed the lever "never fires for the
shipped scenes" because they place asteroids via authored `SpawnScenarioObject`.
That is only true for `shakedown_run` (20 hand-placed, 0 scatter). `asteroid_field`
and `broadside` DO have a real `ScatterObjects((count: 20 / 24))` block that the
lever thinned on Low (20 -> 10, 24 -> 12). So removal is not a no-op for players:
it restores full asteroid/rock counts on Low in those two scenes, which is the
intended gameplay behaviour.

## Difficulties

- `cargo check -p nova_gameplay -p nova_scenario` passed, but `cargo test -p
  nova_scenario --lib` failed to *compile* with `ScenarioConfig: Serialize`
  errors in `loader.rs`. This is unrelated to the change: those round-trip tests
  need the `serde` feature, which is off when the crate is tested in isolation.
  CI runs `cargo test --workspace --features debug`, which enables
  `nova_scenario/serde` transitively. Re-ran with `--features serde` and the
  scatter tests passed. Worth remembering: test nova_scenario with `serde` (or
  via the workspace), never `-p nova_scenario --lib` bare.

## Verification

- `cargo fmt --all` clean.
- `cargo check -p nova_gameplay -p nova_scenario` clean, no warnings from our code.
- `cargo test -p nova_gameplay --lib settings::` -> 5 passed.
- `cargo test -p nova_scenario --lib scatter --features serde` -> 7 passed,
  including `scatter_action_ignores_graphics_budget`.
- Full workspace suite deferred to CI (`cargo test --workspace --features debug`)
  per the project's local-test policy.
