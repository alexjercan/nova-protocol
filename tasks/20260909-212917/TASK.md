# Velocity and gravity spheres inscribe the hull

- STATUS: OPEN
- PRIORITY: 86
- TAGS: v0.14.0, bug, hud

## Defect

The velocity sphere and the gravity sphere have a fixed radius. A big hull
pokes through them.

- `crates/nova_hud/src/lib.rs` `setup_hud_velocity`: `radius: 5.0` for the
  velocity widget and `radius: 5.6` for the gravity widget, world units
  (50 m and 56 m), the same for a 3-cell skiff and a 30-cell carrier.
- `crates/nova_hud/src/velocity.rs` `VelocityHudConfig::radius` is the orbit
  radius of the cone and the sphere shell.
- `crates/nova_hud/src/flight_status.rs:43` documents the 5.6 u shell as a
  constant; the speed chip parks beside it.

Owner report (2026-09-09): "it has a fixed radius, so a really big ship would
not be able to display it properly, we would see it clipping into the ship, so
it needs to scale with the ship and inscribe around it."

## Fix

The hull already publishes its size: `HullRadius`
(`crates/nova_ship/src/sections/hull_radius.rs`), centre of mass to the outer
face of the furthest live section, every tick.

- Derive the velocity shell radius from `HullRadius` plus an authored margin
  in meters. Keep the gravity shell nested a fixed step outside it so the two
  never z-fight.
- Read the radius every frame, not at spawn: a hull that loses sections
  shrinks. Ease the change so a severed section does not snap the shell.
- Move the speed chip and the mode chip with the shell.
- Author the margin in meters through the `nova_events` quantity types.
  Convert at the render boundary.

## Proof

- A `system_` range in `examples/systems/` that spawns `block_skiff` and
  `block_carrier` as the player hull in turn and asserts the shell radius is
  at least `HullRadius` plus the margin, then severs a section and asserts the
  shell follows. Put its slugs on the roster in
  `crates/nova_probe_cli/tests/catalog_drift.rs`.
- Re-shoot `screenshot_combat_hud` (or the loop that shows the sphere) with
  the carrier so the wiki image shows the shell clear of the hull.
- Update the flight instruments wiki page if it names the radius.
