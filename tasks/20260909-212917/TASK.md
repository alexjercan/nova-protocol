# Velocity and gravity spheres enclose the live hull

- STATUS: OPEN
- PRIORITY: 86
- TAGS: v0.14.0, bug, hud

## Defect

The velocity sphere and the gravity sphere have a fixed radius. A big hull
pokes through them.

- `crates/nova_hud/src/lib.rs` `setup_hud_velocity`: `radius: 5.0` for the
  velocity widget and `radius: 5.6` for the gravity widget, world units
  (50 m and 56 m), regardless of hull size. The shipped skiff has 21 sections;
  the 33-cell-long carrier has 2,081 sections and cannot fit inside the fixed
  112 m outer diameter.
- `crates/nova_hud/src/velocity.rs` `VelocityHudConfig::radius` is the orbit
  radius of the cone and the sphere shell.
- `crates/nova_hud/src/flight_status.rs:43` documents the 5.6 u shell as a
  constant; the speed chip parks beside it.

Owner report (2026-09-09): "it has a fixed radius, so a really big ship would
not be able to display it properly, we would see it clipping into the ship, so
it needs to scale with the ship and inscribe around it."

## Plan

`HullRadius` (`crates/nova_ship/src/sections/hull_radius.rs`) is not the
containment contract. It measures from the centre of mass to the outer face
along each section's radial ray, explicitly not to the furthest collider point,
because flight uses it as a structural arm and arrival distance. Changing that
meaning would retune flight and still leave the HUD contract implicit.

Add a separate `HullEnvelopeRadius`: the smallest radius this section-collider
model can publish that contains every live section collider about the live
centre of mass. Publish it beside `HullRadius` in the same one-pass fixed-tick
scan so the two values cannot disagree about which sections are live. Compute
the exact furthest point for each supported `SectionCollider` shape in its local
frame, rather than using a rotated AABB whose empty corners would unnecessarily
inflate shells around rotated long sections. The authored HUD clearance remains
outside this physical envelope to cover the visual skin and keep the projection
clear. Use a fixed 5 m hull clearance and retain the current 6 m separation
between the velocity and gravity shells. These are fixed physical gaps, not
percentages of hull size.

The shell must share the envelope's centre. Centering a COM-relative radius on
the ship root is incorrect after asymmetric construction or damage moves the
COM.

1. Establish the visual reproduction before changing production code:
   - add a dedicated `screenshot_hud_shell` example and its explicit workspace
     `[[example]]` entry first;
   - stage `block_carrier` as the player ship with nonzero velocity and a
     dominant gravity well, so both fixed shells are visible around the same
     hull;
   - use one pinned broadside camera, fixed capture resolution, deterministic
     setup, and the normal `AppBuilder` path so the before and after shots differ
     only because of the fix;
   - capture `hud-shell-carrier.png` from the unfixed build into
     `target/hud-shell-comparison/before/`, inspect it, and record in this task
     that the 50 m and 56 m shells clip through the carrier;
   - keep the example unchanged through the implementation so it is the visual
     test fixture, not a separately restaged after-shot.
2. Add and export `HullEnvelopeRadius` in `nova_ship`:
   - define it beside `HullRadius` as a derived world-unit component;
   - rename the private publisher to `publish_hull_radii`, since its one pass
     now publishes both contracts;
   - add a `SectionCollider` helper that returns the exact furthest distance
     from an arbitrary COM-relative point for cuboid, sphere, capsule, and
     cylinder shapes, including section rotation;
   - derive `HullRadius` and `HullEnvelopeRadius` together from the same live
     section query and `ComputedCenterOfMass`;
   - update an existing component with change detection and insert it when
     absent, matching `HullRadius` lifecycle and dead-root behavior;
   - register and export it through the ship preludes.
3. Define the shell contract in `nova_hud`:
   - velocity radius = live `HullEnvelopeRadius` + `Meters(5.0)`;
   - gravity radius = velocity radius + `Meters(6.0)`;
   - convert the `nova_events` `Meters` values only at the render boundary;
   - grow immediately so an expanding radius cannot expose the hull;
   - shrink with a frame-rate-independent 150 ms exponential half-life using
     `alpha = 1 - 2^(-dt / half_life)`, then snap to the target within 1 cm, so
     repeated damage retargets without restarting a fixed-duration transition;
   - ease one shared envelope radius and derive both shells from it every frame,
     so their 6 m separation cannot drift during convergence.
4. Extend the directional HUD's live state instead of baking radius only at
   spawn. Each frame, read the target's `HullEnvelopeRadius`, `Position`,
   `Rotation`, and `ComputedCenterOfMass`. Transform the local COM into world
   space and use it immediately as the common centre of the velocity sphere,
   gravity sphere, and their orbiting cones. Do not ease COM movement: exact
   centering and containment take precedence over smoothing a damage-driven COM
   shift.
5. Synchronize every radius consumer from the eased value:
   `DirectionalSphereOrbit.radius`, the sphere child's translation and scale,
   and `DirectionSphereMaterial.radius`. Keep the gravity shell at the exact
   authored separation so the shells do not z-fight.
6. Handle startup without showing the carrier through the old 50 m shell. Do
   not reveal a shell until the first `HullEnvelopeRadius` is available.
   Preserve the gravity widget's separate rule that hides it in flat space.
7. Move the speed and mode chips with the outer gravity shell:
   - anchor each chip from the live COM to the screen-right edge of the current
     outer gravity radius, including while the gravity shell itself is hidden in
     flat space;
   - add a fixed 12 px horizontal gap beyond the projected shell edge;
   - retain the current -90 px speed-chip and -114 px mode-chip vertical
     offsets and existing off-screen behavior;
   - calculate the world edge after the chase camera updates and before screen
     indicator projection, so placement uses the camera pose rendered that
     frame and follows both hull size and camera zoom.
8. Keep the generic directional-orbit mechanism in `nova_gameplay` independent
   of ships. Ship radius, COM, metric margins, child mesh synchronization, and
   chip placement remain HUD responsibilities.
9. Add focused unit coverage:
   - each `SectionCollider` shape reports its exact furthest point under
     translation and rotation;
   - `HullEnvelopeRadius` contains all live colliders, follows a moved COM, and
     shrinks when an outer section becomes inactive without changing the
     existing `HullRadius` result;
   - the velocity shell stays 5 m outside the physical envelope and the gravity
     shell stays exactly 6 m outside the velocity shell;
   - metric conversion, COM centering, immediate growth, 150 ms shrink
     half-life, 1 cm completion snap, child transform/material updates, startup
     visibility, chip placement 12 px outside the projected outer shell with
     the existing vertical offsets, and unchanged gravity visibility.
10. Add a dedicated `system_hud_shell` range in `examples/systems/`. Spawn
   `block_skiff` and `block_carrier` as the player hull in turn and assert that
   both shells clear `HullEnvelopeRadius` from the live COM. Sever the live
   section that determines the envelope, wait for convergence, and assert that
   the envelope and shell shrink while remaining nested. Assert that both
   flight chips remain outside the projected gravity shell. Put every
   `outcome:` slug on the roster in
   `crates/nova_probe_cli/tests/catalog_drift.rs`.
11. Re-run the unchanged `screenshot_hud_shell` example after the fix into
    `target/hud-shell-comparison/after/`. Compare the two captures at native
    resolution and side by side. The after-shot must show the carrier wholly
    inside both COM-centred shells, the gravity shell outside the velocity
    shell, and both chips clear of the outer edge. Keep `screenshot_combat_hud`
    unchanged so its armed-hull combat showcase retains its existing purpose.
12. Register `screenshot_hud_shell` as the producer of
    `web/src/assets/wiki-hud-shell.png` and add that focused figure under Flight
    readouts in `web/src/wiki/hud.md`. State that the shells follow the live
    hull, remove the fixed 56 m claim from `flight_status.rs`, and add one
    concise Interface & HUD entry to `[Unreleased]`.

## Verification

- Run the affected `nova_ship` and `nova_hud` unit tests.
- Run `system_hud_shell` correctness-only, then rendered, and inspect
  `checks.json` and `report.html`.
- Run the `nova_probe_cli` catalog-drift test.
- Capture `screenshot_hud_shell` before and after into separate directories;
  inspect both at native resolution and as a side-by-side comparison.
- Run the screenshot producer-map checks after registering
  `wiki-hud-shell.png`.
- Run `cd web && npm run ci` after the wiki and screenshot mapping changes.
