# Faction colour palettes for ships

- STATUS: OPEN
- PRIORITY: 28
- TAGS: v0.19.0,ships,art

Planned 2026-11-20 from the owner sign-off of the start hull looks
(`tasks/20261116-132144/TASK.md`).

## User facts
- The owner likes that the six start hulls have different colours.
- Owner idea: each faction has a colour palette that its ships use, so
  ships are coloured per faction.
- The options below stay open. The owner decides when the task is picked
  up.

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

## Options (agent ideas, not approved)
- A. Full hull colour per faction. Split colour from style: the style
  keeps the greeble kit; a new authored palette kind holds the plating
  finishes. Each civilization draws one palette from its id (like
  `lawful`; nothing new in the save). A hull with a civilization takes
  that palette; a hull with none keeps its style's palette. The role
  shifts the tone of the faction hue. Stations take the same palette.
  Main cost: greeble pieces with baked hull colour must take the palette
  material.
- B. Role colour stays on the plating; each faction adds one trim colour
  (a third finish on stripes, markings and some edge faces). Less work,
  no baked-colour problem; the faction reads weaker at a distance.

## Open decisions (owner)
- A or B, or another form.
- How many palettes, and what happens when a world has more
  civilizations than palettes (about six to eight hues read apart on
  small hulls).
- Red: the HUD uses red for hostile and the raider is red. Whether a
  faction may have red plating.
- Player hulls: the six start hulls keep their colours or not; a bought
  or captured hull keeps the seller's colour or takes the player's.
- Stations, wrecks and extinct civilizations: same palette, faded, or
  none.
- Whether mods author palettes, and whether the style palette stays as
  the default for hulls with no civilization.

## Delivery (sketch; fixed after the decisions)
1. Proof frames first: one hull of each role in four faction palettes,
   in the `screenshot_base_fleet -- --looks` rig, for the owner.
2. The chosen form in content, lint, load and the skin.
3. A hand-run example, wiki, creator pages, changelog.

## Verification
- Rendered frames of the same hulls in several faction palettes,
  inspected.
- A seeded world: two civilizations in reach show different colours; the
  same seed gives the same colours after a Load.

## Done when
- Ships of different factions read apart by colour in rendered frames,
  and the owner signs off the frames.

## Owner check (2026-11-22)
- Owner moved this task into v0.19.0. The open decisions above still stand.

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
