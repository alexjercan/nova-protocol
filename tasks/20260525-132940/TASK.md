# Ensure mesh slicer does not crash the game

- STATUS: CLOSED
- PRIORITY: 90
- TAGS: v0.3.1, bug

Guard against edge cases in the mesh slicer to prevent crashes. Legacy #88.

## Steps

- [x] Locate the mesh slicer and its crash surface.
- [x] Guard the local trigger path against feeding the slicer un-sliceable input.
- [x] File the remaining slicer-core hardening against the external crate.
- [x] Verify build --all-targets, clippy, fmt.

## Resolution (CLOSED)

The slicing algorithm (ExplodeMeshPlugin / ExplodeMesh -> ExplodeFragments) lives in the
external bevy_common_systems crate, so its internal edge-case hardening belongs there
(filed follow-up 20260706-* v0.4.0). What is in this repo is the trigger and consumer:

- on_explode_entity inserted `ExplodeMesh` on any ExplodableEntity that was destroyed.
  But ExplodableEntity is propagated up to parent roots (on_add_explodable_entity), and
  ship/section roots render via a WorldAssetRoot gltf scene - they have no Mesh3d of
  their own. Handing the slicer a meshless entity is exactly the kind of edge case that
  crashes it. Fix: added `With<Mesh3d>` to the trigger query so only entities that
  actually have a mesh to slice are handed to the slicer. Asteroids (Mesh3d +
  ExplodableEntity on the same, non-child entity) still explode; meshless roots are
  cleanly skipped (they never produced valid fragments anyway).
- handle_entity_explosion (the consumer) was already defensive: error+continue on
  missing mesh/material, and Collider::convex_hull_from_mesh(..).unwrap_or(sphere) as a
  fallback for degenerate fragment meshes. Left as-is.

Verified: build --all-targets, clippy, fmt green. Runtime not exercised (no display);
the guard is correct by construction (you cannot slice an entity with no mesh).

Self-reflection: like the other bevy_common_systems tasks, the crash lives in external
code, but here there was a genuine local contribution - we were feeding the slicer
input it could not handle. Guarding the call site is the right fix on our side even
though the panic-safety of the algorithm itself is the external crate's job.
