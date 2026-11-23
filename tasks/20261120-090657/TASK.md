# Faction looks: palettes and greeble habit

- STATUS: OPEN
- PRIORITY: 28
- TAGS: v0.19.0,ships,art

Planned 2026-11-20 from the owner sign-off of the start hull looks
(`tasks/20261116-132144/TASK.md`).

## User facts
- The owner likes that the six start hulls have different colours.
- Owner idea: each faction has a colour palette that its ships use, so
  ships are coloured per faction.
- Owner, 2026-11-23: split into two tasks; this one is options 1 and 2,
  the second is option 3 ("each faction with different generator").

## Agent findings
- A style holds the plating colour and the greeble kit together
  (`ShipStyleConfig`, `StylePalette { top, wall }`,
  `crates/nova_ship/src/sections/skin_style.rs:85-113`). A hull names one
  style id.
- A generated hull gets its style from its role, not its civilization
  (`role_style_id`, `crates/nova_world_base/src/civilizations.rs:147`).
  Every civilization's scavenger wears the same colour.
- A civilization is seeded, not authored: id, centroid, status,
  advancement, role preference, stance, `lawful`
  (`civilizations.rs:158`). It has no look.
- Some greeble pieces have the hull colour baked into the model. The
  `civilian` palette was left alone in the looks task for this reason.
- Seven styles now: `industrial`, `salvage`, `civilian`, `armoured`,
  `scrap`, `patrol`, `station`.
- Not checked: whether every spawned ship carries its civilization on the
  entity; how many civilizations a world has; how many greeble pieces
  have baked colour.

## Decisions (owner approved options 1 + 2, 2026-11-23)
- Scope: options 1 and 2 of `DESIGN-faction-looks.html`. Option 3 (build
  doctrine) is its own task, `tasks/20261123-093539/`.
- Full hull colour (A): the faction hue on the plating; the role sets the
  tone. The magenta key keeps a trim finish (B) cheap later.
- Palettes are content: 8 or more, picked by `order[(x + 2y + 4z) mod N]`
  per lattice node, so no two neighbours match. Lint refuses a red band.
- Start hulls keep their colours; a bought hull keeps the seller's
  palette (it is in its design). Stations take the palette's station
  finish; wrecks keep their builder's colours, no fade.
- `palette: None` means the style's own palette: start hulls, scenario
  ships, editor. Not a path for old files; old saves are deleted.
- Greebles: recipe slots that follow the hull use a magenta key colour;
  the greeble takes a tint input (palette Top finish) that replaces the
  key. A missed tint renders magenta; lint refuses a keyed piece with no
  tint source and magenta in a palette. Fixes the patrol pale stripe.
- Greeble habit: the palette record (or a sibling record) carries a
  density factor and an accent piece list. Lint: pieces exist, factor in
  range.
- Open: tint replaces the key or multiplies a base colour (agent rec:
  replace; recipes are flat colour).

## Delivery
- Slice 1 (palettes) and slice 2 (greeble habit) as on the design page,
  Delivery sketch. A hand-run example, wiki, creator pages, changelog.

## Verification
- Rendered frames of the same hulls in several faction palettes,
  inspected.
- A seeded world: two civilizations in reach show different colours; the
  same seed gives the same colours after a Load.

## Done when
- Ships of different factions read apart by colour in rendered frames,
  and the owner signs off the frames.

## Owner check (2026-11-22)
- Owner moved this task into v0.19.0.

## Design (2026-11-22)
- Page: `DESIGN-faction-looks.html` (spike frames in `frames/`, raw in
  `~/Pictures/nova-faction-looks/`). Proposal only; the owner picks.
- Colour per faction: yes. Build style per faction: yes, as seeded biases
  on draws the generator already makes, inside each role's ranges.
- Options:
  - 1. Colour only: an authored palette catalog (8+, no red band); each
    civilization takes one by its lattice node, so no two neighbours match;
    the role sets the tone and keeps its greeble kit.
  - 2. Colour + greebles: 1, plus a greeble density and an accent piece set
    per civilization.
  - 3. Colour + proportions: 1, plus seeded biases on scale, broad or
    compact share, waist and taper, cube pattern and drive layout.
  - 4. Full doctrine: 1 + 2 + 3.
- Agent recommendation: 1 + 2 in v0.19.0; 3 as a later slice after a
  layout failure sweep and turn-rate numbers.

## Owner check (2026-11-23)
- Greebles: recipe slots that follow the hull use a magenta key colour;
  the greeble takes a tint input (faction palette) that replaces the key.
  Open: replace or multiply (agent rec: replace).
- Old saves are not a cost: no compatibility for them; delete old saves.
- New shapes (thin parts, wedges, wings): owner agrees, not now; future work.
