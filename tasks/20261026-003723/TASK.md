# Make generated ship traffic visibly purposeful

- STATUS: OPEN
- PRIORITY: 70
- TAGS: v0.17.0

## User facts
- Meaningful moving ambient traffic in v0.17.0; specification only, no active-radius change.
## Agent findings
- Intact generated ships already have AI; seeded patrol has four 1500 m waypoints, 30-60 s holds, empty if fewer than two fit (`crates/nova_world_base/src/sector_ships.rs:65-87,149-153,358-393`). Job escort/distress already spawn bodies (`crates/nova_world_base/src/jobs.rs:460-683`). Shipped active radius 2 is 5x5x5 (`crates/nova_world_base/src/lib.rs:190,362`). More bodies alone is not new activity.
## Decisions needed
- Which observable purpose and interaction, relationship to patrol/jobs, stable IDs and ownership, seeded rate/density, hostility and route safety, freeze/thaw/save and performance budget. No algorithm, rate or radius approved.
## Delivery
- End: at least one recognizable non-job movement with a player-visible consequence. Dies: superseded patrol rule only if replaced. May break: AI orders, identities, encounter pacing, station approaches, streamed entities and frame cost. Bad authored route/ship IDs fail lint/load; blocked runtime route refuses visibly rather than teleports.
- Owner/entry/state/order: `nova_world_base::sector_ships` plan -> world spawn/thaw (`crates/nova_world_base/src/lib.rs:362`) -> AI/autopilot (`crates/nova_ship/src/flight/state.rs:320-380`) -> map/radar. Audit builders, content, saves, allegiance, UI, wiki/changelog, coordinate with obstacle paths.
## Verification / Done when
- Seeded `nova-probe` observes movement, interaction and retire/thaw identity; matched repeat resource/frame counts vs shipped window; `nova-bench` player flow and inspected rendered frames prove visibility. Permanent proof requires prior approval.
