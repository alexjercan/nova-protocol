# Stations: dynamic thrusterless body that self-levels; orbit proof

- STATUS: OPEN
- PRIORITY: 87
- TAGS: v0.16.0,stations

## User facts
- Epic `20261012-095958`. Decision 3 and 3a (2026-10-12): a station is a dynamic body affected by gravity and hits, never `RigidBody::Static`; its controller sections damp spin so it self-levels; a cluster with a planet gets its station in orbit (circular orbital velocity at materialize), else it stands beside a rock with clearance; no station-keeping thrust; the frozen pose is whatever it was when the player left. Needs a test.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Agent findings
- Gravity wells live in `crates/nova_gameplay/src/gravity`; controller sections and the AI stance in `crates/nova_ship/src/input/ai/`. Planetoids are 500-1600 m (`crates/nova_world_base/src/clusters.rs:116-126`); 500 m clearance (`crates/nova_world_base/src/layered.rs:31`).

## Delivery gate
- Reuse the ship controller for attitude only; show which input the station controller writes and why thrust stays zero. Orbit: compute circular velocity from the planet mass at materialize; state the tolerance.

## Verification
- Asserted examples: a station under a gravity well and a torpedo hit stays level within a stated angle after N seconds; an orbiting station survives three orbits and a torpedo hit without tumbling or decaying into the well. Run them under Xvfb, not only `cargo check`.

## Done when
- Both proofs pass and the self-levelling angle and orbit tolerance are named constants with a reason comment.
