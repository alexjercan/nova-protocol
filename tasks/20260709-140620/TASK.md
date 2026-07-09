# Center of mass does not shift when sections are destroyed

- STATUS: OPEN
- PRIORITY: 85
- TAGS: v0.4.0,bug,physics,handling

Reported in play (2026-07-09): when a ship loses sections, its center of mass
feels like it stays where it was with the full ship. Losing everything except a
hull + thruster should handle like a small hull+thruster, but it still flies as
if all the sections were attached.

Likely mechanics to investigate: avian computes `ComputedCenterOfMass` /
`ComputedMass` from the body's colliders, so if destroyed sections' colliders
(and `ColliderDensity`) are removed or the entities despawned, avian should
recompute. Suspects:

- Sections may be disabled (`SectionInactiveMarker` / `IntegrityDisabledMarker`)
  but keep their collider + density, so mass properties never change.
- Something may cache mass/inertia at spawn (PD controller tuning, thruster
  force application, flight/autopilot planner constants) and never refresh when
  the body's `ComputedMass`/`ComputedCenterOfMass`/inertia change.
- The multi-thruster planner and PD wobble tuning (see flight overhaul, #55) may
  assume launch-time handling stats.

## Steps

- [ ] Reproduce: in a range/example, destroy most sections of the player ship
      and log `ComputedMass`, `ComputedCenterOfMass` and the inertia tensor
      before/after; determine whether avian's values update.
- [ ] If avian's values do NOT update: trace what the destroy path leaves behind
      (collider, density, mass components on disabled sections) and remove the
      stale contributions when a section is destroyed.
- [ ] If avian's values DO update: find which flight systems (PD controller,
      thruster allocation, autopilot planner, camera weight) cache handling
      stats at spawn and make them read live values.
- [ ] Physics-level test: body with two sections loses one; assert the center of
      mass moves toward the survivor and total mass halves.
- [ ] Feel check in an example range: kill sections and confirm the ship
      handles lighter/differently.

## Notes

- Source: user play report. Wording: "I lose all sections except a hull with a
  thruster and it doesn't feel like a small hull+thruster, but somehow like it's
  still having all the sections attached."
- Related recent work: flight overhaul (#55) uses `ComputedCenterOfMass` for
  muzzle inertia velocity; integrity destroy pipeline in nova_gameplay
  integrity/ + bevy_common_systems.
