# Sandbox: mark a ship design as a station

- STATUS: OPEN
- PRIORITY: 93
- TAGS: v0.16.0,stations

## User facts
- Owner request 2026-10-13: "should be able to create a Station in sandbox (mark a ship as station basically)".

## Agent findings
- Stations are `Content::Station(StationDesign { id, name, about, hull: ShipDesign, services, storage_cap_g, stock, credits })`; station lint requires a connected hull, a docking port, a controller and no thruster (`crates/nova_scenario/src/lint/station.rs`). The sandbox/editor entry points are not yet surveyed.

## Delivery gate
- Survey the sandbox build flow, then propose the UI (a "station" toggle on a design) and what it writes (a StationDesign with which services/stock defaults), and how lint failures (thrusters present, no port) are shown. Stop for the owner on defaults.

## Verification
- Asserted example: build a hull in sandbox, mark it as a station, spawn it, it behaves as a station (no thrust, levels, admits docking).

## Done when
- A sandbox player can mark a design as a station and fly to it.
