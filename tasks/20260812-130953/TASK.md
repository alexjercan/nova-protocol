# Graph adjacency replaces the distance==1.0 integrity glue (parts gate)

- STATUS: OPEN
- PRIORITY: 80
- TAGS: v0.10.0,ship,integrity

Goal: replace the distance==1.0 integrity glue with graph adjacency so
parts-based ships weld correctly. This is THE gate for parts ships: part
origins sit at varying distances, so today's glue would leave a parts ship
as disconnected sections.

Context:
- Design: tasks/20260812-100246/SPIKE.md R2 D2 (graph-first integrity).
  ConnectedTo stays the representation; edges become the truth.
- Edge sources: derived AABB-touch from SectionCollider::aabb_half_extents
  now; UNION seam ready for authored link-point mates later (see the
  link-points task). Unit-cube parity preserved by construction.
- Current glue: build_integrity_relations, Add<ColliderOf>-keyed.

Scope:
- AABB-touch adjacency derivation with epsilon; parity tests pinning that
  every existing cube ship produces the exact same edge set as today.
- Union seam for a second edge source (mates) without behavioral change.
- Ship lint: keep the connectivity check consistent with the new derivation.
- Out of scope: severing (component split on destroy), link-points, and
  ConvexHull colliders - separate tasks per the SPIKE escalation.

DoD:
- Parity test: cube ships edge-set identical before/after.
- Harness proof: a parts-layout ship (racer 7-part footprint from
  scripts/part-recipes/) spawns welded and survives flight in a player-path
  example.
- No content or save format changes.
