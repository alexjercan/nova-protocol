# Asteroid RigidBody husk lingers after collider child explodes

- STATUS: OPEN
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
