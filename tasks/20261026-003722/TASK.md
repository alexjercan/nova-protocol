# Vary station jobs and stock with living-world events

- STATUS: OPEN
- PRIORITY: 75
- TAGS: v0.17.0

## User facts
- v0.17.0 living events, varied jobs and randomized restock; planning only. Do not import MMO economics.
## Agent findings
- Three slots (`crates/nova_gameplay/src/jobs.rs:89`); held jobs pin slots, expired offers renew, paid refresh exists (`crates/nova_world_base/src/jobs.rs:158-355`). Existing offers reuse local wrecks/hostiles, accepted escort/distress spawn bodies (`:386-683`). Existing clocks: ordinary 1200 s, escort 3600 s, market 1800 s (`crates/nova_authoring/src/base_content/market.rs:35-83`); stock catch-up (`crates/nova_world_base/src/economy.rs:27`). Mere renewal is not new activity.
## Research status
- Design research: [REPORT.html](REPORT.html). Joint slice with [purposeful traffic](../20261026-003723/TASK.md): define and prove a committed station-visit/transfer contract first, then physical autonomous arrival with a visible conserved stock consequence, then connect eligible job changes. Do not ship an unused event or a magic arrival. No gameplay implementation or new tests approved.
- Source review: `restock_thawed_traders` changes stock only on thaw (`crates/nova_world_base/src/economy.rs:1-79`); generic AI docking is not established by parked-hull berthing or ship Move orders (`crates/nova_scenario/src/objects/parked_hulls.rs:646-703`; `crates/nova_ship/src/flight/order.rs:67-139`).
## Decisions needed
- Approve actual AI docking versus explicit standoff exchange; eligible job renewal precedence; stock source and off-window policy; event identity/owner, refusal/save rules, tests, bounded rate and resource budget. No new interface, timer, algorithm, rate or default approved. EVE analogy in umbrella `tasks/20261025-214714/TASK.md` is not Nova evidence.
## Delivery
- End: a world event visibly changes an offer or stock and supports action; held jobs remain valid. Dies: only superseded fixed clocks/rules and stale UI after approval. May break: posted IDs, paid refresh, deterministic generation, saves, pricing and trader inventory. Invalid authored event/ID fails lint/load loudly.
- Owner/entry/state/order: `nova_world_base::post_station_boards` + `restock_thawed_traders` on game clock -> `nova_gameplay::Jobs/JobPostings` and price table -> Jobs/Station UI. Preserve held-slot precedence and deterministic station ordering. Audit builders/generated content, event consumers, thaw/save, UI, wiki/changelog. No active-radius change.
## Verification / Done when
- Approved seeded asserted flow: event -> distinct offer/stock -> action -> retire/thaw/load, IDs and conservation intact, invalid authoring rejected. `nova-bench` new-game Jobs/Station flow and inspected frames for visual claim. Approve any new permanent test first.
