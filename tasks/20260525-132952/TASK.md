# Remove LoadScenarioId

- STATUS: CLOSED
- PRIORITY: 80
- TAGS: v0.3.1, refactor

Replace with direct LoadScenario by config. Menu enumerates ids and calls LoadScenario by config. Legacy #109.

## Resolution (CLOSED - already resolved)

`LoadScenarioId` does not exist anywhere in the workspace. Scenarios are already loaded
by config: the `LoadScenario(ScenarioConfig)` event is the only load trigger
(nova_scenario/src/loader.rs), and callers look a config up by id from the
`GameScenarios` resource and pass it directly - e.g. examples/03_scenario.rs does
`scenarios.get("asteroid_field")` then `commands.trigger(LoadScenario(cfg.clone()))`,
and the editor's NextScenario handling resolves the id to a config before triggering
LoadScenario. The intended design (load by config, ids only used to look configs up) is
already in place. Closed as already-resolved.
