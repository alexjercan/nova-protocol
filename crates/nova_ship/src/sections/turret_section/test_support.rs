//! Rig helpers shared by more than one turret submodule's tests.

use bevy::prelude::*;

use super::*;

/// The muzzle joint entity of `turret`.
pub(super) fn muzzle_entity(app: &App, turret: Entity) -> Entity {
    **app
        .world()
        .get::<TurretSectionMuzzleEntity>(turret)
        .expect("the turret records its muzzle")
}

/// Collect every joint entity of `turret` in DFS order (root first).
pub(super) fn joint_entities(app: &App) -> Vec<Entity> {
    let world = app.world();
    world
        .iter_entities()
        .filter(|e| world.get::<TurretJointMarker>(e.id()).is_some())
        .map(|e| e.id())
        .collect()
}
