//! This module contains all the sections of a spaceship.

use avian3d::prelude::ComputedCenterOfMass;
use bevy::prelude::*;

pub mod ammo;
pub mod base_section;
pub mod controller_section;
pub mod hull_section;
pub mod projectile_hooks;
pub mod thruster_section;
pub mod torpedo_section;
pub mod turret_section;

pub mod prelude {
    pub use super::{
        ammo::prelude::*, base_section::prelude::*, controller_section::prelude::*,
        hull_section::prelude::*, live_structure_anchor, projectile_hooks::prelude::*,
        thruster_section::prelude::*, torpedo_section::prelude::*, turret_section::prelude::*,
        SpaceshipRootMarker, SpaceshipSectionPlugin, SpaceshipSectionSystems,
    };
}

/// This will be the root component for the entire spaceship. All other sections will be children
/// of this entity.
#[derive(Component, Clone, Debug, Default, Reflect)]
pub struct SpaceshipRootMarker;

/// World-space anchor of a ship's live structure: the computed center of
/// mass, which avian keeps in body-local space, lifted with rotation +
/// translation only. Not `transform_point`: avian ignores render scale, so
/// scaling the local COM would move the anchor off the physical pivot (task
/// 20260709-140620). Falls back to the root translation when no COM exists
/// (marker-only roots in tests).
///
/// The root ORIGIN is just the build spot of the ship's first sections and
/// stops meaning anything once those die - a wreck spins about its shifted
/// COM while the origin floats in empty space (task 20260709-150711). Aim
/// targets, lock-cone origins and camera anchors should all use this anchor
/// instead of the root translation.
pub fn live_structure_anchor(
    transform: &Transform,
    center_of_mass: Option<&ComputedCenterOfMass>,
) -> Vec3 {
    match center_of_mass {
        Some(com) => transform.rotation * com.0 + transform.translation,
        None => transform.translation,
    }
}

/// The pose of `descendant` in `root`'s local frame, composed from the local
/// mount `Transform`s along the `ChildOf` chain (render scale deliberately
/// ignored, matching the raw-pose convention of the flight layer). `None`
/// when the walk leaves the tree before reaching `root`.
///
/// This is the raw-clock spawn pattern shared by the weapon sections: a
/// FixedUpdate spawner composes the mount chain onto the root's avian
/// `Position`/`Rotation` instead of sampling `GlobalTransform`, which inside
/// FixedUpdate still holds the previous frame's EASED render pose (task
/// 20260710-231930 for turrets, 20260711-114640 for torpedoes).
pub(crate) fn local_pose_in_root(
    descendant: Entity,
    root: Entity,
    q_chain: &Query<(&Transform, &ChildOf)>,
) -> Option<(Vec3, Quat)> {
    let mut position = Vec3::ZERO;
    let mut rotation = Quat::IDENTITY;
    let mut entity = descendant;
    while entity != root {
        let (transform, &ChildOf(parent)) = q_chain.get(entity).ok()?;
        position = transform.translation + transform.rotation * position;
        rotation = transform.rotation * rotation;
        entity = parent;
    }
    Some((position, rotation))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_structure_anchor_lifts_the_local_com() {
        // Rotation + translation only: a 90 degree yaw turns local +X into
        // world -Z. A render scale must NOT stretch the offset (avian
        // ignores scale), which is why the helper never uses transform_point.
        let mut transform = Transform::from_translation(Vec3::new(10.0, 0.0, 0.0))
            .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2));
        transform.scale = Vec3::splat(3.0);
        let com = ComputedCenterOfMass(Vec3::new(2.0, 0.0, 0.0));

        let anchor = live_structure_anchor(&transform, Some(&com));

        assert!(
            (anchor - Vec3::new(10.0, 0.0, -2.0)).length() < 1e-5,
            "{anchor}"
        );
    }

    #[test]
    fn live_structure_anchor_falls_back_to_the_translation() {
        let transform = Transform::from_translation(Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(
            live_structure_anchor(&transform, None),
            Vec3::new(1.0, 2.0, 3.0)
        );
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpaceshipSectionSystems;

/// A plugin that adds all the spaceship sections and their related systems.
#[derive(Default, Clone, Debug)]
pub struct SpaceshipSectionPlugin {
    pub render: bool,
}

impl Plugin for SpaceshipSectionPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<ammo::SectionAmmo>();
        app.add_plugins((
            hull_section::HullSectionPlugin {
                render: self.render,
            },
            thruster_section::ThrusterSectionPlugin {
                render: self.render,
            },
            turret_section::TurretSectionPlugin {
                render: self.render,
            },
            controller_section::ControllerSectionPlugin {
                render: self.render,
            },
            torpedo_section::TorpedoSectionPlugin {
                render: self.render,
            },
        ));
    }
}
