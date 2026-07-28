# NOVA OS ship app: clearer section rendering (separation + per-section type/HP/ammo indicators)

- STATUS: OPEN
- PRIORITY: 31
- TAGS: v0.9.0,feedback,feature,ui,hud,gameplay

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

## Notes

- Builds on the RTT scene in `crates/nova_gameplay/src/hud/nova_os_ship.rs`
  (blocks from `SectionCollider`, blips from `world_to_viewport` of the LOCAL/
  scene position - keep that frame, see `reused-render-pattern-verify-coordinate-frame`).
- Sibling of the side-inspector-panel task; the panel holds the FULL detail, this
  task makes the 3D view legible at a glance.
- Follows `20260726-115339`.
