# Notes: Build systems/: code-built fixtures for scenario grammar, the player path, and outcomes

Goal in one line: turn `gameplay/` into `systems/` - two renamed-and-deepened
runs plus one new run that composes the whole outcome arc on a ~50-line
code-built fixture instead of 8000 lines of campaign RON.

## What changes

Before: `examples/gameplay/` holds four runs. Two of them (`scenario`,
`playable`) already build their scenario in code and are the model. Two
(`broadside`, `lifeline`) load shipped story scenarios and assert story wave
timings and object ids.

After: `examples/systems/` holds three runs, all on code-built fixtures.
Nothing under `systems/` reads `assets/base/scenarios`. The two story runs are
deleted by `20260804-093910`, which must land AFTER this one so the composed
outcome path is never absent from the tree.

User-visible: `probe run systems` replaces `probe run gameplay`.

## Surfaces

| File | Why |
|-|-|
| `examples/gameplay/scenario.rs` (391 lines) -> `examples/systems/scenario_grammar.rs` | Already the model: `showcase(&GameAssets) -> ScenarioConfig` at :105 exercises `OnStart`/`OnDestroyed`/`OnUpdate`, entity-type and expression filters, variables and arithmetic. Rename + deepen with repeated rounds. |
| `examples/gameplay/playable.rs` (491 lines) -> `examples/systems/player_path.rs` | Already predicate-driven: lock, kill, travel-lock, GOTO, arrive. Rename + deepen: more rounds through the loop point. |
| `examples/systems/outcomes.rs` | NEW. The composed arc. |
| `examples/systems/fixtures.rs` or a shared module | NEW. The builder `fn`s, shaped so `sections/` and `stress/` can call them with a count knob. |
| `Cargo.toml` | Four `[[example]]` blocks under the `gameplay/` comment become three under `systems/`. |
| `tests/examples_smoke.rs` | `GAMEPLAY:43` -> `SYSTEMS`; `gameplay_reach_playing_without_panic:91` renamed. Atomic with the rename. |
| `crates/nova_scenario/src/loader/mod.rs` | Read-only. `ScenarioConfig:147`, `LoadScenario:223`, `ScenarioLoaded { scenario_id, handler_count, ... }`. |
| `crates/nova_menu/src/outcome.rs` | Read-only. `ScenarioOutcomeKind::{Victory, Defeat}` render "VICTORY"/"DEFEAT" with a "Continue"/"Retry" button (:58-63). That button is what `outcomes` must click. |
| `crates/nova_debug/src/harness.rs` | The predicates: `scenario_variable_is:137`, `section_gone:150`, `script_reports_done:173`, `player_ship_present:182`, `assert_scenario_loaded:226`. |

## Data and interfaces

The fixture builders. Note the count knob exists so `stress/many_bodies` and
`sections/` reuse them - it is not speculative:

```rust
/// N asteroids in a deterministic ring plus the seeded variables.
pub fn asteroid_ring(assets: &GameAssets, count: usize) -> ScenarioConfig;

/// Scenario A: one objective, one killable hostile, one player ship.
/// OnDestroyed(hostile) -> objective complete + Victory + CHECKPOINT.
/// Player death -> Defeat.
fn outcome_probe_a(assets: &GameAssets) -> ScenarioConfig;

/// Scenario B: the chain target. One object, one trivially completable
/// objective, so the chain's arrival is observable.
fn outcome_probe_b(assets: &GameAssets) -> ScenarioConfig;
```

Driving is `AutopilotPlugin<GameStates>` (`crates/nova_autopilot/src/autopilot.rs:140`)
with `.step(name).enter(state).on_enter(f).each(f).until(pred).deadline(s).add()`,
plus `click_at` / `move_cursor` from `nova_autopilot::input` for the real
button press.

## Sketches

Illustrative only.

```diff
+// examples/systems/outcomes.rs
+nova_autopilot()
+    .step("die").on_enter(scuttle_player).until(outcome_is(Defeat)).add()
+    .step("retry").on_enter(click_retry_button).until(hostile_respawned()).add()
+    .step("kill").each(fire_at_hostile).until(section_gone("hostile")).add()
+    .step("victory").until(scenario_variable_is("objective_done", 1.0)).add()
+    .step("continue").on_enter(click_continue_button)
+        .until(scenario_loaded_is("outcome_probe_b")).add()
```

```diff
-fn showcase(game_assets: &GameAssets) -> ScenarioConfig {
+pub fn asteroid_ring(game_assets: &GameAssets, count: usize) -> ScenarioConfig {
-    let mut start_actions = (0..ASTEROID_COUNT)
+    let mut start_actions = (0..count)
```

## Shape

```
GameAssetsStates::Loaded
        |
        v
  fixtures::outcome_probe_a()  ->  trigger LoadScenario(cfg)
        |
        v
   GameStates::Playing  <-------------------------+
        |                                          |
   [die] -> ScenarioOutcomeKind::Defeat            |
        |     overlay: "DEFEAT" + [Retry] --click--+  (clean reload:
        v                                              hostile back,
   [kill hostile] -> objective + CHECKPOINT             tally zeroed)
        |
        v
   overlay: "VICTORY" + [Continue] --click--> outcome_probe_b loaded
```

## Consequences and open questions

- The four SYSTEMS are already pinned headlessly (`nova_menu/src/tests/{outcome,pause}.rs`,
  `nova_scenario/src/loader/lifecycle.rs`, `nova_assets/tests/*`). What this run
  adds is the COMPOSED, rendered, click-the-real-button path. Size it as an
  end-to-end composition, not as emergency coverage - and if it proves
  expensive, that framing is what makes trimming it a legitimate option.
- RESOLVED (owner, 2026-08-04): fixtures are built LOCALLY here, not as a
  shared abstraction. 093950 builds its own too; 094006 is the third caller and
  extracts the shared `fn` from three visible shapes. Designing the signature
  here would be designing from one caller - against the standing YAGNI rule -
  and the knobs would likely be wrong.
- OPEN: how does `outcomes` kill the player? Scuttling needs either a scenario
  action that damages the player or a world-mutating `on_enter`. The former
  keeps the fixture declarative; the latter is simpler. Not resolved by reading.
- RESOLVED (owner, 2026-08-04): do NOT click. Trigger the button's `Activate`
  on the target entity. This run's subject is the outcome chain; the menu is
  `ui/`'s subject. Pixel coordinates would couple the sprint's most fragile
  test to overlay layout for no coverage gain - `ui/` already proves the
  buttons are reachable and hittable.
- Consequence, worth stating: this run therefore does NOT prove the outcome
  buttons are clickable. Nothing does, unless `ui/` covers the overlay. If that
  gap matters, it belongs in `20260804-094021`, not here.
- OPEN: CHECKPOINT. The task names "objective + CHECKPOINT" as an assertable
  beat, but nothing read so far shows what a checkpoint IS in code. Needs a
  pass over `nova_scenario` before the Steps are trustworthy.
- `systems/` carries no fps window, so these runs are free to be short and
  assertion-dense. That is a licence to assert more, not to run longer.
