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

## Decisions (owner said continue 2026-11-19; agent calls, stated; map `MAP.md`, frames `before/`)
- Cause, from the baseline: the build parts are cubes only, so a start
  hull of 5 to 10 cells reads as a lump. The hulls are short and bluff
  (interceptor 5 x 5 in plan), four of six share a style, and the drives
  sit in one row. `line_warship` and the generated hulls look better
  because they are long, mirrored and taper from a one-cell nose.
- Rules for all six:
  - Same section counts by kind as now, so mass, HP, hold, worth and
    acceleration do not move on paper. At most two light hull cells more or
    fewer where a shape needs it, reported. The census is rerun at the end.
  - Length at least 2.5 times the body width. A stepped nose down to one
    cell. No bluff two-deck bow wall.
  - Mirrored about the centreline, except one stated offset on the raider.
  - Drives in mirrored groups, not one row; no cell hides a bell from the
    stern quarter.
  - Runs of three or more cells along the length, so the skin lays long
    plates.
  - One signature shape per hull, readable at thumbnail size.
  - One style per start hull with its own hue (now: two pairs share).
- Signature per hull:

  | Hull | Shape | Hue |
  |---|---|---|
  | mining_tug | pusher: thin beam boom ahead of a wide low ore body, cab aft, drives in a triangle | ochre |
  | salvage_hauler | forked bow of two prongs, flat work deck, tall bridge block aft over the drives | rust |
  | light_freighter | narrow cab neck, wide box hold, two drive pods outboard aft | off-white |
  | interceptor | dart: one low deck, narrow nose that widens to the stern, short dorsal block | dark blue-grey |
  | raider | hammerhead: wide head forward with the turrets, narrow tail, one outrigger drive pod | dark red-brown |
  | patrol_picket | keeps its widening plan; low bow, bridge tower aft of midships, short strakes | slate |

- Not in this task: new section shapes (slope, wedge, thin part), new
  section art for the bell, collar, turret and intake, paint, stripes,
  lights. The map lists them as not possible with the current vocabulary.
- The after set uses the same `--looks` rig as `before/`.

## Delivery
1. Rendered baseline: one frame per start hull, same rig and light.
2. Redesign silhouettes and section layout per hull; improve skin and
   greeble use. Keep role balance from task 20261116-132143.
3. Rendered after frames for owner review.

## Verification
- Before/after frames of all six start hulls, inspected, not only built.

## Done when
- The owner signs off the six start hulls from rendered frames.
