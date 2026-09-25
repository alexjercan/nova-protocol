//! How big a body LOOKS: the union of the solid collider AABBs under an
//! entity, and the sphere that wraps that union.
//!
//! Not gameplay logic - this crate owns the walks because it is the lowest
//! crate every consumer already shares: the HUD's indicator sizing and target
//! inset (`nova_hud`) and the editor's framing, gizmo reach, plate placement
//! and ship row (`nova_editor`), which cannot reach the HUD. The sensor rule
//! below is the whole reason the walks live together: it was learned from a
//! shipped bug, and a copy of a walk elsewhere is a second chance to answer it
//! differently.

use avian3d::{
    parry::math::Pose3,
    prelude::{Collider, ColliderAabb, Sensor},
};
use bevy::prelude::*;

/// The subtree collider walks, and the bounding sphere read off the world one.
pub mod prelude {
    pub use super::{subtree_bounding_sphere, subtree_collider_aabb, subtree_local_collider_aabb};
}

/// Union the world-space [`ColliderAabb`]s of `entity` and all of its
/// descendants into a single box, or `None` when nothing in the subtree
/// carries one.
///
/// The whole subtree, because a body holds no collider of its own: an
/// asteroid's lives on its collider node, a ship's on its sections, an editor
/// node's on its views.
///
/// SENSOR colliders are excluded by the query shape rather than by this walk:
/// they are invisible trigger volumes, not apparent size. A locked beacon's
/// only collider is its authored 700 m trigger sphere, so the HUD reticle
/// wrapped the trigger instead of the 20 m orb - the same class of bug as the
/// salvage-crate bracket - and the editor framed the trigger, which puts the
/// beacon in the middle of an empty screen.
///
/// A sensor-only subtree yields `None` and the caller supplies its own floor:
/// the indicator's `min_px`, the inset's section half-extent, the node's own
/// origin.
pub fn subtree_collider_aabb(
    entity: Entity,
    q_children: &Query<&Children>,
    q_aabb: &Query<&ColliderAabb, Without<Sensor>>,
) -> Option<ColliderAabb> {
    let mut acc: Option<ColliderAabb> = None;
    let mut stack = vec![entity];
    while let Some(current) = stack.pop() {
        if let Ok(aabb) = q_aabb.get(current) {
            acc = Some(match acc {
                Some(existing) => existing.merged(*aabb),
                None => *aabb,
            });
        }
        if let Ok(children) = q_children.get(current) {
            stack.extend(children.iter());
        }
    }
    acc
}

/// The centre and radius of the sphere around [`subtree_collider_aabb`]'s
/// box, or `None` on the same empty subtree that walk returns `None` for.
///
/// Half the DIAGONAL, not the largest half-extent: the radius has to reach the
/// corners, or a reticle sized from it cuts through the hull it is drawn
/// around and a framed camera stands too close to hold the body on screen.
pub fn subtree_bounding_sphere(
    entity: Entity,
    q_children: &Query<&Children>,
    q_aabb: &Query<&ColliderAabb, Without<Sensor>>,
) -> Option<(Vec3, f32)> {
    subtree_collider_aabb(entity, q_children, q_aabb)
        .map(|aabb| (aabb.center(), aabb.size().length() * 0.5))
}

/// Union the solid [`Collider`] shapes of `entity` and all of its
/// descendants into one box about `entity`'s own origin, in its parent's
/// axes, or `None` when nothing in the subtree carries a solid collider.
///
/// This is the editor's unit-scale bound. It reads each shape and the local
/// [`Transform`] chain down to it, so it is right on every frame, not only on
/// frames that ran a physics step: avian writes [`ColliderAabb`] and the
/// scaled shape only in its step, and a frame that reads those against a pose
/// it has moved since sees a stale box. The entity's own rotation counts and
/// its translation does not, so the box moves with the entity.
///
/// Sensors are skipped as in [`subtree_collider_aabb`], but the walk still
/// descends through them.
///
/// # Panics
///
/// When a solid collider sits on or under an entity with no [`Transform`], or
/// with a scale other than [`Vec3::ONE`]. Editor ship, section and view
/// transforms carry no scale, so either is a bug in whatever spawned the
/// subtree, and a box read past it would lay the row out wrong without a sign.
/// A section's art scale sits on its render child, which has no collider under
/// it, so the walk never refuses that scale.
pub fn subtree_local_collider_aabb(
    entity: Entity,
    q_children: &Query<&Children>,
    q_transforms: &Query<&Transform>,
    q_colliders: &Query<&Collider, Without<Sensor>>,
) -> Option<ColliderAabb> {
    let mut acc: Option<ColliderAabb> = None;
    // `path` is the chain from `entity` to the node just popped; each stack
    // entry carries its depth in that chain. An unreadable pose is carried
    // down as the depth of the entity that broke it, and the panic message is
    // built only when a collider below needs the pose: a section's scaled
    // render art is walked every frame and never needs one.
    let mut path: Vec<Entity> = Vec::new();
    let mut stack: Vec<(Entity, usize, Result<Transform, usize>)> =
        vec![(entity, 0, Ok(Transform::IDENTITY))];
    while let Some((current, depth, above)) = stack.pop() {
        path.truncate(depth);
        path.push(current);
        let pose = above.and_then(|above| match q_transforms.get(current) {
            Ok(local) if local.scale != Vec3::ONE => Err(depth),
            Ok(local) if depth == 0 => Ok(Transform::from_rotation(local.rotation)),
            Ok(local) => Ok(above * *local),
            Err(_) => Err(depth),
        });
        if let Ok(collider) = q_colliders.get(current) {
            let pose = pose.unwrap_or_else(|broken| {
                let chain = &path[..=broken];
                match q_transforms.get(path[broken]) {
                    Ok(local) => panic!(
                        "collider walk: {chain:?} has scale {}, and the collider on {current:?} \
                         needs a unit-scale chain",
                        local.scale
                    ),
                    Err(_) => panic!(
                        "collider walk: {chain:?} has no Transform, and the collider on \
                         {current:?} needs one"
                    ),
                }
            });
            let shape = collider
                .shape()
                .compute_aabb(&Pose3::from_parts(pose.translation, pose.rotation));
            let aabb = ColliderAabb::from_min_max(shape.mins, shape.maxs);
            acc = Some(match acc {
                Some(existing) => existing.merged(aabb),
                None => aabb,
            });
        }
        if let Ok(children) = q_children.get(current) {
            stack.extend(children.iter().map(|child| (child, depth + 1, pose)));
        }
    }
    acc
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::SystemState;

    use super::*;

    #[test]
    fn a_subtree_walk_unions_the_child_collider_aabbs() {
        // A tracked body keeps its colliders on child entities (asteroid
        // collider node, ship sections). The parent itself has none, so the
        // union can only come from walking the children.
        let mut world = World::new();
        let child_a = world
            .spawn(ColliderAabb::from_min_max(
                Vec3::new(-1.0, -1.0, -1.0),
                Vec3::ZERO,
            ))
            .id();
        let child_b = world
            .spawn(ColliderAabb::from_min_max(
                Vec3::ZERO,
                Vec3::new(2.0, 3.0, 4.0),
            ))
            .id();
        let parent = world.spawn_empty().add_children(&[child_a, child_b]).id();

        let mut state: SystemState<(Query<&Children>, Query<&ColliderAabb, Without<Sensor>>)> =
            SystemState::new(&mut world);
        let (q_children, q_aabb) = state.get(&world).unwrap();

        let aabb = subtree_collider_aabb(parent, &q_children, &q_aabb)
            .expect("subtree has collider AABBs");
        assert_eq!(aabb.min, Vec3::new(-1.0, -1.0, -1.0));
        assert_eq!(aabb.max, Vec3::new(2.0, 3.0, 4.0));
    }

    #[test]
    fn a_subtree_without_colliders_has_no_bounds() {
        // A body whose colliders are not ready yet (spawn frame) yields no
        // AABB, so every consumer falls back to its own floor.
        let mut world = World::new();
        let entity = world.spawn_empty().id();

        let mut state: SystemState<(Query<&Children>, Query<&ColliderAabb, Without<Sensor>>)> =
            SystemState::new(&mut world);
        let (q_children, q_aabb) = state.get(&world).unwrap();

        assert!(subtree_collider_aabb(entity, &q_children, &q_aabb).is_none());
        assert!(subtree_bounding_sphere(entity, &q_children, &q_aabb).is_none());
    }

    /// Sensor colliders are not apparent size: a locked BEACON's only collider
    /// is its huge trigger sphere, and the consumer must fall back to its own
    /// floor instead of wrapping the trigger. A mixed subtree (ship hull plus
    /// an aim sensor) unions only the solid part. Delivery guard: the same
    /// entity WITH the Sensor removed does contribute - it is the query shape,
    /// not entity absence, doing the excluding.
    #[test]
    fn a_sensor_volume_is_never_part_of_a_bodys_size() {
        let mut world = World::new();
        // The beacon shape: root carries a big sensor AABB, the render child
        // has no collider at all.
        let beacon = world
            .spawn((
                ColliderAabb::from_min_max(Vec3::splat(-70.0), Vec3::splat(70.0)),
                Sensor,
            ))
            .id();
        // The mixed shape: solid hull child plus sensor child.
        let hull = world
            .spawn(ColliderAabb::from_min_max(Vec3::splat(-1.0), Vec3::ONE))
            .id();
        let trigger = world
            .spawn((
                ColliderAabb::from_min_max(Vec3::splat(-50.0), Vec3::splat(50.0)),
                Sensor,
            ))
            .id();
        let ship = world.spawn_empty().add_children(&[hull, trigger]).id();

        {
            let mut state: SystemState<(Query<&Children>, Query<&ColliderAabb, Without<Sensor>>)> =
                SystemState::new(&mut world);
            let (q_children, q_aabb) = state.get(&world).unwrap();

            assert!(
                subtree_collider_aabb(beacon, &q_children, &q_aabb).is_none(),
                "a sensor-only body has no apparent size"
            );
            let mixed = subtree_collider_aabb(ship, &q_children, &q_aabb)
                .expect("the solid hull still contributes");
            assert_eq!(mixed.min, Vec3::splat(-1.0));
            assert_eq!(mixed.max, Vec3::ONE);
        }

        // Delivery guard: strip the Sensor and the same AABB counts.
        world.entity_mut(beacon).remove::<Sensor>();
        let mut state: SystemState<(Query<&Children>, Query<&ColliderAabb, Without<Sensor>>)> =
            SystemState::new(&mut world);
        let (q_children, q_aabb) = state.get(&world).unwrap();
        assert!(
            subtree_collider_aabb(beacon, &q_children, &q_aabb).is_some(),
            "the query shape, not entity absence, excludes the sensor"
        );
    }

    /// The HUD's apparent radius and the editor's framing spread are the same
    /// number read from the same walk, so a beacon-shaped subtree - a big
    /// trigger volume wrapped around a small orb - cannot size one surface off
    /// the orb and the other off the trigger.
    #[test]
    fn the_bounding_sphere_of_a_beacon_measures_the_orb_not_the_trigger() {
        let mut world = World::new();
        // The orb the player sees: a 4 x 4 x 4 box about the origin.
        let orb = world
            .spawn(ColliderAabb::from_min_max(
                Vec3::splat(-2.0),
                Vec3::splat(2.0),
            ))
            .id();
        let beacon = world
            .spawn((
                ColliderAabb::from_min_max(Vec3::splat(-70.0), Vec3::splat(70.0)),
                Sensor,
            ))
            .add_children(&[orb])
            .id();

        let mut state: SystemState<(Query<&Children>, Query<&ColliderAabb, Without<Sensor>>)> =
            SystemState::new(&mut world);
        let (q_children, q_aabb) = state.get(&world).unwrap();

        let (centre, radius) =
            subtree_bounding_sphere(beacon, &q_children, &q_aabb).expect("the orb is solid");
        assert_eq!(centre, Vec3::ZERO);
        // Half the orb box's diagonal: sqrt(4^2 * 3) / 2.
        let orb_radius = (48.0_f32).sqrt() * 0.5;
        assert!(
            (radius - orb_radius).abs() < 1e-4,
            "the orb's half-diagonal {orb_radius}, not the trigger's: {radius}"
        );
    }

    /// A sensor on the way down is not size, but the solid hull under it is,
    /// and a section's scaled render art carries no collider, so its scale is
    /// never refused.
    #[test]
    fn the_local_walk_skips_a_sensor_but_measures_the_hull_under_it() {
        let mut world = World::new();
        let hull = world
            .spawn((Transform::default(), Collider::cuboid(2.0, 2.0, 2.0)))
            .id();
        let art = world.spawn(Transform::from_scale(Vec3::splat(10.0))).id();
        let trigger = world
            .spawn((
                Transform::from_xyz(5.0, 0.0, 0.0),
                Collider::sphere(50.0),
                Sensor,
            ))
            .add_children(&[hull, art])
            .id();
        let ship = world
            .spawn(Transform::from_xyz(100.0, 0.0, 0.0))
            .add_children(&[trigger])
            .id();

        let mut state: SystemState<(
            Query<&Children>,
            Query<&Transform>,
            Query<&Collider, Without<Sensor>>,
        )> = SystemState::new(&mut world);
        let (q_children, q_transforms, q_colliders) = state.get(&world).unwrap();

        let aabb = subtree_local_collider_aabb(ship, &q_children, &q_transforms, &q_colliders)
            .expect("the hull under the sensor is solid");
        assert_eq!(aabb.min, Vec3::new(4.0, -1.0, -1.0));
        assert_eq!(aabb.max, Vec3::new(6.0, 1.0, 1.0));
    }

    /// A subtree with no solid collider has no bounds, and the row stands the
    /// ship on its own origin. The editor's placement ghost keeps a stale
    /// `ColliderAabb` with no `Collider`, and a sensor is not size, so neither
    /// gives the ship a width.
    #[test]
    fn the_local_walk_of_a_ship_without_a_solid_collider_has_no_bounds() {
        let mut world = World::new();
        let ghost = world
            .spawn((
                Transform::from_xyz(-1.0, 0.0, 0.0),
                ColliderAabb::from_min_max(Vec3::splat(-1.0), Vec3::ONE),
            ))
            .id();
        let trigger = world
            .spawn((Transform::default(), Collider::sphere(50.0), Sensor))
            .id();
        let ship = world
            .spawn(Transform::from_xyz(100.0, 0.0, 0.0))
            .add_children(&[ghost, trigger])
            .id();

        let mut state: SystemState<(
            Query<&Children>,
            Query<&Transform>,
            Query<&Collider, Without<Sensor>>,
        )> = SystemState::new(&mut world);
        let (q_children, q_transforms, q_colliders) = state.get(&world).unwrap();

        assert!(
            subtree_local_collider_aabb(ship, &q_children, &q_transforms, &q_colliders).is_none()
        );
    }

    /// Rotations compose down the chain, and the entity's own rotation counts
    /// while its translation does not: the box is about the entity's origin,
    /// in its parent's axes.
    #[test]
    fn the_local_walk_composes_nested_rotations_about_the_entity_origin() {
        let quarter = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        let mut world = World::new();
        let view = world
            .spawn((
                Transform::from_xyz(0.0, 0.0, 5.0),
                Collider::cuboid(2.0, 4.0, 6.0),
            ))
            .id();
        let section = world
            .spawn(Transform::from_xyz(10.0, 0.0, 0.0).with_rotation(quarter))
            .add_children(&[view])
            .id();
        let ship = world
            .spawn(Transform::from_xyz(100.0, 0.0, 0.0).with_rotation(quarter))
            .add_children(&[section])
            .id();

        let mut state: SystemState<(
            Query<&Children>,
            Query<&Transform>,
            Query<&Collider, Without<Sensor>>,
        )> = SystemState::new(&mut world);
        let (q_children, q_transforms, q_colliders) = state.get(&world).unwrap();

        let aabb = subtree_local_collider_aabb(ship, &q_children, &q_transforms, &q_colliders)
            .expect("the view is solid");
        // The ship's quarter turn puts the section at -z 10, and the two
        // quarter turns together put the view 5 further along -z, half-turned.
        assert!(
            aabb.min.abs_diff_eq(Vec3::new(-1.0, -2.0, -18.0), 1e-4),
            "min {}",
            aabb.min
        );
        assert!(
            aabb.max.abs_diff_eq(Vec3::new(1.0, 2.0, -12.0), 1e-4),
            "max {}",
            aabb.max
        );
    }

    /// The walk has no scale in its sums, so a scaled link above a collider
    /// would measure a wrong box without a sign. It must refuse and name the
    /// chain down to the scaled entity, not a sibling branch walked before it.
    #[test]
    fn the_local_walk_refuses_a_scaled_link_above_a_collider() {
        let mut world = World::new();
        let view = world
            .spawn((Transform::default(), Collider::cuboid(2.0, 2.0, 2.0)))
            .id();
        let section = world
            .spawn(Transform::from_scale(Vec3::splat(2.0)))
            .add_children(&[view])
            .id();
        // The last child is walked first, so its deeper branch is on the path
        // until the walk steps back to the scaled section.
        let rim = world.spawn(Transform::default()).id();
        let plate = world.spawn(Transform::default()).add_children(&[rim]).id();
        let ship = world
            .spawn(Transform::default())
            .add_children(&[section, plate])
            .id();

        let mut state: SystemState<(
            Query<&Children>,
            Query<&Transform>,
            Query<&Collider, Without<Sensor>>,
        )> = SystemState::new(&mut world);
        let (q_children, q_transforms, q_colliders) = state.get(&world).unwrap();

        let refusal = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            subtree_local_collider_aabb(ship, &q_children, &q_transforms, &q_colliders)
        }))
        .expect_err("a scaled link above a collider is refused");
        let message = refusal.downcast::<String>().expect("a formatted message");
        assert!(
            message.contains(&format!("{:?} has scale", [ship, section])),
            "the refusal names the chain to the scaled entity: {message}"
        );
    }

    /// A link with no Transform leaves the collider below it with no pose to
    /// measure. The walk must refuse and name the chain down to that link.
    #[test]
    fn the_local_walk_refuses_a_missing_transform_above_a_collider() {
        let mut world = World::new();
        let view = world
            .spawn((Transform::default(), Collider::cuboid(2.0, 2.0, 2.0)))
            .id();
        let section = world.spawn_empty().add_children(&[view]).id();
        let ship = world
            .spawn(Transform::default())
            .add_children(&[section])
            .id();

        let mut state: SystemState<(
            Query<&Children>,
            Query<&Transform>,
            Query<&Collider, Without<Sensor>>,
        )> = SystemState::new(&mut world);
        let (q_children, q_transforms, q_colliders) = state.get(&world).unwrap();

        let refusal = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            subtree_local_collider_aabb(ship, &q_children, &q_transforms, &q_colliders)
        }))
        .expect_err("a missing Transform above a collider is refused");
        let message = refusal.downcast::<String>().expect("a formatted message");
        assert!(
            message.contains(&format!("{:?} has no Transform", [ship, section])),
            "the refusal names the chain to the bare entity: {message}"
        );
    }
}
