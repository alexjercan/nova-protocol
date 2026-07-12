# Reset scenario progress (salvaged crates) on scenario start/exit

- STATUS: OPEN
- PRIORITY: 60
- TAGS: v0.5.0, bug, scenario

## Goal

Playtest bug (2026-07-12): exiting the main scenario to the main menu and
playing again keeps the previous run's progress - the salvaged crate tally (and
likely other scenario variables / beat state) persists instead of resetting.
Starting (or exiting) a scenario should reset its progress so a replay begins
fresh.

## Notes / where to look

- The salvage tally lives in the scenario event world's variables
  (`NovaEventWorld`, bevy_common_systems event system - see the shakedown
  scenario's crate count and the OnUpdate/variable machinery referenced in
  `nova_scenario/src/loader.rs`). `teardown_scenario_entities` already calls
  `world.clear()` on unload/reload and clears `HintEmphasis`; confirm whether
  `world.clear()` actually resets scenario VARIABLES (the crate tally) or only
  entities/handlers - the bug suggests the variables (or `GameObjectives`
  progress) survive.
- Also check `GameObjectives` (bcs ObjectivesPlugin) and any beat/progress
  resource: they may need an explicit reset on `LoadScenario` / `UnloadScenario`
  the same way `HintEmphasis` is cleared (state-reset class, task 20260712-125342
  / the emphasis reset in loader.rs).
- Repro: New Game -> salvage some crates -> back to main menu -> New Game again
  -> observe the tally starts non-zero / the salvage beat is already partly done.

## Steps (to be planned)

- [ ] Reproduce and identify exactly which state persists (event-world variable,
      GameObjectives, or a beat resource).
- [ ] Reset it on scenario load (and/or unload), alongside the existing
      `world.clear()` + emphasis clear in `teardown_scenario_entities`.
- [ ] Regression test: load a scenario, mutate the progress variable, reload,
      assert it is back to zero (driven through the real LoadScenario observer,
      like the `teardown_clears_hint_emphasis` test).
