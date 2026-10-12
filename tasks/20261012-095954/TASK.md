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
- Owner rescope 2026-10-12: `OnSectorEntered` is DROPPED (a sector is a streaming cell, not an author verb; areas with `OnEnter`/`OnExit` cover "went somewhere"). The six events are `OnMined`, `OnPickedUp`, `OnJettisoned`, `OnTraded`, `OnCreditsChanged`, `OnRepaired`. All six are valid on ANY world (mining, intake, trade and repair are ship/gameplay systems that run in a Void chapter today), so there is NO void lint rule. Explicit payload fields; each fires from its owning system, never from a poll. State the plugin ordering for each emitter.

## Verification
- One asserted example per event; a probe harness that counts them over a short flown walk.

## Done when
- The six events are in the closed vocabulary, documented in `docs/scenario-system.md`, and a harness objective can complete on each.
