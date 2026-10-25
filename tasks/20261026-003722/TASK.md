# Vary station jobs and stock with living-world events

- STATUS: OPEN
- PRIORITY: 75
- TAGS: v0.17.0

## User facts
- v0.17.0 living events, varied jobs and randomized restock; planning only. Do not import MMO economics.
## Agent findings
- Three slots (`crates/nova_gameplay/src/jobs.rs:89`); held jobs pin slots, expired offers renew, paid refresh exists (`crates/nova_world_base/src/jobs.rs:158-355`). Existing offers reuse local wrecks/hostiles, accepted escort/distress spawn bodies (`:386-683`). Existing clocks: ordinary 1200 s, escort 3600 s, market 1800 s (`crates/nova_authoring/src/base_content/market.rs:35-83`); stock catch-up (`crates/nova_world_base/src/economy.rs:27`). Mere renewal is not new activity.
## Decisions needed
- Observable event and consequences, eligible job variety, reward and stock accounting, board size, seeded bounded random schedule, stable IDs, session/save and missing-ID policy. No new timer, algorithm or default approved. EVE analogy in umbrella `tasks/20261025-214714/TASK.md` is not Nova evidence.
## Delivery
- End: a world event visibly changes an offer or stock and supports action; held jobs remain valid. Dies: only superseded fixed clocks/rules and stale UI after approval. May break: posted IDs, paid refresh, deterministic generation, saves, pricing and trader inventory. Invalid authored event/ID fails lint/load loudly.
- Owner/entry/state/order: `nova_world_base::post_station_boards` + `restock_thawed_traders` on game clock -> `nova_gameplay::Jobs/JobPostings` and price table -> Jobs/Station UI. Preserve held-slot precedence and deterministic station ordering. Audit builders/generated content, event consumers, thaw/save, UI, wiki/changelog. No active-radius change.
## Verification / Done when
- Approved seeded asserted flow: event -> distinct offer/stock -> action -> retire/thaw/load, IDs and conservation intact, invalid authoring rejected. `nova-bench` new-game Jobs/Station flow and inspected frames for visual claim. Approve any new permanent test first.
