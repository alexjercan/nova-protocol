# Stations: a severed station stops taking damage

- STATUS: OPEN
- PRIORITY: 98
- TAGS: v0.16.0,stations,bug

## User facts
- Owner report 2026-10-13: "when shooting a station after it got severed it stopped taking damage".
- Stations take damage since the station rework (ad4e89637): damage stops at the design's `collapse_threshold` share of the hull pool (0.5 for every station), so a station never breaks up.

## Agent findings
- Unverified hypothesis: the sever removes sections from the pool, the pool reaches the 0.5 floor, and every later hit is clamped to zero. If so the floor works as written but reads as a bug in play. Other candidates: the kept half loses `StationMarker`/AI or the hit routing after a sever (`crates/nova_ship/src/sections/integrity.rs` sever path, `crates/nova_gameplay/src/integrity/health.rs` `on_damage`).

## Delivery gate
- Reproduce first (asserted example or unit test that severs a station and then hits it), keep the before artifact, name the cause, then fix. If the cause is the floor, stop for the owner on the floor rule before changing it.

## Verification
- After a sever, hits on standing sections reduce their health until the defined end state (dead station) is reached.

## Done when
- The reproduced failure has a permanent proof and passes.
