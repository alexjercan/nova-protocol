# Stations: Map shows station contacts, held jobs and the chart layer

- STATUS: OPEN
- PRIORITY: 81
- TAGS: v0.16.0,stations

## User facts
- Epic `20261012-095958`. Decisions 2b and 9b (2026-10-12): held jobs and last-seen station info stay on the Map when undocked; the chart layer (sector grid, visited cells, cluster types, found stations and wrecks) lands here, with a visited-and-found record in the save from the first format bump.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Agent findings
- No visited-cell record exists apart from the frozen ledger; every retired cell is a `Visited` record (`crates/nova_world/src/frozen.rs:270-279`). `WorldSaveState` fields at `crates/nova_world_base/src/save/mod.rs:103-117`.

## Delivery gate
- A `Chart` save field (visited coords, found station ids with last-seen stock summary, found wreck ids); the Map pane renders it as a layer. Jobs are a later strand; leave a typed slot, no placeholder data.

## Verification
- Asserted example: visit three sectors, find a station, save, load, the chart shows the three cells and the station with its last-seen summary.

## Done when
- The chart layer is on the Map, in the save, and the format bump is in the changelog as **(breaking)**.
