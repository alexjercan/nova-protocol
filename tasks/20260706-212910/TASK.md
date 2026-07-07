# Asteroid RigidBody husk lingers after collider child explodes

- STATUS: CLOSED
- PRIORITY: 60
- TAGS: v0.4.0, bug

Surfaced while testing collider/integrity changes. When an asteroid is destroyed, avian logs
`Dynamic rigid body <id> has no mass or inertia. This can cause NaN values.` for a short
window.

Cause: an asteroid is a `RigidBody::Dynamic` body whose `Collider` + `Health` live on a child
entity (see `insert_asteroid_collider` in nova_scenario/objects/asteroid.rs). On destruction,
the child (collider + mesh + health) explodes and despawns, but the asteroid parent entity -
the RigidBody - remains until the scenario unloads. A dynamic body with no colliders has no
mass/inertia, hence the warning; the empty husk also lingers (minor leak) and is invisible
(the mesh was on the despawned child).

Fix direction: when an asteroid's collider/health node is destroyed, despawn the asteroid
root too (or convert it to static / remove RigidBody). Pre-existing; predates the v0.3.1
integrity work. Low priority (cosmetic warning + short-lived husk cleaned on scenario unload).

## Steps

- [x] Observe `Add IntegrityDestroyMarker` on the collider/health node; when its `ChildOf`
      parent carries `AsteroidMarker`, mark that parent with an `AsteroidHuskDespawn` tag.
- [x] Despawn tagged husks in an `Update` system, deferred one frame so the destruction
      observers (explosion fragments + node despawn) all run first.
- [x] Co-located tests: destroying an asteroid node despawns the husk; destroying a
      non-asteroid parent's node leaves that parent alone.
- [x] Full check suite green: `cargo test --workspace` (incl. examples_smoke under Xvfb),
      `cargo clippy --workspace --all-targets`.

## Resolution

An asteroid root is now despawned once its collider/health node is destroyed. The
`on_asteroid_node_destroyed` observer captures the node's `ChildOf` at the moment
`IntegrityDestroyMarker` lands (before the node despawns) and, if the parent is an
`AsteroidMarker`, tags it `AsteroidHuskDespawn`. A separate `despawn_asteroid_husk` `Update`
system then `try_despawn`s the tagged root next frame - deferring past the destruction
observers so explosion fragments (separate `TempEntity`s, not children) spawn normally and the
node despawns first, leaving the root childless before it is removed. This clears the
invisible `RigidBody::Dynamic` husk and silences avian's mass/inertia warning.

Marking-then-deferred-despawn (rather than despawning inside the observer) avoids racing the
other `Add IntegrityDestroyMarker` observers in the destruction pipeline.

## Notes

Removing the husk also unmasks torpedo target-loss: previously a torpedo locked onto an
asteroid kept a live (invisible) entity to chase after the visible rock exploded. Now the
root despawns, so the torpedo's target-loss path (already handled) fires as it should.
