//! This module contains all the sections of a spaceship.

use avian3d::prelude::ComputedCenterOfMass;
use bevy::prelude::*;
use nova_gameplay::gravity::prelude::NovaGravitySystems;

pub mod ammo;
pub mod base_section;
pub mod controller_section;
pub mod damage_tint;
pub mod hull_section;
pub mod thruster_section;
pub mod torpedo_section;
pub mod turret_section;

/// Every section submodule's prelude, `live_structure_anchor`, `SpaceshipRootMarker`, and
/// `SpaceshipSectionPlugin` with `SpaceshipSectionSystems`.
pub mod prelude {
    pub use super::{
        ammo::prelude::*, base_section::prelude::*, controller_section::prelude::*,
        damage_tint::prelude::*, hull_section::prelude::*, live_structure_anchor,
        thruster_section::prelude::*, torpedo_section::prelude::*, turret_section::prelude::*,
        SpaceshipSectionPlugin, SpaceshipSectionSystems,
    };
}

/// World-space anchor of a ship's live structure: the computed center of
/// mass, which avian keeps in body-local space, lifted with rotation +
/// translation only. Not `transform_point`: avian ignores render scale, so
/// scaling the local COM would move the anchor off the physical pivot. Falls
/// back to the root translation when no COM exists (marker-only roots in
/// tests).
///
/// The root ORIGIN is just the build spot of the ship's first sections and
/// stops meaning anything once those die - a wreck spins about its shifted
/// COM while the origin floats in empty space. Aim targets, lock-cone origins
/// and camera anchors should all use this anchor instead of the root
/// translation.
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
/// FixedUpdate still holds the previous frame's EASED render pose.
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

/// System set holding every section's per-frame systems (aim, fire, reload,
/// thrust). Ordered between the input and hud sets by
/// [`SpaceshipSystems`](nova_gameplay::plugin::SpaceshipSystems), and after
/// [`NovaGravitySystems`] so a section acts on this tick's well forces.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpaceshipSectionSystems;

/// A plugin that adds all the spaceship sections and their related systems.
#[derive(Default, Clone, Debug)]
pub struct SpaceshipSectionPlugin {
    /// Whether the render-side section plugins (meshes, damage tint) are added.
    pub render: bool,
}

impl Plugin for SpaceshipSectionPlugin {
    fn build(&self, app: &mut App) {
        // The ship declares its own edge against the layers below it. This used
        // to be `NovaGravityPlugin`'s `.before(SpaceshipSectionSystems)`, which
        // made gravity - the lower layer - name the ship.
        app.configure_sets(
            FixedUpdate,
            SpaceshipSectionSystems.after(NovaGravitySystems),
        );

        app.register_type::<ammo::SectionAmmo>();
        app.register_type::<ammo::SectionReload>();
        // NOTE: reload is add-only against the consume in
        // `shoot_spawn_projectile`, so it needs no ordering versus the fire
        // systems - only the same fixed clock.
        app.add_systems(FixedUpdate, ammo::tick_section_reload);
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

        // Diegetic hull integrity: grade player-ship section materials by
        // health. Only meaningful when sections actually render.
        if self.render {
            app.add_plugins(damage_tint::SectionDamageTintPlugin);
        }
    }
}
