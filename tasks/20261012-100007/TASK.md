# Stations: Map shows station contacts, held jobs and the chart layer

- STATUS: CLOSED
- PRIORITY: 81
- TAGS: v0.16.0, stations

## User facts
- Epic `20261012-095958`. Decisions 2b and 9b (2026-10-12): held jobs and last-seen station info stay on the Map when undocked; the chart layer (sector grid, visited cells, cluster types, found stations and wrecks) lands here, with a visited-and-found record in the save from the first format bump.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Agent findings
- No visited-cell record exists apart from the frozen ledger; every retired cell is a `Visited` record (`crates/nova_world/src/frozen.rs:270-279`). `WorldSaveState` fields at `crates/nova_world_base/src/save/mod.rs:103-117`.

- Owner decision 2026-10-13 (survey by orchestrator, all eight items approved as proposed):
  1. Visited cells stored: the set of `SectorCoord` the player entered, fed by `CurrentSector` changes (the frozen ledger holds every streamed cell, about 125 per save, so it cannot be the source). Save format 6 -> 7.
  2. A station is found on sensor contact; its last-seen summary is written on dock and refreshed on undock. Seen but never docked shows no stock.
  3. Summary: name, living or dead, services, trade stock, the player's stored items there. Item ids pass the saved-item load check.
  4. Wrecks: derelict hulls only, keyed by `GeneratedBodyId`, found on sensor contact. Wreck fragments and destroyed roots are out.
  5. Cluster type stored per visited cell at entry; `ClusterType` moves down to `nova_gameplay` so the Map reads it.
  6. `Chart` is a session resource in `nova_gameplay`, saved as a `WorldSaveState` field; web keeps it per session only.
  7. No jobs slot: task 20261012-100023 adds its own save field (overrides the gate's typed-slot line).
  8. The Map gets a chart view toggle: grid, visited cells, stations and wrecks as meshes on map layer 21, labels as UI; the local contact view stays.
  Changelog: edit the existing **(breaking)** world-save entry to format 7.

- Orchestrator close 2026-10-13: landed in eb5126382 (Chart resource in `crates/nova_gameplay/src/chart.rs`, `ClusterType` moved there, feeding systems in `crates/nova_world_base/src/chart.rs`, save format 7 with chart item check) and 00744cb53 (Map chart view on `C` plus a footer Chart button, `examples/systems/system_world_chart.rs`). Per-cell rule: a visited cell keeps every cluster type with a body in it. Live lavapipe run passed (3 entered cells, station summary kept across save and load). `content lint` re-run: 0 errors. Screenshot reviewed: grid, visited cells, STN-1 and its last-seen panel render. Gaps: no wreck found on the proof walk (wreck marks unproven live), no permanent proof for the Retry clear, panel summary text is low contrast and lists raw item ids.

## Delivery gate
- A `Chart` save field (visited coords, found station ids with last-seen stock summary, found wreck ids); the Map pane renders it as a layer. Jobs are a later strand; leave a typed slot, no placeholder data.

## Verification
- Asserted example: visit three sectors, find a station, save, load, the chart shows the three cells and the station with its last-seen summary.

## Done when
- The chart layer is on the Map, in the save, and the format bump is in the changelog as **(breaking)**.
