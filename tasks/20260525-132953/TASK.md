# Improve despawning and entity management in scenarios

- STATUS: CLOSED
- PRIORITY: 80
- TAGS: v0.3.1, refactor

Consistent teardown across scenarios. Legacy #104.

## Steps

- [x] Find where scenario teardown happens and whether it is consistent.
- [x] Extract the shared teardown so load and unload cannot drift apart.
- [x] Verify build --all-targets, clippy, fmt.

## Resolution (CLOSED)

Scenario teardown was duplicated: `unload_scenario` and `on_load_scenario` each inlined
the same "clear the NovaEventWorld, then despawn every ScenarioScopedMarker entity"
sequence. Duplicated teardown is how the two paths drift apart (someone fixes a leak in
one and forgets the other).

Extracted a single `teardown_scenario_entities(commands, q_scoped, world)` helper and
call it from both. Now unload and load-over-existing tear the previous scenario down
through the exact same code; unload additionally clears CurrentScenario to None, and load
sets it to the incoming scenario, which is the only intended difference between them.

This complements the cleanup contract documented for task 20260525-132939 (the five
buckets that guarantee no leftover entities): that task proved *what* gets cleaned up;
this one makes the *teardown routine itself* a single source of truth.

Behavior is unchanged (despawn is recursive, so children still go with their scoped
roots). Verified: build --all-targets, clippy, fmt green.

Self-reflection: small DRY extraction, but teardown is exactly the kind of code where
duplication is dangerous - the two copies were already subtly different in shape (order
of current_scenario reset vs despawn), and unifying them removes a class of future
inconsistency bugs.
