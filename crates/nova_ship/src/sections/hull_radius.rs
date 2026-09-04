//! How big a hull is: one derived number, published on the ship root, that
//! both the attitude ceiling and the flight layer's arrival rule read.
//!
//! Engine units: the arm is measured off the sections' avian colliders, so
//! [`HullRadius`] is world units (10 m), like every other radius the flight
//! layer compares against an avian `Position`.

use avian3d::prelude::ComputedCenterOfMass;
use bevy::{ecs::entity::EntityHashMap, prelude::*};
use nova_gameplay::prelude::{SectionInactiveMarker, SectionMarker};

use crate::prelude::{structural_arm, SectionCollider};

/// The `HullRadius` component.
pub mod prelude {
    pub use super::HullRadius;
}

/// A hull's own outer reach, world units: the distance from its live centre of
/// mass to the outer FACE of its furthest live section.
///
/// Derived every tick, never authored. Lose sections and it shrinks, which is
/// what lets a damaged hull turn sharper than it did intact and park closer
/// than it did whole.
///
/// Two readers, one number: the attitude envelope divides the global load limit
/// by it for the structural turn ceiling, and the autopilot adds it to the
/// arrival so a leg parks the hull's FACE at the authored margin instead of
/// putting its origin there. It is also the size a ship publishes as a GOTO
/// TARGET - deliberately not [`BodyRadius`](crate::prelude::BodyRadius), which
/// additionally means "a solid body patrol legs steer around", and a hull is
/// not something the AI detours past.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub struct HullRadius(pub f32);

/// Publish every hull's [`HullRadius`] from its live sections.
///
/// ONE pass over every live section rather than one pass per hull: the arm
/// needs each section's offset from its own root's centre of mass, and
/// re-filtering the section query per hull is quadratic in a busy scene.
///
/// A root with no live sections left is not written at all: a wreck has no
/// arrival to plan and no attitude loop to feed, and zeroing it would hand
/// the envelope an infinite structural ceiling on the tick a hull dies.
pub(crate) fn publish_hull_radius(
    // Reused across ticks: this runs for every hull on every fixed tick and
    // must not allocate per ship per tick.
    mut arms: Local<EntityHashMap<f32>>,
    mut commands: Commands,
    mut q_root: Query<(&ComputedCenterOfMass, Option<&mut HullRadius>)>,
    q_section: Query<
        (&Transform, Option<&SectionCollider>, &ChildOf),
        (With<SectionMarker>, Without<SectionInactiveMarker>),
    >,
) {
    arms.clear();
    for (transform, collider, &ChildOf(root)) in &q_section {
        let Ok((center_of_mass, _)) = q_root.get(root) else {
            continue;
        };
        let half_extents = collider.copied().unwrap_or_default().aabb_half_extents();
        let arm = structural_arm(
            center_of_mass.0,
            [(transform.translation, transform.rotation, half_extents)],
        );
        let entry = arms.entry(root).or_insert(0.0);
        *entry = entry.max(arm);
    }

    for (&root, &arm) in arms.iter() {
        match q_root.get_mut(root) {
            Ok((_, Some(mut radius))) => {
                radius.set_if_neq(HullRadius(arm));
            }
            Ok((_, None)) => {
                commands.entity(root).try_insert(HullRadius(arm));
            }
            Err(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;
    use nova_gameplay::prelude::SpaceshipRootMarker;

    use super::*;

    /// Unit sections along z, with the centre of mass at the origin.
    fn hull(world: &mut World, section_offsets: &[f32]) -> Entity {
        let ship = world
            .spawn((SpaceshipRootMarker, ComputedCenterOfMass(Vec3::ZERO)))
            .id();
        for &z in section_offsets {
            world.spawn((
                ChildOf(ship),
                SectionMarker,
                Transform::from_xyz(0.0, 0.0, z),
                SectionCollider::Cuboid {
                    size: Vec3::splat(1.0),
                },
            ));
        }
        ship
    }

    #[test]
    fn a_hull_publishes_the_face_of_its_furthest_section() {
        let mut world = World::new();
        // Sections at -1, 0 and +1: the furthest FACE is 1.5 u out, not the
        // section centre (1.0) and not its corner (~1.87).
        let ship = hull(&mut world, &[-1.0, 0.0, 1.0]);

        world.run_system_once(publish_hull_radius).unwrap();

        let radius = world
            .get::<HullRadius>(ship)
            .expect("the pass publishes it");
        assert!((**radius - 1.5).abs() < 1e-4, "got {}", **radius);
    }

    #[test]
    fn losing_the_outer_sections_shrinks_the_published_radius() {
        let mut world = World::new();
        let ship = hull(&mut world, &[-1.0, 0.0, 1.0]);
        world.run_system_once(publish_hull_radius).unwrap();

        let outer: Vec<Entity> = world
            .query_filtered::<(Entity, &Transform), With<SectionMarker>>()
            .iter(&world)
            .filter(|(_, transform)| transform.translation.z.abs() > 0.5)
            .map(|(entity, _)| entity)
            .collect();
        for section in outer {
            world.entity_mut(section).insert(SectionInactiveMarker);
        }

        world.run_system_once(publish_hull_radius).unwrap();

        // Only the centre section is left, and a section sitting on the centre
        // of mass offers its own furthest face.
        let radius = world.get::<HullRadius>(ship).expect("still published");
        assert!((**radius - 0.5).abs() < 1e-4, "got {}", **radius);
    }
}
