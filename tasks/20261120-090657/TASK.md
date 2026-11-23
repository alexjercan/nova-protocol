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
  per lattice node, so no two neighbours match. Reds are allowed (owner,
  2026-11-23: "I would even add some reds"); lint keeps only the magenta
  band. This replaces the earlier red-band rule.
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
- The tint replaces the magenta key colour outright (owner, 2026-11-23).

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
  Owner picked replace.
- Old saves are not a cost: no compatibility for them; delete old saves.
- New shapes (thin parts, wedges, wings): owner agrees, not now; future work.

## Build (2026-11-23)
Options 1 and 2 built; uncommitted on `master`. Frames in
`~/Pictures/nova-faction-palettes/` (looks rig, start hull looks, world).

- Content kind `Palette` (`FactionPaletteConfig`, typed
  `FactionPaletteDesignId`): a finish per role and for stations,
  `greeble_density` (0.5 to 2.0) and `accents`. Owner:
  `crates/nova_ship/src/sections/faction_palette.rs`. Base catalog: 8
  palettes in `crates/nova_authoring/src/base_content/palettes.rs`, one hue
  each, role tones shared (civilian L 0.55, industrial 0.40, scavenger 0.30,
  armored 0.24, station 0.45; wall a third of the top).
- Pick: `ShipPartSnapshot::palette(civ)` = `order[(x + 2y + 4z) mod N]`,
  `order` a Fisher-Yates shuffle seeded by the world seed. The snapshot pins
  the sorted palette ids and refuses fewer than 8 (`TooFewPalettes`).
- `ShipPresentationConfig.palette: Option<ShipPaletteConfig { id, role }>`.
  `generate_ship` and `generate_wreck` write the civilization's palette with
  the role finish; stations write it with `Station`. `None`: start hulls,
  scenario ships, editor.
- `ShipStyle { style, palette }` replaces the tuple; `GameStyles { styles,
  palettes }` replaces the tuple. `SkinLook` resolves both;
  `SkinLook::worn_style()` applies the finish and the habit (empty weights
  divided by the density, accents added as lone greebles). Plate materials
  are keyed by `ShipStyle`, so (style, palette, role).
- Greebles: the 10 civilian recipes use the key `[1, 0, 1]` and were
  regenerated (`gen-greebles.py --check` clean). A fixture carries
  `tint: Hull | Baked`. `tint_decor_keys` replaces every key material with
  `DecorTint` (the look's top finish) when the model is ready; no tint leaves
  magenta. Lint reads the `.glb` and refuses a key with `Baked` or `Hull`
  with no key.
- Shed fixtures and detached pieces freeze the full `ShipStyle`; a Load
  refuses an unknown style or palette by id.
- Lint: palette faults (key magenta band 20 deg; the hostile red band was
  removed in "Build: more palettes",
  density range, unknown/duplicate/unweighted accent, habit that does not
  compile), design unknown palette or palette without style, fewer than 8.

Proof: `nova_world_base` `neighbour_civilizations_never_share_a_palette`
(also same id, same palette), `a_catalog_with_too_few_palettes_is_refused`,
`a_generated_ship_its_wreck_and_its_station_wear_their_civilizations_palette`
(replaces `every_role_wears_its_style_intact_and_wrecked`),
`a_load_whose_piece_wears_an_unknown_style_or_palette_is_refused` (save and
load keeps the palette id: the refusal names the id read back from disk).
`nova_ship` `plate_materials_are_shared_within_a_palette_and_differ_between_palettes`,
`a_keyed_greeble_takes_its_tint_and_an_untinted_one_stays_magenta`, and the
`faction_palette` tests (magenta band, unknown accent and density, finish
and habit; the red band test was removed with its rule). `nova_scenario`
`a_design_naming_an_unknown_palette_is_refused`. `nova_authoring` lint_walk
`a_keyed_greeble_with_no_tint_source_is_refused`.

Frames:
- `faction_palettes` example, ochre, teal, cobalt, indigo: every role plus
  the armored wreck. Smoke PASS each: every hull wears the palette, plates
  carry its role top, civilian keyed greebles tinted (164 to 213 meshes), no
  magenta (also 0 magenta pixels). Greebles 1559 (teal, 0.7) to 2293
  (indigo, 1.5) on the same five hulls.
- `screenshot_base_fleet --looks`: the patrol picket's pale stripe fillet is
  gone; the band face and pinstripe stay baked.
- `world_features`: two ships of sector (-3, -3, -2), ochre scavenger
  (civ (-1, 0, -1)) and steel industrial wreck (civ (0, -1, 0)), each
  checked live for its palette. They stand 2.3 km apart, so they are two
  frames, not one.

Draw cost: palettes add no draw calls per hull; plate materials grow from
one set per style to one per (style, palette, role) in view. The row of five
hulls draws 36518 to 38080 meshes; the count follows the greeble density.

Open for the owner: tone tuning (civilian and industrial read close; cobalt
and indigo are near hues).

## Build: more palettes (2026-11-23)
Owner: "the styles look good, but I would even add some reds and maybe some
more colors". Uncommitted on `master`. Frames in
`~/Pictures/nova-faction-palettes/more/` (one per palette and
`contact-sheet-all-palettes.png`).

- The red band is gone: `HOSTILE_BAND_DEGREES`, `FactionPaletteFault::HostileHue`
  and its test. The magenta key band (20 deg round hue 300) stays.
- 15 palettes. The first 8 are unchanged. New: maroon (2, dark), rust (18),
  crimson (352), bone (45, pale, low saturation), emerald (140, light), sky
  (195, light), plum (267, dark). The builder takes a lightness scale per
  palette (1.0 for the first 8); the role order stays.
- A pale violet (265) was rendered and dropped: it read as indigo.
- Closest new pairs in the frames (OKLab distance of rendered hull colour,
  x100): plum-indigo 4.8, rust-ochre 4.6, teal-sky 4.4, bone-olive 4.7,
  moss-emerald 5.3. The first build's close pairs: steel-slate 2.1,
  cobalt-indigo 2.3, ochre-olive 3.2. The shuffle can make any two palettes
  neighbours.
- `faction_palettes` takes `NOVA_PALETTE_WALK=<id,id,...>`; all 15 PASS, 0
  magenta pixels.
- The mod editor lists, makes and opens palettes (`KINDS` row `made`).
