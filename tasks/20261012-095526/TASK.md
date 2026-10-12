# Epic: world first, scenarios as the harness

- STATUS: OPEN
- PRIORITY: 99
- TAGS: v0.16.0,epic,world-first

## User facts
- Owner direction 2026-10-12: the open world is the main mode and the first citizen. Scenarios and `nova_events` are a harness put on top of it so the world works in bounded form. Story mode stays as it is; every shipped chapter keeps running in a void.
- Highest priority of the v0.16.0 game expansion; done before any station work. Decision 14 on the design page is still open for discussion; the first child task closes it.
- Design page: `DESIGN.html` in this folder (the owner-reviewed "Open-World Progression Directions" page, version 19). Research behind it: `RESEARCH-CODEBASE.md`, `RESEARCH-DESIGN.md`.

## Agent findings
- Today the dependency points the other way. New Game inserts `OpenWorldSession { seed }` and loads the `open_world` scenario named by the base bundle (`crates/nova_menu/src/world_setup.rs:362-365`, `assets/base/base.bundle.ron:304`); that scenario says "the world is the scenario" (`crates/nova_authoring/src/base_content/scenarios/open_world.rs:5-7`).
- The generator arms only while a `role: OpenWorld` scenario is live with a session and exactly one player (`crates/nova_world_base/src/lib.rs:165-167`, `:248-252`); any other scenario disarms it.
- The role is read in four places: the arming system, scenario lint (`crates/nova_scenario/src/lint/scenario.rs:88`), probe snapshots (`crates/nova_probe/src/capabilities/snapshot.rs:437`, `:2202`) and world-base tests (`crates/nova_world_base/src/tests.rs:348-495`).
- Scenario state is not in the save (`crates/nova_world_base/src/save/mod.rs:103-117`); generated bodies are scoped but not addressable (`crates/nova_world/src/streaming.rs:176-182`); no event exists for a sector crossing, mine pulse, pickup, trade or credits change (`crates/nova_scenario/src/events.rs:76-160`).

## Strand
Children carry the `World first:` title prefix and the `world-first` tag. Order is the priority order.
1. Research note and decision 14 shape.
2. `world` block on the scenario; arming from the block; `ScenarioRole::OpenWorld` dies.
3. New Game harness takes the wizard seed; chapters and backdrops declare `Void`.
4. Bounds area on a harness.
5. Open-world events in `nova_events`.
6. Addressable generated bodies.
7. Scenario state in the world save.

## Done when
- Every child is CLOSED, the open world starts through the same loader as a chosen scenario, no code reads `ScenarioRole::OpenWorld`, and a bounded scenario on a generated world runs end to end under `nova-probe`.

## Children
- `20261012-095949` World first: research note and decision 14 shape
- `20261012-095950` World first: world block on the scenario and arming from it
- `20261012-095952` World first: New Game harness takes the wizard seed; chapters declare Void
- `20261012-095953` World first: bounds area on a harness
- `20261012-095954` World first: open-world events in nova_events
- `20261012-095955` World first: addressable generated bodies
- `20261012-095956` World first: scenario state in the world save
