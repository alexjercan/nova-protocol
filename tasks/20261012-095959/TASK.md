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

- Survey 2026-10-12: a new `Content` kind needs arms in `kind()`/`id()` (`crates/nova_modding/src/lib.rs:143-168`), `merge_content_item` (`crates/nova_assets/src/merge.rs:950-996`), `section_errors` (`:671-713`), the lint bucket and bundle loop (`crates/nova_authoring/src/lint_walk.rs:122-138`, `:306`), `BaseContent` (`base_content/mod.rs:25-46`) and `content_files` (`generation.rs:206-246`); duplicate ids and the catalog digest are handled per kind already (`merge.rs:922-934`, `:789-820`). `ShipDesign { sections, integrity, presentation }` holds what spawns share, per-spawn fields live on `SpaceshipConfig` (`ship_design.rs:67-91`, `spaceship.rs:477-524`). A controller section parks and holds attitude with no thrusters (`controller_section.rs:521-533`, `:481-502`). `FrozenBody` already stores transform and motion for every type (`frozen.rs:342-356`); a new variant touches `freeze_body` (`:497-510`), `thaw_record` (`:545-561`), the saved-item and saved-stock checks (`save/mod.rs:423-431`, `:481-500`). Docking, admission, undock and trade all query `SpaceshipRootMarker` (449 refs); a partner trades when not lootable or neutralized (`inventory/app.rs:1092-1104`); no trader flag, no service concept, no invulnerability mechanism exists.
- Owner decision 2026-10-12: approved field list. `StationDesignId` (transparent, `GameStationDesigns`); `StationDesign { id, name, about, hull: ShipDesign (embedded, fork 1a), services: Vec<StationServiceType> (closed enum Rebuild/Refit/Trade/Storage/Jobs/Roster, fork 2a), storage_cap_g: u32, stock: ShipInventoryStock (authored seed, fork 3a), credits: u32 }`. Per spawn: `StationConfig { design, allegiance, derelict, initial_velocity }`. Spawn arm builds the ship bundle with `controller: None`, only `dock_enabled`, `lootable: derelict`, plus `StationMarker(StationDesignId)` and `StationStorage`; root keeps `SpaceshipRootMarker` (fork 4a). `FrozenBodyType::Station(Box<FrozenStation { design, ship: FrozenShip, storage }>)`, no repeated pose/velocity; `WORLD_SAVE_FORMAT` 5, no migration. Lint Errors: unknown/duplicate/reserved section id; no Docking section; empty services (duplicate service Warn); any Thruster section; no Controller section; unknown stock item or stock over the hull hold; `storage_cap_g > 0` iff Storage listed; spawn naming an unknown design id. Section invulnerability is DEFERRED to task `20261012-100000`.

## Delivery gate
- Propose `StationDesignId`, `StationDesign { id, name, about, sections, services, storage_cap_g }` as content, the object kind, the frozen type with `pose + velocity`, and the lint rules (unknown section id, no docking section, empty services). Show the before/after call graph for load, spawn, retire and thaw. Stop for the owner on the field list.

## Verification
- A station spawns from content in an asserted example, retires to a frozen body, thaws with the same pose and velocity; lint refuses the three invalid cases.

## Done when
- One base station design lints, loads, freezes and thaws; the example mod can declare one.
