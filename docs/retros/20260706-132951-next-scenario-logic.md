# Retro: next_scenario logic cleanup (task 20260525-132951)

## What was asked
Cleaner implementation of the next_scenario switch.

## What happened
The switch in `state_to_world_system` consumed the `next_scenario` request by *side
effect*: it relied on the LoadScenario/UnloadScenario observers calling `world.clear()`
to reset the flag. Made the consumption local and explicit (take the request, set the
flag to None up front), de-shadowed the variables, and used `Option::filter` + `match`.

## Lessons
- Consumption-by-side-effect is a real smell: a one-shot request that is cleared in a
  *different* function will silently re-fire if that other function ever stops clearing
  it. Consume it where you read it.
- Variable shadowing (`next_scenario` the request vs `next_scenario` the resolved config)
  actively hides bugs; distinct names (`request` / `config`) make the two roles obvious.
