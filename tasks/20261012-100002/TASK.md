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

## Landed
- `DockAdmission::refusal` is the one admission point; a station answers by `station_admission(allegiance, derelict, occupied)`: `StationOccupied` (the `DockedShip` on its root, the one-connection-per-hull rule the port search already enforces), `EnemyStation` for a living Enemy station, admitted otherwise and for every derelict. `FlightVerbHints::dock_refusal` carries `<NAME> REFUSES DOCKING` / `<NAME> DOCK OCCUPIED`; the mode chip shows it, the DOCK chip reads `DOCK REFUSED`.
- `DockedHelmType::Station(station)` is set at capture by `helm_on_capture` for a living station (no joint change); `HELM` is refused (`helm_request_admitted`), the HUD reads `STATION HELM`, the Map GOTO note reads `STATION HELM`. A dead station opens on `Held(player)`. A station drives with its bridge's torque as the helm wrench and no thrust, so a station-held pair stays put.
- Retire: the player is always inside the active window, so the player's pair never retires. A docked NPC pair at retirement stays the `freeze_body` fault; nothing on master docks an NPC at a station (`DOCK` is a player verb, no AI or scripted order issues a `DockingConnectionRequest`), so no handling was added.
- Proofs: unit tests in `docking_section/tests.rs` (admission by side, dead and occupied; helm on capture; the station rule through the real admission and request); `system_station_docking` (living station holds the helm under a held burn, dead station hands it over, Enemy station refuses on the HUD).

