# Harden mesh slicer against degenerate meshes (bevy_common_systems)

- STATUS: CLOSED
- PRIORITY: 68
- TAGS: v0.4.0, bug, crates

Follow-up from task 20260525-132940. The mesh slicer (ExplodeMeshPlugin / ExplodeMesh)
lives in the external bevy_common_systems crate, so guarding its internal edge cases
(empty/degenerate meshes, slice planes that miss, zero-area triangles, non-triangle-list
topologies) must happen there, not in this repo.

On the nova side we already: (a) only trigger the slicer on entities that have a Mesh3d
(on_explode_entity), and (b) fall back to Collider::sphere when convex_hull_from_mesh
fails (handle_entity_explosion). The remaining hardening is the slicing algorithm itself
returning gracefully instead of panicking on bad input - do that in the external repo.

Closed here: the remaining work lives entirely in bevy_common_systems and is now tracked
there as its task 20260708-134706. Nothing more to do in this repo.
