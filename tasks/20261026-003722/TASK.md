# Vary station jobs and stock with living-world events

- STATUS: OPEN
- PRIORITY: 75
- TAGS: v0.17.0

## User facts
- v0.17.0 living events, varied jobs and randomized restock; planning only. Do not import MMO economics.
## Agent findings
- Three slots (`crates/nova_gameplay/src/jobs.rs:89`); held jobs pin slots, expired offers renew, paid refresh exists (`crates/nova_world_base/src/jobs.rs:158-355`). Existing offers reuse local wrecks/hostiles, accepted escort/distress spawn bodies (`:386-683`). Existing clocks: ordinary 1200 s, escort 3600 s, market 1800 s (`crates/nova_authoring/src/base_content/market.rs:35-83`); stock catch-up (`crates/nova_world_base/src/economy.rs:27`). Mere renewal is not new activity.
## Implementation decisions
- Owner approved genuine autonomous AI docking at a station, not standoff exchange. A disposable moving-station spike proved a generated Civilian can reach a free port through the real `DockingConnectionRequest` without pose writes; nose-only `MatchVelocity.facing` could not align its side-mounted port. Spike: `/tmp/nova-ai-dock-m0/` (local-only; no rendered capture).
- Owner requires a visiting NPC to **remain docked** across save/load and freeze/thaw. Save/load must preserve its dock, pose, order/visit phase, inventories, receipt, clocks and job state without a visible discontinuity or duplicate transfer; do not release a visitor as a save/retire shortcut.
- Owner approved: a completed visit adds a temporary extra job to the station board at once. It does not remove the three slot postings (`JOBS_PER_BOARD`, `crates/nova_gameplay/src/jobs.rs:89`). A board renewal removes extras the player has not accepted.
- Still undecided: exact extra-job cap, extra-job expiry, which visit source makes which job, conserved stock source/sink, saved receipt ownership and schema, berth contention, route density and resource budget. Worker-proposed defaults are not owner approvals.
## Committed infrastructure (b72b2f78c)
- AI dock order: `ShipOrderDirective::Dock { station }` (`crates/nova_ship/src/flight/order.rs:114-129`) engages `AutopilotAction::DockApproach` (`crates/nova_ship/src/flight/state.rs:443-475`). It picks a free port pair whose corridor one hull shape cast found clear (`crates/nova_ship/src/flight/dock_corridor.rs`) and captures through `DockingConnectionRequest`. The order is callable, but no production system installs it: no traffic writer exists. The order is never saved; its installer must derive it again after a thaw or Load.
- Visiting NPC dock persistence: a plain AI hull with a stable `EntityId`, docked at a station with an id, freezes inside the station record as `FrozenDockedShip` (`crates/nova_world/src/frozen.rs:526-545,776-808,838-908`) and thaws through `restore_docking_connection` (`crates/nova_world/src/frozen.rs:1069-1106`; `crates/nova_ship/src/sections/docking_section/connection.rs:513`). Every other dock still refuses the save as `UnsavedDock` (`crates/nova_world/src/frozen.rs:916-940,1431-1442`). Load refuses an unknown port and two ships on one station port (`check_saved_ids`, `crates/nova_world_base/src/save/mod.rs:1169-1201` at b72b2f78c). Proof: `crates/nova_ship/src/flight/tests/dock.rs`, `crates/nova_world/src/tests/frozen.rs`, `crates/nova_world_base/src/save/tests.rs`.
- Hire-escort lane and frozen motion: done. Hire escorts park on the station +Y (`crates/nova_scenario/src/objects/parked_hulls.rs:640-679`) and keep their saved relative motion across Load and sector streaming; the save layout gained motion (breaking). Proof: `crates/nova_scenario/src/tests/station_hires.rs`.
- Not saved yet: visit phase, receipt and visit clocks. No visit runtime exists.
## Pending, uncommitted
- `StationVisits` ledger (working tree only, not committed): `crates/nova_gameplay/src/station_visits.rs` keeps each ship's next visit ordinal and one pending visit (`bind`, `validate`). The world save carries it as `WorldSaveState::station_visits`, Load checks it with `check_saved_visits`, and `clear_chart` clears it. No system calls `bind`; it is dormant.

## Research status
- Design research: [REPORT.html](REPORT.html). Joint slice with [purposeful traffic](../20261026-003723/TASK.md): define and prove a committed station-visit/transfer contract first, then physical autonomous arrival with a visible conserved stock consequence, then connect eligible job changes. Do not ship an unused event or a magic arrival. Visit gameplay (traffic writer, ledger writer, extra jobs, stock change) is not implemented.
- Source review: `restock_thawed_traders` changes stock only on thaw (`crates/nova_world_base/src/economy.rs:1-79`).
## Decisions needed
- Exact extra-job cap and expiry; job source per visit; stock source and off-window policy; receipt owner and schema; event identity/owner, refusal/save rules, tests, berth contention, bounded route density and resource budget. No other new interface, timer, algorithm, rate or default approved. EVE analogy in umbrella `tasks/20261025-214714/TASK.md` is not Nova evidence.
## Delivery
- End: a world event visibly changes an offer or stock and supports action; held jobs remain valid. Dies: only superseded fixed clocks/rules and stale UI after approval. May break: posted IDs, paid refresh, deterministic generation, saves, pricing and trader inventory. Invalid authored event/ID fails lint/load loudly.
- Owner/entry/state/order: `nova_world_base::post_station_boards` + `restock_thawed_traders` on game clock -> `nova_gameplay::Jobs/JobPostings` and price table -> Jobs/Station UI. Preserve held-slot precedence and deterministic station ordering. Audit builders/generated content, event consumers, thaw/save, UI, wiki/changelog. No active-radius change.
## Verification / Done when
- Approved seeded asserted flow: event -> distinct offer/stock -> action -> retire/thaw/load, IDs and conservation intact, invalid authoring rejected. `nova-bench` new-game Jobs/Station flow and inspected frames for visual claim. Approve any new permanent test first.
