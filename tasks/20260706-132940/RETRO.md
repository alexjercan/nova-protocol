# Retro: mesh slicer crash guard (task 20260525-132940)

## What was asked
Guard against edge cases in the mesh slicer to prevent crashes.

## What happened
The slicer algorithm itself is external (bevy_common_systems), so its panic-safety is
that repo's job (filed a v0.4.0 follow-up). But we found a genuine local cause: the
slicer was being *triggered* on meshless entities. `ExplodableEntity` propagates up to
ship/section roots, which render via a `WorldAssetRoot` gltf scene and have no `Mesh3d`.
Added `With<Mesh3d>` to `on_explode_entity`'s trigger query so only entities with a mesh
to slice are handed to the slicer.

## Lessons
- When the crashing code is in a dependency, still check the *call site*: we were
  feeding it input it couldn't handle. Guarding the call site is the right fix on our
  side even when the algorithm's panic-safety belongs upstream.
- `ExplodableEntity` is not where you'd expect it: `on_add_explodable_entity` moves it
  from a child to its parent root, so the entity that eventually explodes is the root,
  not the mesh-bearing child. Worth remembering when reasoning about the integrity chain.
- Rendering via `WorldAssetRoot`/gltf means the root has no `Mesh3d` (meshes live in
  child scene entities) - a recurring gotcha for anything that queries `Mesh3d` on ships.
