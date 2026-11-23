# Faction build doctrine: per-civilization hull proportions

- STATUS: OPEN
- PRIORITY: 27
- TAGS: v0.19.0,ships,generation

Split 2026-11-23 from `tasks/20261120-090657/` (faction palettes and
greeble habit), which holds the design page `DESIGN-faction-looks.html`
and its spike frames. This task is option 3 on that page.

## User facts
- Owner: the second task is option 3, "each faction with different
  generator" for the structure of the ships. It comes after options 1
  and 2.
- New shapes (thin parts, wedges, wings) are not in this task; future
  work.
- Old saves are not a cost; no compatibility for them.

## Decisions (page recommendation, owner approved the split)
- A `BuildDoctrine` per civilization, drawn from its own seeded stream
  (`id.stream(b"doctrine")`, like `lawful` and the station archetype).
- It biases only the safe knobs the generator already draws: scale
  centre, broad and compact share, cube pattern weights, drive pair
  share, waist and taper amplitude. Read in `plan`.
- Size and `min_aspect` stay per role: they move berth fit, clearance
  and turn rate.

## Agent findings
- The generator draws scale, long/broad/compact, waist and taper, cube
  pattern and drive layout from the hull seed (see the page, Build
  style).
- A failed layout fails the whole sector after 8 attempts
  (`ship_layout.rs:93`, `sector_ships.rs:33-35`).
- A longer or wider hull turns slower; turn rate is not gated.
- Design page: `DESIGN-build-doctrine.html` (frames in `frames/`). Code
  at 1e3e2243c. Spike code reverted.
- `ShipLayoutRequest` already names its civilization, so
  `BuildDoctrine::of(request.civilization)` needs no caller change.
  Knobs read in `plan` (`ship_layout.rs:1916-1934`), `stations`
  (`:2014-2015`), `nose_length` (`:1253`), pattern (`:1274`) and drive
  pair (`:1571`).
- Knob extremes, one at a time (32 settings x 4,000 hulls + 1,000
  wrecks): 0 layout fails, 0 wreck fails. Redraw rate 9.6% to 17.1%
  (today 11.6%; shoulder +1 highest).
- Turn-rate medians, today, deg/s at advancement 0 / 0.5 / 1: civilian
  70/42/34, industrial 72/41/32, scavenger 78/50/42, armored 69/37/29.
  Scale 0.7 to 1.3 moves them +6 to -7. Broad or compact 1.0 cuts
  advancement-0 hulls by up to 13 (scavenger 78 to 65) and adds up to 7
  at 1. Nose, waist, taper and pair move them 2 or less. Cube pattern
  does not change layout at all.
- Page doctrine (scale centre 0.85..1.0, waist +-0.4, other knobs as on
  the page), 200 civilizations x 4 roles x 5 advancements x 32 seeds:
  128,000 hulls and 32,000 wrecks, 0 fails; turn medians within 2 deg/s
  of today. A wider draw (scale to 1.15, waist +-0.75) failed 1.
- Section budget is not checked after fitting. Today 3 of 128,000 hulls
  pass 2000 sections (largest 2063); page doctrine 17; wider draw 23.
  `generated_wrecks.rs:136-150` checks one civilization only.
- Escort raiders (`Budget` sizes, scavenger): today 0 fails in 14,400.
  Page doctrine 23 fails, all in band 2 (108 sections, 3 turrets):
  "no eligible cargo intake part fits the layout". No single knob is
  the cause. Shoulder +1 fails every band-0 raider. A job raider failure
  panics (`nova_world_base/src/jobs.rs:960`).
- Clearance: today largest 339 m; scale 1.3 reaches 386 m (ceiling
  400 m). Berth fit not measured.
- Neighbours: 10 of 424 neighbour pairs (2.4%) draw alike doctrines.
- Frames: long, stout and stubby doctrines read apart at one fixed
  camera. Waist -1 makes thin dumbbell necks (sever risk not measured).

## Open decisions (page recommendations, not approved)
- Seeded doctrine, authored catalog, or palette field. Rec: seeded.
- Request field or derived from the civilization. Rec: derived.
- Section budget: rec check `MAX_SHIP_SECTIONS` in `check` and redraw,
  with scale centre 0.85..1.0 until berth fit is measured.
- Raiders: rec `Budget` sizes read only the cube pattern, so raider
  layouts stay as today.
- Nose divisor 3..8 as a new knob. Rec: add.
- Width and height bias. Rec: leave out.
- Per-hull spread narrow (scale +-0.1, waist +-0.5, taper +-0.25).
  Rec: narrow.
- Proof: new playable example `faction_doctrines` with a civilization
  stepper. Rec: yes.

## Verification
- Sweep: 200 civilizations x 4 roles x advancement steps x 32 seeds,
  zero layout failures.
- Turn-rate table per doctrine extreme; escort and raider difficulty
  checked against it.
- Frames: two civilizations' hulls of each role side by side read apart
  by shape. Owner signs off.

## Done when
- Civilizations build visibly different hulls inside each role's
  ranges, the sweep shows zero failures, and the owner signs off the
  frames.
