# NOVA OS ship app: clearer section rendering (separation + per-section type/HP/ammo indicators)

- PRIORITY: 31
- TAGS: v0.9.0, feedback, feature, ui, hud, gameplay
- KIND: TASK
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE

## Story

As a player using the NOVA OS `ship` app, I want each section to be visually
distinct and self-describing in the 3D schematic, so the ship reads as labelled
sections rather than one green blob - I can tell what each section is/does and
its HP/ammo at a glance.

Playtest verdict (2026-07-28): the schematic app from `20260726-115339` "looks
AMAZING" but "it's just a green blob"; this is the owner's follow-up polish
request #2 (the rendering-legibility half).

## What it should do

- Give the proxy blocks real **separation**: edges/outlines/wireframe or gaps so
  adjacent sections don't merge into one mass.
- Encode section **kind** visibly (per-kind hue and/or a small icon/silhouette:
  hull vs thruster vs controller vs PDC turret vs torpedo bay), keeping the
  CRT/phosphor palette.
- Surface per-section **HP (and ammo where present)** in the view itself (e.g. a
  small integrity bar on each blip/label, colour by status), not only in the
  readout.
- Keep it readable while orbiting and at the default framing; do not regress the
  projected-blip picking.

## Approach

Per `DECISION.md`: keep the proxy blocks **uniform green** (the block colour no
longer encodes status), get separation from a bright edge outline + a small gap,
encode kind with a per-kind glyph on the blip label, and move status + HP + ammo
onto the projected blip (integrity bar coloured by status, ammo pips for
weapons). No new hues; the CRT/phosphor palette stays pure.

## Steps

- [x] In `crates/nova_gameplay/src/hud/nova_os_ship.rs`, add a `cuboid_edges`
      helper that builds a `LineList` `Mesh` of a unit cuboid's 12 edges (no face
      diagonals), so each block can carry a crisp box outline.
- [x] Change `manage_ship_scene` to spawn each block as a **dim translucent
      green fill** cuboid scaled to ~0.9 of the collider size (the gap) plus a
      child **bright-phosphor edge outline** at full size. Keep the `ShipBlock`
      marker on the fill so picking/selection still resolves the section.
- [x] Drop the status material buckets used for blocks
      (`mat_degraded`/`mat_critical`/`mat_inactive`) and rewrite
      `update_ship_blocks` so blocks stay uniform green; instead tint the
      **selected** block's outline toward `NOVA_OS_AMBER` and leave the rest
      phosphor. (Status now lives on the blip, per DECISION.md.)
- [x] Add a `kind_glyph(SectionDamageClass) -> &'static str` helper mapping each
      of the 5 kinds to a distinct ASCII glyph, and prepend it to the blip label
      text in `spawn_ship_blip` (e.g. `"<glyph> HULL-3"`).
- [x] Extend the blip in `spawn_ship_blip` with an **integrity bar** node under
      the label: a track + a fill child whose width fraction = section integrity
      and whose colour = `status_color()`. Update its width/colour each frame in
      `project_ship_blips` from the live view.
- [x] For weapon sections (`Turret`/`Torpedo`) with `SectionAmmo`, render
      **ammo pips** (filled = rounds, empty = remaining capacity) on the blip,
      refreshed alongside the bar; non-weapon sections show no pips.
- [x] Add a `kind_glyph`-mapping unit test (5 distinct non-empty glyphs) and an
      integrity-bar helper test (fraction == `integrity`, colour ==
      `status_color`); add a live-tree test that builds the scene, projects a
      blip for a critically-damaged section, and asserts the block fill material
      is the same uniform-green handle as a nominal section's (regression pin on
      the removed status-recolour) and the blip carries the kind glyph + a bar
      fill sized to the section's integrity.

## Definition of Done

- Blocks render uniform green regardless of status: a critically-damaged
  section's block fill uses the same material handle as a nominal section's
  (test: `blocks_stay_uniform_green_regardless_of_status`).
- Each block has a bright edge outline distinct from its dim fill, and a gap so
  adjacent blocks no longer merge into one mass
  (cmd: `grep -n "cuboid_edges" crates/nova_gameplay/src/hud/nova_os_ship.rs`;
  manual: while orbiting, sections read as separated boxes, not a blob).
- Section kind is shown by a distinct per-kind glyph on every blip label
  (test: `kind_glyph_distinct_per_kind`; manual: hull/thruster/controller/PDC/
  torpedo are each visually distinguishable at default framing).
- Per-section HP shows as an integrity bar on the blip, filled by HP fraction
  and coloured by status; weapon sections also show ammo pips
  (test: `blip_carries_kind_glyph_and_integrity_bar`; manual: a damaged
  section's bar reads short/amber and a full weapon shows full pips).
- The projected-blip picking still works and the palette stays phosphor/amber:
  no new hue constants are introduced
  (cmd: `grep -n "srgb" crates/nova_gameplay/src/hud/nova_os_ship.rs`;
  manual: clicking a block still selects its section).

## Notes

- Builds on the RTT scene in `crates/nova_gameplay/src/hud/nova_os_ship.rs`
  (blocks from `SectionCollider`, blips from `world_to_viewport` of the LOCAL/
  scene position - keep that frame, see `reused-render-pattern-verify-coordinate-frame`).
- Sibling of the side-inspector-panel task; the panel holds the FULL detail, this
  task makes the 3D view legible at a glance.
- Follows `20260726-115339`.

## Work Log (close-out)

**What changed.** In `crates/nova_gameplay/src/hud/nova_os_ship.rs`:

- `cuboid_edges()` builds a `LineList` mesh of a unit cuboid's 12 edges (with
  placeholder NORMAL/UV so the `unlit` `StandardMaterial` pipeline binds). Each
  block now spawns a dim uniform-green fill (alpha 0.22) shrunk to
  `SHIP_BLOCK_FILL_SCALE` (0.86) for the gap, wrapped in a bright box outline at
  full collider size (a `ShipBlockOutline` child).
- The block colour NO LONGER encodes status (per `DECISION.md`): the
  nominal/degraded/critical/inactive material buckets are gone, replaced by one
  shared `mat_fill` plus `mat_outline` / `mat_outline_selected`.
  `update_ship_blocks` now only tints the SELECTED block's outline amber.
- `kind_glyph()` maps each of the 5 kinds to a distinct ASCII glyph, prepended to
  the blip label (`"@ CTL-1"`).
- The blip grew an integrity bar (a dim track + a status-coloured fill sized to
  `bar_fraction()`) and, for weapons, an ammo-pip line (`ammo_pips()`,
  `●●○○○○`). `project_ship_blips` refreshes bar width/colour and pip text each
  frame; `ShipBlip` now records its `bar_fill`/`ammo` child entities. The dot is
  a uniform phosphor marker with an amber border when selected (status left the
  dot too, it rides the bar).

**Tests.** `kind_glyph_distinct_per_kind`, `integrity_bar_and_ammo_pips_track_live_data`,
`blocks_stay_uniform_green_regardless_of_status` (regression pin: a critical and
a nominal section share the same fill handle; off-origin fixture per
`spatial-fixture-off-the-trivial-point`), and `blip_carries_kind_glyph_and_integrity_bar`
(live-tree: bar-fill width == HP fraction, label carries the glyph). All 12
`nova_os_ship` tests pass; `cargo fmt` clean.

**Visual verification.** Ran `NOVA_SHOT_DIR=target/reel BCS_AUTOPILOT=1 BCS_REEL=1
cargo run --example screenshot_nova_os --features debug` (real GPU, exit 0, no
render panic). `nova-os-ship.png` shows separated green wireframe boxes (not a
blob), the selected section's amber outline + amber dot, dim green fills, per-blip
glyph labels, and a full green integrity bar for the nominal controller. This
retired the one real risk - that `LineList` + `StandardMaterial` might not draw
in this pipeline; it does.

**Difficulties.** The chief unknown was whether a `LineList` mesh renders through
the PBR `StandardMaterial` pipeline (no prior use in this repo). De-risked by
supplying NORMAL/UV placeholder attributes and then confirming with the
screenshot harness rather than trusting a headless entity-tree test.
`project_ship_blips` needed splitting into per-component queries
(`&mut Node`/`&mut BackgroundColor`/...) so the dot and its bar-fill/ammo
children can all be updated by entity id without conflicting `&mut` access to the
same component type.

**Self-reflection.** Test-first on the pure helpers was clean; the live-tree test
for the blip subtree was the right altitude (it fails if the glyph or bar wiring
breaks) but cannot prove pixels - the screenshot harness is what closes that gap,
and reaching for it early would have been even better. The screenshot range has
no weapon sections, so the ammo pips were verified only by the unit test and the
ECS tree, not on screen - a follow-up visual check on an armed ship would fully
close that.

**Manual acceptance (pending owner playtest):** kinds are visually distinguishable
at default framing; a damaged section's bar reads short/amber and an armed weapon
shows filled pips; clicking a block still selects its section; the schematic no
longer reads as one green blob while orbiting.
