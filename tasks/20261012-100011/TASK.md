# Wizard: advancement curve smoothstep to about 4000 km

- STATUS: OPEN
- PRIORITY: 77
- TAGS: v0.16.0,wizard

## User facts
- Epic `20261012-100008`. Decision 4b (2026-10-12): switch the advancement curve to the existing smoothstep variant with the saturation raised to about 4000 km and the seeded offset scaled by the curve value; tune the far radius together with the start-role kit by how many rings a start hull crosses before the first Armored patrol. The owner's feel: many small ships, then larger.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Agent findings
- `AdvancementCurveType::{Linear, Smoothstep}` and `ADVANCEMENT_SATURATION = 1_000_000 m` with a seeded offset in [-0.1, 0.1) (`crates/nova_world_base/src/civilizations.rs:41-44`, `:103-110`, `:206-214`); hull clearance and spine sizes interpolate linearly with advancement (`crates/nova_world_base/src/ship_layout.rs:89-95`, `:797-799`, `:1054-1068`). The curve carries `TODO(20260923-110307)`.

## Delivery gate
- Two constants and one offset rule; no new tables. Replace the TODO with the decision. Record the ring table (0.01 / 0.04 / 0.09 / 0.15 at rings 1-4) as the expected output in the test.

## Verification
- Unit test over the four rings and 2000/3000/4000 km; a seed sweep asserts no medium hull inside ring 2.

## Done when
- The curve is smoothstep, the constants are named with a reason, and the sweep is a permanent test.
