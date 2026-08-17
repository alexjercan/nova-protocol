# Sever destroyed interior sections into physical wreck fragments

- STATUS: CLOSED
- PRIORITY: 49
- TAGS: v0.11.0, ship, destruction, physics, combat

## Goal

Destroy a depleted spaceship section even when it is not a graph leaf. If its
removal disconnects surviving structure, turn each detached connected component
into an independently simulated wreck body instead of leaving an invisible
compound-body connection or deleting healthy structure.

## Accepted design

- Direct health depletion destroys any section immediately. The leaf rule
  remains only for ordering the existing structural-collapse peel.
- Partition surviving direct section children after all destruction/pruning in
  the frame. A redundant graph stays one body.
- The original spaceship root keeps the component ranked by: most live
  controller sections, greatest surviving maximum section health, then stable
  entity order. Without a live controller, the largest component wins.
- A wreck fragment keeps its largest component on later splits. Every other
  component gets a new `RigidBody::Dynamic` wreck root.
- Wreck roots are inert unsigned debris, not spaceships: no scenario identity,
  allegiance, AI, player authority, capability, or defeat events. Their
  sections remain damageable.
- Detached sections use the existing inactive capability seam but retain their
  health-derived visual tint rather than turning black merely because the
  command bus is gone.
- Fragments persist until scenario teardown. Scenario scoping must be explicit,
  not a fake infinite `TempEntity` timer. Empty fragment roots despawn.
- Preserve angular velocity and rigid-body point velocity across each new centre
  of mass. Then apply a `1 u/s` separation kick away from the destroyed section,
  balanced by mass so total linear momentum does not change. No angular kick.
- Lower the default structural-collapse threshold from 25 percent to 5 percent.
  Per-ship overrides and the existing `<=` boundary remain; 0 means dismantle
  every section.
- Keep the current generic cube burst for destroyed sections. Better
  representation-independent mesh destruction is separate task 20260817-154330.
- Existing cladding follows its owning section. Existing root aggregation,
  neutralization, exact-once scenario outcomes, and collapse stall handling stay
  authoritative.

## Required proof

- Reproduce a depleted non-leaf remaining black before the fix.
- A redundant connection loses the dead section but does not split the body.
- A bridge cut creates independent rigid bodies and reassigns `ColliderOf`.
- A rotating split preserves point velocity before the balanced kick.
- Each component receives the accepted 1 u/s outward kick before mass-weighted
  balancing, and the final kick conserves linear momentum.
- Controller-first and no-controller/fragment tie breaks are deterministic.
- Detached weapons, controllers, and thrusters are inert but healthy sections
  retain their visual damage state and remain destructible.
- A fragment can split again and its empty root is removed.
- Scenario teardown removes persistent fragments.
- Structural collapse still peels over several frames and starts at 5 percent,
  not 25 percent.
- Add a rendered systems range and a capital-scale bounded-body stress case.

## Implementation notes

- Directly depleted ship sections now insert `IntegrityDestroyMarker` at any
  graph degree. Collapse-disabled healthy sections still use the generic
  leaf-first path.
- Depletion snapshots the pre-removal section mass centroid, body pose and
  velocities. The post-prune partition ranks connected components and reparents
  detached sections to inert `ShipWreckFragmentMarker` dynamic roots.
- Avian hierarchy changes can land after its normal mass queue. The sever motion
  pass therefore forces mass recomputation before restoring body origins,
  rigid-point velocities and the balanced fracture kick.
- The original body keeps the controller-ranked component. Fragment roots rank
  by health on later cuts. Detached sections use `SectionInactiveMarker`, while
  damage tint now follows health rather than capability state.
- Scenario lifecycle scopes persistent wreck markers explicitly. Empty wreck
  roots clean themselves up.
- `examples/systems/section_severing.rs` renders and asserts the hole, collider
  reassignment, command-root retention, and intact inert wreck.

## Done when

- Player-path evidence shows an interior section becoming a real hole and a cut
  component drifting free.
- Affected crate tests, probe catalog drift, content lint, web CI, formatting,
  and rendered output pass.
- Owner reviews code and playtests severing before closure.
