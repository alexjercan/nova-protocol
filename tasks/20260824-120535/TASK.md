# Engineer readout: the ship's numbers on the build screen

- STATUS: OPEN
- PRIORITY: 50
- TAGS: v0.12.0,editor,ui

Child of the v0.12.0 editor epic (`20260812-131912`). The engineer's panel
the epic's 2026-08-19 addendum asked for; the attitude readout already
shipped as its first tenant. Research: `tasks/20260815-231945/EDITOR-STATE.md`
section 5. First release-cut candidate per the epic.

## Goal

Extend the attitude rail row into the engineer readout: the ship's derived
numbers, live, while it is being built. Factorio is the reference - the
consequence of a choice is visible at the moment the choice is made. A big
ship that genuinely cannot turn must read as a design constraint, not a bug.

## What exists

- `AttitudeEnvelope` (nova_ship/src/physics/attitude.rs:56-131): `ceiling()`,
  `binds()` (Torque vs Structure with `label()`), `sustained_turn_rate()`.
  Editor side: `preview_envelope` (nova_editor/src/attitude.rs:18-70)
  assembles it from the build state; `readout_line` (:80-90) prints
  "Turn X rad/s2 / <limit>" into the rail row (ui/mod.rs:196-207).

## The panel's lines, all one step from shipped code

- Flip time (bang-bang 180): `2 * sqrt(pi / ceiling())` - the formula is
  pinned in attitude.rs tests (:250, :262). One line.
- Mass + centre of mass: computed and DISCARDED in `preview_envelope`
  (attitude.rs:39-46). Return `MassProperties3d` alongside the envelope.
- Total thrust: sum `ThrusterSectionConfig.magnitude`
  (nova_ship/src/sections/thruster_section.rs:40) over the build state's
  thrusters; per-axis split via `exit_normal` (forward is -Z).
- Max acceleration: thrust / mass, from the two above.
- Weapon totals (count, damage, ammo, reload): the per-part lines exist in
  the gallery focus card (gallery/catalog.rs:119-208); a ship-level sum is
  aggregation only.

## Explicitly OUT (separate model spikes if ever wanted; do not gate the panel)

- Power: no power/energy stat exists anywhere in the section configs. The
  gameplay stat must be invented before a line can show it.
- Weapon coverage: per-joint rotation limits exist
  (turret_section/config.rs:71, :77) but a hull-shadowed solid-angle union
  is genuinely new derivation work.

## Scope note

This is a SURFACE, not one number: one panel, each stat a row, updating on
place/delete. After the foundations task it reads the ACTIVE edit context's
build state. Do not let each stat invent its own display.

## Done when

- The panel shows flip time, mass, thrust and max acceleration live while
  building, alongside the existing attitude line, and updates on
  place/delete.
- A screenshot range covers it; the numbers agree with the physics for a
  pinned fixture ship.
