# Model the sections to the thruster's standard

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog

## Goal

Give the sections real models. The thruster is the quality bar - it is the
one section that already reads as built hardware - and everything else
should come up to it.

Owner framing (2026-08-31): "new model for the torpedo bay and actually
maybe models for all sections (except the thrusters which are now the
baseline in terms of quality for sections)".

## Start with the torpedo bay, because it is a placeholder

`assets/base/gltf/torpedo-bay-01.glb` is a UNIT CUBE. One node, one mesh,
bounds -1..1 scaled by 0.5. It has no tube, no mouth and no directionality
at all, which is why nothing about the launch reads as coming out of a
launcher.

The launch mechanic is already built and waiting for it. A torpedo coasts
inert for `ignition_delay` seconds (0.6 by default) with its colliders
disabled and guidance suspended - see task 20260822-204201 stage 4. That
window is exactly where an emergence animation lives, so this is art hanging
off a hook that already exists, not new systems work.

The bay fires out of its -Z face. `link_points` deliberately leaves that
face unlinkable so it can be a muzzle, and the authored `spawn_rotation`
now turns the launch axis onto it. Model the tube mouth THERE.

Wanted, in the owner's words: "torpedo bay with doors that open to let the
torpedo out and stuff like that". So the bay wants an animation track, not
just a mesh - which is the first section that does. Decide how an authored
section declares an animation before modelling the second one.

## The rest

- Every section except the thruster, which is the reference.
- The PDC turret is a special case: the stow task needs more geometry around
  the mount for the turret to disappear into. Agree the turret shape with
  that task before either commits.
- Keep the sections readable at gameplay distance and at the silhouette
  level - a player has to tell a turret from a bay at a glance, which is the
  same constraint the damage effects were written against
  (`damage_sparks.rs`: "a player has to be able to tell at a glance that it
  is the thing shooting at them").

## Watch for

- Section colliders come from the spec, not the mesh, so a prettier mesh
  must not silently change hitboxes. Check `collider:` per section.
- `render_mesh_transform` is visual-only and does not move the launch point.
  Do not use it to hide a spawn-point mismatch.
- Asset coverage checks are advisory and exit 0. Missing art warns, it does
  not gate.

## Done when

- The torpedo bay is a launcher with a tube, and a torpedo is seen leaving
  it rather than appearing beside it.
- An authored section can declare an animation, and the bay doors use it.
- Every non-thruster section has a model at the thruster's standard.
- Silhouettes stay readable at combat range.
