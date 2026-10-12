# World first: open-world events in nova_events

- STATUS: OPEN
- PRIORITY: 94
- TAGS: v0.16.0,world-first

## User facts
- Epic `20261012-095526`. A harness on a generated world needs events for the verbs the open world has: sector entered, ore mined, item picked up, trade completed, credits changed. The located jobs strand reuses them.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Agent findings
- No such events exist (`crates/nova_scenario/src/events.rs:76-160`; grep for `SectorCrossed|OnSectorEnter|SectorEntered` finds nothing). `CurrentSector` changes drive saves (`crates/nova_world_base/src/save/session.rs:165-172`); mining, pickup and trade have owning systems in `crates/nova_scenario/src/mining.rs`, `crates/nova_ship/src/sections/cargo_intake_section.rs` and `crates/nova_gameplay/src/inventory.rs`.

## Delivery gate
- Add `OnSectorEntered`, `OnMined`, `OnPickedUp`, `OnTraded`, `OnCreditsChanged` with explicit payload fields; each fires from its owning system, never from a poll. Lint: a handler for these on a `Void` scenario is an error. State the plugin ordering for each emitter.

## Verification
- One asserted example per event; a probe harness that counts them over a short flown walk.

## Done when
- The five events are in the closed vocabulary, documented in `docs/scenario-system.md`, and a harness objective can complete on each.
