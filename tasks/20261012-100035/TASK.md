# Stance: research note

- STATUS: CLOSED
- PRIORITY: 48
- TAGS: v0.16.0, stance

## User facts
- Epic `20261012-100034`. Write the note decision 12 asks for: cluster radius and neighbour-only seeding, the event and threshold table, the single-hop propagation rule, the redemption path, and a seed sweep proving every neighbourhood holds a rival and no flip ever reaches a non-neighbour. No code.

## Agent findings
- Civilizations carry centroids 240 km apart (`crates/nova_world_base/src/civilizations.rs:33`); allegiance is `Player / Enemy / Neutral` with no Ally variant (`crates/nova_gameplay/src/relations.rs:37-44`).
- 2026-10-17: `RESEARCH.md` written against `9b507e6e5`. It separates measured facts, proposed rules and test design; its section 11 lists the decisions the owner must take (neighbour radius, seed of the player table, shares, thresholds, propagation direction, group retaliation, fine, decay, names). The seed sweep ran once through a disposable census spike that was deleted; the permanent proof is test design only. Owner feedback 2026-10-16 (group retaliation) and the 2026-10-17 milestone direction are folded in. The role-dependent fleet cap is recorded as an idea, not a rule.

## Status
- 2026-10-17: the owner accepted section 11's recommendations in full ("go ahead with stance graph") and directed task 100036 to implement them; recorded here as the acceptance the Done-when asks for. No code, test or type was added under this task.

## Done when
- `RESEARCH.md` in this folder with the tables and the sweep design, and an owner acceptance recorded in the task.
