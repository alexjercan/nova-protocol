# Sandbox: mark a ship design as a station

- STATUS: OPEN
- PRIORITY: 93
- TAGS: v0.16.0,stations

## User facts
- Owner request 2026-10-13: "should be able to create a Station in sandbox (mark a ship as station basically)".

## Agent findings
- Stations are `Content::Station(StationDesign { id, name, about, hull: ShipDesign, services, storage_cap_g, stock, credits })`; station lint requires a connected hull, a docking port, a controller and no thruster (`crates/nova_scenario/src/lint/station.rs`). The sandbox/editor entry points are not yet surveyed.

## Agent decision
Owner told the orchestrator to work the list without a stop at the gate. The agent chose the marked options and implemented them (2026-10-14).

- UI. Survey: the sandbox document is a node tree (`crates/nova_editor/src/node.rs` `ShipNode`); Play lowers it in `crates/nova_editor/src/scenario.rs` (`lower_fleet`, `sandbox_objects`, `setup_scenario`); Save/Open is `crates/nova_editor/src/bundle.rs`; the ship panel rows are `ship_rows` in `crates/nova_editor/src/inspect.rs`.
  - (A, chosen) A `Station` checkbox row on the ship inspector, stored as `ShipNode::station`. Consequence: the mark lives on the ship, survives Play and Save, and is visible where Driver is.
  - (B) A fourth Driver option `Station`. Consequence: it mixes who drives with what the hull is, and a dead station could not be said.
  - (C) A Ship-menu verb. Consequence: the state is hidden until the menu is opened.
- Driver. The ship you fly cannot be a station: marking a Player ship and choosing Player for a station are refused on the status line. Driver AI spawns a living station; driver None spawns a dead one (`derelict: true`).
- What it writes: a `StationDesign` with
  - id: the ship node id in a save (same id rule as the ship prototypes a save writes); `editor_sandbox_<node id>` on Play, registered into `GameStationDesigns` for the Play session (the content restart drops it, as it drops the sandbox scenario);
  - hull: the ship's design with `integrity.collapse_threshold` = `STATION_COLLAPSE_THRESHOLD` (0.5, moved from `nova_authoring` to `nova_scenario` so both use one value);
  - services: Rebuild, Refit, Trade, Storage, as every generated station has; storage cap 100 kg and credits 5000, as the waystation;
  - stock: the waystation's role goods (8 hull plates, 20 rations, 10 salvaged parts), each line cut to what fits the hull hold, so the spawn never overfills it;
  - side: the ship's allegiance, else Neutral (the sandbox default for an AI ship).
  - Alternatives: (B) empty stock and no credits. Consequence: Trade at a sandbox station has nothing to sell. (C) an editable services/stock panel. Consequence: a new editor surface for no current ask.
- Errors: `lint_station_design` is the one rule set.
  - (A, chosen) A read-only `Station Check` row under the toggle reads `ready` or the first fault (all faults in its hint); Play refuses on the status line, naming the station and its faults. Save still writes the file: it is the builder's work, and the content lint reports it when the mod loads.
  - (B) Refuse the toggle while the hull fails. Consequence: a builder cannot mark a station first and then build it.
  - (C) Spawn the hull as a ship when it fails. Consequence: a silent fallback that hides invalid authoring.
  - A fault that gets past Play (a rig that enters the state directly) leaves the design unregistered, so the scenario start gate refuses the range with its FAILED TO START report.
- Save format: the save is a mod bundle; a station writes `Content::Station` and a `Station` scenario object, both shipped formats. Open lifts a station object whose design is in the same file back into a ship node with `station` set. No format change.

## Delivery gate
- Survey the sandbox build flow, then propose the UI (a "station" toggle on a design) and what it writes (a StationDesign with which services/stock defaults), and how lint failures (thrusters present, no port) are shown. Stop for the owner on defaults.

## Verification
- Asserted example: build a hull in sandbox, mark it as a station, spawn it, it behaves as a station (no thrust, levels, admits docking).

## Done when
- A sandbox player can mark a design as a station and fly to it.
