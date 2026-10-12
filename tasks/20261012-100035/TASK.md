# Stance: research note

- STATUS: OPEN
- PRIORITY: 48
- TAGS: v0.16.0,stance

## User facts
- Epic `20261012-100034`. Write the note decision 12 asks for: cluster radius and neighbour-only seeding, the event and threshold table, the single-hop propagation rule, the redemption path, and a seed sweep proving every neighbourhood holds a rival and no flip ever reaches a non-neighbour. No code.

## Agent findings
- Civilizations carry centroids 240 km apart (`crates/nova_world_base/src/civilizations.rs:25`); allegiance is `Player / Enemy / Neutral` with no Ally variant (`crates/nova_gameplay/src/relations.rs:37-44`).

## Done when
- `RESEARCH.md` in this folder with the tables and the sweep design, and an owner acceptance recorded in the task.
