# Improve next_scenario logic

- STATUS: CLOSED
- PRIORITY: 80
- TAGS: v0.3.1, refactor

Cleaner implementation. Legacy #125.

## Steps

- [x] Review the next_scenario handling in NovaEventWorld::state_to_world_system.
- [x] Make request consumption explicit and one-shot; clarify names and linger.
- [x] Verify build --all-targets, clippy, fmt.

## Resolution (CLOSED)

Cleaned up the next-scenario switch in world.rs (state_to_world_system):

- It relied on the *side effect* of LoadScenario/UnloadScenario calling world.clear() to
  reset next_scenario. That is fragile: if the load path ever stopped clearing, the
  switch would re-fire every frame. Now the request is taken and next_scenario is set to
  None up front, so the switch fires exactly once regardless of what the load path does.
- Removed the confusing variable shadowing (the outer `next_scenario` request vs the
  inner resolved ScenarioConfig were both called `next_scenario`). Now `request` (the
  NextScenarioActionConfig) and `config` (the resolved ScenarioConfig).
- Replaced the nested if/else with `Option::filter(|r| !r.linger)` + a `match`, and gave
  the log lines clear, prefixed messages.
- Documented what `linger` means (keep the request pending without switching).

Behavior is unchanged for the normal case (switch to the next scenario, or unload if the
id is unknown) and for lingering (still held, not switched); the change is clarity plus
making the one-shot consumption explicit instead of incidental.

Verified: build --all-targets, clippy, fmt green.

Self-reflection: the real smell was consumption-by-side-effect. Spotting that the flag
was cleared somewhere else entirely (in the load observer) was the key - making the
consumption local and explicit is what makes the logic safe to reason about.
