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
