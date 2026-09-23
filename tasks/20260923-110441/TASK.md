# Investigate asteroid and planet vulnerability ownership

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog, scenario, destruction, world, research

## User facts

- The `invulnerable: bool` field on `AsteroidConfig` and `PlanetConfig` is awkward and should be reconsidered.
- `PlanetConfig` currently exposes `false`, but lint and loading refuse it because planets do not implement destruction.
- The first delivery step is investigation. Implement a replacement only if the investigation finds a feasible, explicit design.

## Decisions

- Do not preselect a replacement for the boolean in this task specification.
- Do not make `PlanetConfig::invulnerable = false` load successfully until planet damage, destruction, gravity-well teardown, and scenario consequences are real and proven.
- Invalid or unsupported authoring must keep failing loudly. Do not add a fallback that silently makes a body invulnerable.
- Stop for owner approval after research identifies the owning interface, names, defaults, failure policy, and migration impact. Implementation follows only an approved feasible design.

## Agent findings

- `AsteroidConfig::invulnerable` changes real construction: destructible rocks receive carve and damage state, while invulnerable rocks remain collidable without health or carve fields (`crates/nova_scenario/src/objects/asteroid.rs`).
- `PlanetConfig::invulnerable` is currently required to be `true`; scenario lint and `planet_scenario_object_prepared` both reject `false` because planets receive no damage marks or destruction events (`crates/nova_scenario/src/objects/planet_type.rs`, `planet.rs`, and `lint/scenario.rs`).
- Planet destruction also affects gravity-well lifetime, orbit directives, collision ownership, sensors, objectives, cleanup, and authored assumptions. Allowing the value is therefore not a field-only change.
- Base content and examples author both vulnerable and invulnerable asteroids, while authored planets currently use only the supported invulnerable behavior.

## Research

- Trace every authoring, lint, load, spawn, damage, carve, health, destruction, gravity, orbit, objective, cleanup, save, world-generation, editor, and documentation consumer of asteroid and planet vulnerability.
- State the behavior represented by the current boolean for each body type. Identify where one name hides different capabilities.
- Compare explicit alternatives, including separate object/config variants, a named durability or destruction policy, marker-owned behavior, and removing an unchoosable planet field.
- Determine whether asteroid and planet configuration should share one abstraction or remain distinct because their destruction pipelines differ.
- Reproduce and record the current planet `false` lint and load refusals before proposing a change.
- Define what a destructible planet must do when damaged and destroyed, including collider, render body, gravity well, orbiting entities, objectives, IDs, and cleanup.
- Determine shipped RON migration requirements and effects on generated content and editor controls.
- End research with options, one consequence each, a recommendation, and the smallest proof for the recommended behavior.

## Potential implementation

Proceed only after owner approval of the research decision.

If feasible, change the owning scenario interface first, migrate every caller and shipped format, delete the boolean or obsolete path, update lint and editor behavior, and add only the stable proofs named by the research. If destructible planets are not feasible in this slice, remove or replace the misleading unchoosable authoring surface without pretending support exists.

## Verification

Research must preserve before artifacts for the current planet lint and load refusals. Any implementation must prove the approved asteroid modes and the full approved planet lifecycle, not only successful parsing or spawning. It must also prove gravity and orbit behavior after destruction and reject unsupported authoring before gameplay begins.

## Done when

- Research maps the complete ownership and failure surface with code evidence.
- The owner has approved an explicit interface and migration policy, or the task records that implementation is not feasible and why.
- If implementation is approved, obsolete fields and paths are removed, all callers and shipped content are migrated, and focused lifecycle proofs pass.
- Skipped implementation is not reported as completed behavior.
