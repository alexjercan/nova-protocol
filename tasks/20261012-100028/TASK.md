# Economy: play-time clock in the world save

- STATUS: OPEN
- PRIORITY: 57
- TAGS: v0.16.0,economy

## User facts
- Epic `20261012-100025`. Decision 8 (2026-10-12): add a play-time clock to the save when the first consumer (restock) lands, with a catch-up rule per frozen body type; nothing earlier needs it.

## Agent findings
- No clock exists; the only time is scenario `Elapsed` (grep `WorldClock|world_time|WorldTime` in `crates/nova_world*` and `nova_scenario`: no hits). Frozen sectors do not simulate (`crates/nova_world/src/lib.rs:116-117`).

## Delivery gate
- `WorldClock { play_seconds }` in the save state, advanced only while unpaused and armed; `FrozenBody` records `frozen_at`; a thaw hook receives the elapsed play time. Format bump, **(breaking)**.

## Verification
- Unit test: pause does not advance the clock; save and load keep it; a thaw hook sees the right elapsed time.

## Done when
- The clock is in the save and one consumer (restock) reads it.
