# Generated content review

`nix develop --command cargo run content gen` rewrote every base file. Four
came out different. Each was compared entry by entry against `HEAD`, keyed by
id, so a move is told apart from an edit.

## `assets/base/sections/base.content.ron`

18 prototypes -> 16.

- Removed: `heavy_torpedo_section`, `siege_railgun_lance_section`.
- Added: none.
- Retained entries with a changed body: NONE. Every kept prototype is byte
  identical to its `HEAD` text.
- Order changed by the family split. Was: reinforced hull, the three drives,
  controller, the three light hulls, the four mounts, the two bays, the two
  lances, docking port. Now: the four hulls, the three drives, controller, the
  four mounts, the two bays, the lance, docking port.

## `assets/base/ships/base.content.ron`

18 designs -> 8. 67 lines added, 49,920 removed.

- Removed: `block_carrier`, `block_claw`, `block_cleanup_leader`,
  `block_raider`, `block_skiff`, `block_tug`, `block_warship`,
  `block_wreck_bridge`, `block_wreck_shoulder`, `block_wreck_spine`.
- Added: none.
- Retained entries with a changed body: NONE.
- Relative order of the eight kept designs is unchanged.

The 67 added lines are the trailing-comma and bracket shifts left where the
removed entries sat.

## `assets/base/styles/base.content.ron`

5 styles -> 4. 0 lines added, 88 removed.

- Removed: `placeholder`.
- The four kept styles are untouched, which the zero added lines prove on
  their own.

The four magenta `placeholder_*` greeble models stay under
`assets/base/gltf/greebles/`. They are art, not content, and
`web/src/create/base-content.md` already describes them as pieces no base
style uses.

## `assets/base/scenarios/menu_duel.content.ron`

One change. The Breaker bay's

    source: Prototype(id: "heavy_torpedo_section")

became `source: Inline(...)`: the whole section, authored in the duel. It
carries its own name and description, health 100, the explosion destroy sound,
the 1x1x2 cuboid collider, all 13 link points, `hide_in_editor: true`, the
Cracks and Sparks damage effects, the `door_petal_` MuzzleDoor track, the
`bay_tube.glb` mesh with its spawn pose and recess, the siege ballistics
(fire_rate 1.0, blast 450 m / 2000, projectile_health 5000), its three sounds,
the Breaker torpedo type with its red tint and 700 m/s cruise, and
`Limited(6)` ammunition on a 10 s single-round reload.

No other base scenario changed.

## Bench fixtures

- `hunt.content.ron`, `slingshot.content.ron`: one line each,
  `block_raider` -> `block_picket`.
- `arsenal.content.ron`: the player hull is now authored inline out of base
  prototypes (railgun spine, a Serpent bay to port, a Lance bay to starboard,
  two mounts), so the fixture no longer needs `block_warship`. Its section
  triggers drop from 18 keys to 5. The ram target moved `block_carrier` ->
  `block_hauler` and the late hostile `block_raider` -> `block_picket`.

Arsenal's revision therefore moved. A performance set taken against the old
file is not a matched comparison for the new one.
