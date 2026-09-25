# Prove asteroid-group composition and field readability

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog,world,design

## User facts
- Owner proposes `P* A+ S*`: optional planets, at least one asteroid or asteroid group, optional ships; prefer asteroid groups over isolated ship groups. This is a generation idea, not approved base-world tuning. Empty streamed cells can remain intentional unless explicitly changed.

## Agent findings
- NovaLayeredWorld has a 50 km cluster lattice and rock-only, asteroid-rich, planet-heavy and derelict-only groups (`crates/nova_world_base/src/clusters.rs:59,164-194`). Derelict-only currently plans no rocks (`clusters.rs:680-695`). Planned rocks can be skipped on clearance or faces; `SectorClusters` reports placement and skip counts (`clusters.rs:198-244`). Seeded owner and cross-sector geometry exist, but A+ after placement is unproven.

## Proposed design work, not implementation approval
- Compare A+ per nonempty cluster, per interesting place, or per streamed sector; state the effect of each on quiet space, sparse travel, derelict fields and body budget. Recommend testing per-cluster first without claiming it is settled.
- Specify deterministic placement or fail-loud refusal if the guaranteed parent rock cannot fit, preserving unique ID, cross-cell owner, same-seed/visit-order results, geometry and atomic sector readiness.
- Check whether mining/resource kind belongs on existing asteroid content and whether all groups need value beyond mere rock presence. Coordinate with static-well gravity and moving-rock lifetime research before choosing orbit arrangements.

## Verification to design
- Seed/edge sweep of final *placed* bodies and skipped counts; seam/reverse-order and manifest validity assertions, repeated rendered flight through rock, mixed and empty fields, and matched window fill cost. Do not assert a numeric gameplay density from a mockup.

## Done when
- Owner has reviewed composition scope, guarantee/failure rule, variant pacing and proof, with any production generator change gated by a separate code-backed plan.
