# Nicer ship looks, starting with the six start hulls

- STATUS: OPEN
- PRIORITY: 65
- TAGS: v0.19.0, ships, art

Planned 2026-11-16 from the owner review of the game-feel research doc
(claude.ai artifact Hq6GmMBXm2zAfVFwGnQg45).

## User facts
- "ships look a bit meh and could be done nicer".
- The start ships need work on how they look.

## Agent findings
- Start hulls are hand-placed cube cells in
  `crates/nova_authoring/src/base_content/ships/block.rs:136-491` (ids at
  `ships/mod.rs:34-47`). The mining tug is a 3x1x7 block (`block.rs:139`).
- Hulls wear a derived skin from `styles.rs`, with greebles from
  `scripts/greeble-recipes/` through `scripts/gen-greebles.py`
  (`assets/base/base.bundle.ron:97-98`).
- Start hulls may use advancement-0 parts only (`block.rs:9-11`).
- Owner rule on record: content and config languages are solver rules, not
  fiction vocabulary; greeble placement candidates A/B/C were rejected
  2026-10-14.

## Delivery
1. Rendered baseline: one frame per start hull, same rig and light.
2. Redesign silhouettes and section layout per hull; improve skin and
   greeble use. Keep role balance from task 20261116-132143.
3. Rendered after frames for owner review.

## Verification
- Before/after frames of all six start hulls, inspected, not only built.

## Done when
- The owner signs off the six start hulls from rendered frames.
