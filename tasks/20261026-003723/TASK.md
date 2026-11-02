# Make generated ship traffic visibly purposeful

- STATUS: OPEN
- PRIORITY: 70
- TAGS: v0.17.0

## User facts
- Meaningful moving ambient traffic in v0.17.0; specification only, no active-radius change.
## Agent findings
- Intact generated ships already have AI; seeded patrol has four 1500 m waypoints, 30-60 s holds, empty if fewer than two fit (`crates/nova_world_base/src/sector_ships.rs:65-87,149-153,358-393`). Job escort/distress already spawn bodies (`crates/nova_world_base/src/jobs.rs:460-683`). Shipped active radius 2 is 5x5x5 (`crates/nova_world_base/src/lib.rs:190,362`). More bodies alone is not new activity.
## Research status
- Joint design research: [station visit and traffic report](../20261026-003722/REPORT.html), with [station jobs and stock](../20261026-003722/TASK.md). Develop a committed visit/transfer contract and named proof first; ship it only with a physically approaching autonomous ship, a conserved visible stock consequence and later eligible job changes. Traffic alone is not a completed station visit.
- Source review: ship helm Move has no dock directive (`crates/nova_ship/src/flight/order.rs:67-139`); parked-hull berthing requests a connection but does not prove autonomous approach (`crates/nova_scenario/src/objects/parked_hulls.rs:646-703`). Actual AI docking is a new candidate capability, while a standoff exchange is lower cost but may not meet the intended visit.
## Decisions
- Owner 2026-11-02: genuine AI docking, profiles drawn per generated ship, one visitor per station, defaults authorized; see [station jobs and stock](../20261026-003722/TASK.md) for the full rule set.
## Implemented 2026-11-02 (9fb0a8760, example 826d0187e)
- Generated ships with a trading or posting profile visit the nearest living station within 20 km on a seeded 60-300 s first wait and 300-600 s between visits; they dock, trade, post jobs, dwell 60 s and undock. Patrol stays the passive routine between visits.
- Live proof on seed 13: a generated ore hauler flies 1.5 km, docks, trades and leaves; frames inspected. See the proof section of 20261026-003722.
## Open
- Density: in 8 of 9 scanned seeds no trading/posting profiled ship starts within 20 km of the one offering station the window holds; most sit 50-120 km away. Purposeful traffic is rare in normal play.
- Hand-off: a visitor that binds as its predecessor undocks fails NoSafeDockPath on the departing hull and waits 300-600 s.
- No matched-reference frame or resource measurement yet; the bound is one pending visit per live station.
## Delivery
- End: at least one recognizable non-job movement with a player-visible consequence. Dies: superseded patrol rule only if replaced. May break: AI orders, identities, encounter pacing, station approaches, streamed entities and frame cost. Bad authored route/ship IDs fail lint/load; blocked runtime route refuses visibly rather than teleports.
- Owner/entry/state/order: `nova_world_base::sector_ships` plan -> world spawn/thaw (`crates/nova_world_base/src/lib.rs:362`) -> AI/autopilot (`crates/nova_ship/src/flight/state.rs:320-380`) -> map/radar. Audit builders, content, saves, allegiance, UI, wiki/changelog, coordinate with obstacle paths.
## Verification / Done when
- Seeded `nova-probe` observes movement, interaction and retire/thaw identity; matched repeat resource/frame counts vs shipped window; `nova-bench` player flow and inspected rendered frames prove visibility. Permanent proof requires prior approval.
