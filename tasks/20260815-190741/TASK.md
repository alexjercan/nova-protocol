# Ship skin: derive cladding from structure with quarter-cell shells

- STATUS: CLOSED
- PRIORITY: 78
- TAGS: v0.11.0, ship, editor, render, destruction

## Goal

A ship is structure: a controller plus semantic sections. The skin is DERIVED on
top of that structure as a pure function, with no void gaps. The player never
places a cladding tile. They place a hull "under" the skin and watch the skin
reflow live while they drag it.

Skin plates are destructible. Each carries health and mass. A destroyed plate
leaves a real hole, and pierce damage lets a round carry on into the section
behind it.

Requires: `20260812-131005` link-point snapping for the editor placement path.
Related: `20260813-224826` owns carving; this task owns plate-granularity loss.

## Prototype state

Branch `wfc-shells`, sprout `.cache/sprouts/nova-protocol/wfc-shells`.

Treat the branch as a prototype and a starting point, NOT as history to
preserve. It lands on master as ONE squash commit. Nothing on the branch needs
backward compatibility with anything else on the branch. Removing code is
preferred over adapting it.

What the prototype settled:

- An eight-sample boundary height field describes a cladding tile: 4 corner
  heights plus 4 edge-midpoint heights. Neighbours compute identical shared
  samples, so surfaces meet exactly and no seam geometry is needed.
- Canonicalisation is under C4 (rotation), not D4. A section is placed with a
  `Quat`; no rotation produces a mirror, so folding reflections yields classes
  that cannot be placed.
- The skin is a LOOKUP, not a constraint solve. Every cell's shape follows from
  its neighbourhood. This removes WFC, the stability problem, and the tileset
  totality risk in one step.
- Meshes are generated, so roster size is irrelevant. A whole ship touches a few
  dozen shapes.
- One mesh per surface role, never vertex colours: bevy's PBR fragment ASSIGNS
  `base_color` from vertex colour and `damage_tint` writes that same field.

## Design

### Quarter-cell samples

Three heights (floor/half/full) cannot express a half-step edge. Its true
midpoint is a quarter cell, so a `0 -> half` edge sags to the floor and every
rim tile grows a facet. Move the alphabet to quarter cells, 0..4.

The corner rule still emits only 0, 2 and 4. Midpoints become exact means, so
every live edge is a straight ramp: `(0,4) -> 2`, `(0,2) -> 1`, `(2,4) -> 3`.

This is only affordable because nothing enumerates the roster any more.

### Plates are damageable, not structural

Plates carry `Collider`, `ColliderDensity` and `Health`, and parent to the
structural section they clad. They are NOT integrity-graph nodes: structure
alone decides connectivity, or cladding would hold a severed ship together.

Mass needs no tuning. `base_section` feeds `mass` to avian as DENSITY and avian
multiplies by collider volume, so a quarter-height plate weighs a quarter of a
full cube. Health scales by the same volume.

### Derive versus destroy

The skin is a pure function of structure, so a naive re-derive regrows whatever
combat blew off. Derivation runs on spawn and on editor edits only. Combat
removes plate entities and nothing re-runs. This is the one place the
pure-function story breaks; it is a deliberate seam, not an oversight.

## Plan

1. Commit the prototype baseline: midpoint fix plus the skin render half.
2. Quarter-cell alphabet, radix-5 ids. Delete `every_canonical_shell` and the
   Burnside/exhaustiveness tests.
3. Demolition: `generated_shell_sections`, `PALETTE`, `palette_reading`,
   `shell_description`, `SHELL_HEALTH`, `SHELL_MASS`, `with_generated_shells`,
   `HullSectionConfig.shell`, `HullSectionShell`, `ShellRenderAssets`, the hull
   render branch. `GameSections` returns to ~30 entries.
4. Plates become destructible: collider, density, health, parented to the
   section they clad.
5. Resolve derive-versus-destroy with the spawn/edit-only rule above.
6. Pierce damage. Independent commit; can land before or after the skin.
7. Wiring: skin spawn in `nova_ship` after `build_ship_integrity_graph`; skin
   field on `SpaceshipConfig`; editor re-derive on `PlayerSpaceshipConfig`
   change as `PreviewRole::Display`; toggle in the Tools block plus key legend;
   example emits structure only.
8. Verify by RENDER, not by exit code. The spike regression was invisible to
   `cargo check` and to 18 passing tests.

Steps 2, 3 and 6 are independent. 4 and 5 need 3. 7 needs 3 and 4.

## Open questions

- `damage_tint` clones a material per section. A clad ship has hundreds of
  plates. Decide whether tinting applies to plates before wiring step 4.
- Plate collider fidelity. Start with a cuboid from the shape bounds; the
  meshes are not convex, and cladding is thin.
- Pierce: does a round survive an impact that destroys its target, and does
  leftover damage carry to what is behind?
- Skin styles are hardcoded to start. Content-driven styles are a follow-up.

## Out of scope

- Decoration continuity across tiles. Researched, not built.
- Interactive adaptation, where hovering a thruster reshapes the ship.
- Cladding as an authorable, palette-visible section kind. It is removed.
- Voxel carving of plates. `20260813-224826` owns that.

## Definition of done

- A ship spawns clad, with no void gaps, from structure alone.
- The editor shows the skin reflowing live while a hull is dragged.
- The skin toggle works and no `shell_*` prototype exists in `GameSections`.
- Plates take damage, die, and leave a hole a round can pass through.
- Rendered output is inspected and attached, not merely produced.
- `cargo check`, `cargo fmt --check`, affected `--lib` tests, and `content lint`
  pass. Docs that describe the removed cladding sections ship with the change.
