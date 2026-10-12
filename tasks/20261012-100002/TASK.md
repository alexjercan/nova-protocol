# Stations: docking admission and helm ownership

- STATUS: OPEN
- PRIORITY: 85
- TAGS: v0.16.0,stations

## User facts
- Epic `20261012-095958`. Decisions 3b and 6b (2026-10-12): one connection per hull stays; a station exposes several ports but admits one player connection; docked to a living station the station owns the helm and the player cannot take it; a dead station is a derelict and hands the helm to the docked player; a station of an Enemy civilization refuses admission.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Agent findings
- Docking owner `crates/nova_ship/src/sections/docking_section/` (`DockingSectionConfig` at `mod.rs:66-118`); the pair flies as one `DockedAssembly`; helm Neutral then Held today. Allegiance is `Player / Enemy / Neutral` (`crates/nova_gameplay/src/relations.rs:37-44`).

## Delivery gate
- Admission reads the station's civilization allegiance; refusal is a visible HUD line, not a silent no-capture. Helm rule as one admission branch, no joint change. State what happens to a docked pair at retire (today a docked ship at retirement is a fault, `crates/nova_world/src/frozen.rs:442-451`).

## Verification
- Asserted examples: dock at a living station, helm input does nothing; dock at a dead station, helm passes; approach an Enemy station, capture refuses with the HUD line.

## Done when
- The three cases are permanent proofs and the Part 6 Docking row of the design page matches the code.
