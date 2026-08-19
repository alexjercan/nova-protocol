//! This module contains all the sections of a spaceship.

use avian3d::prelude::ComputedCenterOfMass;
use bevy::prelude::*;
use nova_gameplay::gravity::prelude::NovaGravitySystems;

pub mod ammo;
pub mod base_section;
pub mod catalog_ids;
pub mod clearance;
pub mod controller_section;
pub mod damage_cracks;
pub mod damage_effects;
pub mod damage_plume;
pub mod damage_sparks;
pub mod fixture;
pub mod hull_section;
pub mod integrity;
pub mod link_points;
pub mod shell_shape;
pub mod shell_skin;
pub mod skin_decor;
pub mod skin_reading;
pub mod skin_report;
pub mod skin_style;
pub mod thruster_section;
pub mod torpedo_section;
pub mod turret_section;

/// Every section submodule's prelude, `live_structure_anchor`, `SpaceshipRootMarker`, and
/// `SpaceshipSectionPlugin` with `SpaceshipSectionSystems`.
pub mod prelude {
    pub use super::{
        ammo::prelude::*, base_section::prelude::*, catalog_ids::prelude::*, clearance::prelude::*,
        controller_section::prelude::*, damage_cracks::prelude::*, damage_effects::prelude::*,
        damage_plume::prelude::*, damage_sparks::prelude::*, fixture::prelude::*,
        hull_section::prelude::*, integrity::prelude::*, link_points::prelude::*,
        live_structure_anchor, shell_shape::prelude::*, shell_skin::prelude::*,
        skin_decor::prelude::*, skin_reading::prelude::*, skin_report::prelude::*,
        skin_style::prelude::*, thruster_section::prelude::*, torpedo_section::prelude::*,
        turret_section::prelude::*, SpaceshipSectionPlugin, SpaceshipSectionSystems,
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

/// A nose-forward projectile body: a cylinder capped by a cone, centered on its
/// own midpoint, nose along +Y.
///
/// +Y because that is the axis bevy's primitives are built on; every caller
/// rotates it onto the axis its own frame travels along, and there is no axis
/// this crate can call "forward" for both a bullet (whose transform IS the
/// flight direction) and a torpedo body (mounted on a section with an authored
/// rotation of its own).
///
/// ONE merged mesh, not two child entities: the turret path builds up to 100
/// rounds/s per muzzle and one render child per round is already the budget.
pub(crate) fn nose_cone_mesh(radius: f32, body_length: f32, nose_length: f32) -> Mesh {
    // Body and nose sit either side of the midpoint of the whole shape, so it
    // spins about its centre like the cuboid primitives it replaces.
    let mut mesh = Mesh::from(Cylinder::new(radius, body_length))
        .translated_by(Vec3::Y * (-nose_length / 2.0));
    let nose =
        Mesh::from(Cone::new(radius, nose_length)).translated_by(Vec3::Y * (body_length / 2.0));
    mesh.merge(&nose)
        .expect("cylinder and cone primitives share a vertex layout");
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shared body is a nose-forward solid of revolution on +Y, and it is
    /// exactly as long as its two parts - both things every caller's own
    /// orientation math is derived from.
    #[test]
    fn nose_cone_mesh_points_up_and_spans_body_plus_nose() {
        use bevy::mesh::VertexAttributeValues;

        let (radius, body, nose) = (0.02, 0.1, 0.06);
        let mesh = nose_cone_mesh(radius, body, nose);
        let positions: Vec<Vec3> = match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
            Some(VertexAttributeValues::Float32x3(p)) => {
                p.iter().copied().map(Vec3::from_array).collect()
            }
            other => panic!("unexpected positions: {other:?}"),
        };
        let min = positions
            .iter()
            .copied()
            .reduce(Vec3::min)
            .expect("vertices");
        let max = positions
            .iter()
            .copied()
            .reduce(Vec3::max)
            .expect("vertices");

        assert!(
            (max.y - min.y - (body + nose)).abs() < 1e-5,
            "{min:?} {max:?}"
        );
        assert!((max.x - radius).abs() < 2e-3, "{max:?}");

        // The single leading vertex is the cone tip, on the axis.
        let tip = positions
            .iter()
            .copied()
            .max_by(|a, b| a.y.total_cmp(&b.y))
            .expect("a vertex");
        assert!(
            tip.x.abs() < 1e-5 && tip.z.abs() < 1e-5,
            "the leading vertex is the cone tip on the axis, not a body rim: {tip:?}"
        );
    }

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
        app.add_plugins(integrity::ShipIntegrityPlugin);
        // A successful shot resets reload progress. Run the reload pass after
        // every section fire system so the shot wins an exact completion tick.
        app.add_systems(
            FixedUpdate,
            ammo::tick_section_reload.after(SpaceshipSectionSystems),
        );
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
            // Not one of the kinds above: cladding is a FIXTURE derived from
            // whatever those five build, so it has no prototype and no palette
            // entry to sit next to them in.
            shell_skin::ShipSkinPlugin {
                render: self.render,
            },
        ));

        // WHICH damage looks a section wears, authored in its content. Split
        // from the effects themselves: the list is data a headless server loads
        // and saves, the components it names are read only by render systems.
        app.add_plugins(damage_effects::DamageEffectsPlugin {
            render: self.render,
        });

        // The damage effects themselves: what a section's own damage LOOKS
        // like. Only meaningful when sections actually render.
        if self.render {
            app.add_plugins(damage_cracks::SectionCracksPlugin);
            app.add_plugins(damage_sparks::DamageSparksPlugin);
            app.add_plugins(damage_plume::DamagePlumePlugin);
        }
    }
}
