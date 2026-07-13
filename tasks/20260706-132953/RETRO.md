# Retro: consistent scenario teardown (task 20260525-132953)

## What was asked
Consistent teardown across scenarios.

## What happened
unload_scenario and on_load_scenario each inlined the same "clear the event world +
despawn all ScenarioScopedMarker entities" sequence. Extracted a single
teardown_scenario_entities() helper used by both, so the paths cannot drift.

## Lessons
- Teardown/cleanup is the highest-value place to kill duplication: the two copies were
  already subtly different in shape, and a leak fixed in one copy silently persists in
  the other. One helper = one place to be correct.
- Pairs with the 132939 cleanup contract: that task defined *what* must be cleaned;
  this one makes the *routine* that cleans it a single source of truth.
