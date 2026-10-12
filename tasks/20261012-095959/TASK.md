# Stations: Content::Station kind, object kind and frozen body type

- STATUS: OPEN
- PRIORITY: 88
- TAGS: v0.16.0,stations

## User facts
- Epic `20261012-095958`. Decision 1 (2026-10-12): a new `Content::Station` kind with its own `ScenarioObjectKind::Station` and `FrozenBodyType::Station`, never a thrusterless `Spaceship` in disguise. Mods must be able to add stations.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Agent findings
- Content kinds: `Content::{Section, Scenario, Campaign, Style, Ship, Lesson, UiTheme, Item}`; adding one is an enum variant plus a router arm (`crates/nova_modding/src/lib.rs:78-137`, `:71-76`). Objects: `Anchor, Asteroid, Spaceship, Beacon, Light, Planet` (`crates/nova_scenario/src/actions/spawn.rs:127-143`). Frozen body types (`crates/nova_world/src/frozen.rs:419-438`).
- Precedent for a hull without thrusters: the gauntlet Battery (`crates/nova_authoring/src/base_content/scenarios/main_menu/gauntlet.rs:100-107`).

## Delivery gate
- Propose `StationDesignId`, `StationDesign { id, name, about, sections, services, storage_cap_g }` as content, the object kind, the frozen type with `pose + velocity`, and the lint rules (unknown section id, no docking section, empty services). Show the before/after call graph for load, spawn, retire and thaw. Stop for the owner on the field list.

## Verification
- A station spawns from content in an asserted example, retires to a frozen body, thaws with the same pose and velocity; lint refuses the three invalid cases.

## Done when
- One base station design lints, loads, freezes and thaws; the example mod can declare one.
