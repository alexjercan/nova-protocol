# Decision: Free the block-color channel from status; blocks stay uniform green, status moves to the blip integrity bar

- DATE: 20260728-115435
- STATUS: ACCEPTED
- TASK: 20260728-115435
- TAGS: decision, nova_os, ship, hud, ui

Partial supersede note: the "status rides the blip integrity bar + ammo pips"
part is superseded by tasks/20260728-125514/DECISION.md; the uniform-green
blocks, kind glyph, and outline separation still stand.

## Context

The `ship` schematic (from `20260726-115339`) renders each section as an unlit
cuboid whose COLOR encodes STATUS (green nominal / muted degraded / amber
critical / dim neutralized). When the whole ship is healthy every block is the
same green, which is the owner's "just a green blob" playtest complaint. This
task must (a) separate adjacent sections, (b) encode section KIND, and (c)
surface per-section HP/ammo in the view - all while "keeping the CRT/phosphor
palette."

That palette is the forcing constraint: it is monochrome green + one amber
accent. Colour is a single channel and it is already spoken for by status, so
it CANNOT also carry five distinct kind-hues without either abandoning the
monochrome look or dropping status. The owner resolved the fork directly in the
plan Q&A: "keep the colour green ... think of some other thing for status, for
now keep it just green."

## Decision

- Blocks render as a **uniform green** proxy (a dim translucent fill + a bright
  phosphor edge outline), independent of section status. The per-status block
  recolour (`mat_degraded` / `mat_critical` / `mat_inactive`) is removed.
- **STATUS moves onto the blip**: each section's projected blip carries an
  integrity bar filled by HP fraction and coloured by status (green -> amber),
  plus ammo pips for weapon sections. This is the "some other thing for status"
  the owner asked for.
- **KIND** is encoded by a per-kind glyph/icon on the blip label (the CRT font's
  ASCII), reinforcing the code prefix (`HULL`/`THR`/`CTL`/`PDC`/`TRB`) that
  already names the kind. No new hues are introduced.
- **SEPARATION** comes from the bright edge outline plus a small gap (fill
  scaled below the collider extents), so adjacent blocks no longer merge.

## Alternatives considered

- **Per-kind hue in the 3D scene** (green hull / cyan thruster / teal
  controller / amber weapons) with status pushed onto the blip bar. Rejected by
  the owner: it bends the monochrome phosphor palette, and amber already reads
  as "critical," so a pale-amber weapon block would be ambiguous.
- **Full per-kind 3D silhouettes** (distinct proxy meshes: nozzle cone,
  turret barrel, torpedo tube, antenna). Deferred: high authoring risk sizing
  five readable shapes from arbitrary section colliders at schematic scale, for
  a legibility win the outline + label glyph already delivers. A blip-label
  glyph is the "small icon" the task allows; silhouettes can be a later polish
  task if the glyph proves insufficient.
- **Keep status on the block colour** and encode kind only via a label glyph.
  Rejected by the owner's explicit steer to make the blocks read green.

## Consequences

- The 3D scene becomes a clean green wireframe schematic - directly answers the
  "green blob" complaint and keeps the palette pure.
- Status is no longer glanceable from the block colour alone; a player reads
  damage from the blip bar (and the readout / future side panel). Acceptable:
  the sibling inspector-panel task (`20260728-115430`) also surfaces status, and
  the bar is on every blip.
- `update_ship_blocks` no longer swaps materials by status. The selected block
  is highlighted by tinting its OUTLINE amber (a cheap in-scene selection cue)
  rather than its fill.
