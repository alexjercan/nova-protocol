# Charters: discovery log and charter 0

- STATUS: CLOSED
- PRIORITY: 53
- TAGS: v0.16.0, charters

## User facts
- Epic `20261012-100030`. A discovery log on the Map lists found stations, wrecks and clusters with when and where; charter 0 is the shakedown that points a new player at the nearest station.

## Agent findings
- The chart layer and its save field land in the stations strand; charters are authored content over the open-world events from the world-first strand.

## Delivery gate
- The log is derived from the chart field, no second record; charter 0 is a harness objective, authored as content, completed by `OnDocked` at any living station.

## Verification
- Asserted example: a fresh start shows charter 0, docking completes it, the log shows the station.

## Done when
- Charter 0 ships as content and the log is on the Map.

## Agent decisions 2026-10-17
- When is the play-time clock (`WorldClock`), stamped inside the chart record: `ChartCell.entered_at`, `ChartStation.found_at`, `ChartWreck.found_at`. `Chart::log` derives the discovery log from those stamps; there is no second record. Save format 18.
- The Map's log shows the newest 12 finds in the side panel; older finds stay reachable through the cells and marks on the grid. A log row reads `h:mm:ss  CODE  (x, y, z)`, with `CELL` as a visited cell's code.
- A visited cell is selectable; its panel names its cluster words, its entry time and the finds in it. Cells are striped one colour per cluster type and tagged with words, not two-letter tags; the legend folds the three station states into one entry.
- Charter 0 is posted on `OnStart` only. A Load restores the saved objective and the spent handlers, so `OnResume` posts nothing.
- The pointer (`nova_world_base::charter`) marks the nearest living, non-Enemy streamed station with `STATION` and remembers it in `GeneratedMarks`; with none in the window there is no mark until one streams in.
- Owner decision: the completing `OnDocked` filter is `type_name: living_station, other_id: player`. The docking payload gained the docked-with hull's `type_name`, and a station wears `living_station` or `derelict_station` (spawn, runtime turn and thaw) instead of `spaceship`. Consequence: a dock at a dead station or a wreck does not complete the charter, and no shipped filter read a station as `spaceship`. Proof: `nova_authoring` test `only_the_players_dock_at_a_living_station_completes_charter_zero` (derelict does not complete, living does) and `station_derelict` asserts the type name after the turret loss and after a thaw.
- The proof extends `system_world_chart` with three claims rather than adding a range.

## Current state (superseded 2026-10-19)
- Charter 0 was removed by `4c885d33d` after this child closed. The old authored content, pointer and `only_the_players_dock_at_a_living_station_completes_charter_zero` proof no longer exist. The historical delivery and done-when notes above describe what landed at the time, not what ships now.
- The discovery log still derives from the chart. The epic's current acceptance is the six role goals as optional saved milestones, visible on the Map; no charter-0 objective or ring refresh is required. See `20261012-100030` for that superseding owner decision.
