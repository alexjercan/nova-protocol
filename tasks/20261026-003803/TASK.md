# Route GOTO around observable obstacles without widening the world

- STATUS: OPEN
- PRIORITY: 65
- TAGS: v0.17.0

## User facts
- Obstacle-aware autopilot paths, with manual override and no active-radius change. Specification only.
## Agent findings
- `AutopilotAction::Goto` tracks target position each tick; its doc says no collision avoidance (`crates/nova_ship/src/flight/state.rs:320-344`). `autopilot_system` and `autopilot_step` own flight (`crates/nova_ship/src/flight/autopilot.rs:528,819`); predicted path is published after physics and when the paused map is shown (`crates/nova_ship/src/flight/mod.rs:227-238`; `crates/nova_ship/src/flight/prediction.rs:465`). Map GOTO engages real autopilot (`crates/nova_interface/src/map/tests.rs:761`). Shipped window already 5x5x5 (`crates/nova_world_base/src/lib.rs:190`). ORCA reference in umbrella `tasks/20261025-214714/TASK.md` is not a Nova algorithm prescription.
## Decisions needed
- Which obstacles and clearance, visible vs unknown information, target motion, route ownership/ID/state and recalculation, stuck/refusal policy, preview fidelity, AI vs player applicability, deterministic budget and freeze/thaw. No algorithm or radius approved.
## Delivery
- End: GOTO honors chosen obstacles, route is visible and cancellable, impossible route refuses clearly; player can still fly manually. Dies: superseded direct-leg assumption and stale UI/wiki copy only after approval. May break: path prediction, arrival, AI patrols, gravity, navigation IDs, save and frame cost. Invalid authored obstacle geometry/route ID fails lint/load; runtime unrouteable leg does not silently fly through.
- Owner/entry/state/order: flight `AutopilotAction` -> autopilot_step -> predicted path -> map route UI; audit world geometry/streaming, input/cancel, AI orders, content, save and docs/changelog. No MMO/local avoidance algorithm implied.
## Verification / Done when
- Seeded `nova-probe`: blocked direct leg, safe detour and unreachable refusal, then cancel; inspect real trajectory/collisions and map frames. Matched repeat resource cost against direct-leg baseline; permanent tests only on approval.
