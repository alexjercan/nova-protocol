//! How big a hull is: two derived numbers, published together on the ship root
//! from one pass over its live sections.
//!
//! [`HullRadius`] is the STRUCTURAL arm the attitude ceiling and the flight
//! layer's arrival rule read. [`HullEnvelopeRadius`] is the CONTAINMENT radius
//! the HUD's shells stand outside of. They answer different questions and are
//! published side by side so they can never disagree about which sections are
//! live.
//!
//! Engine units: both are measured off the sections' avian colliders, so both
//! are world units (10 m), like every other radius the flight layer compares
//! against an avian `Position`.

use avian3d::prelude::ComputedCenterOfMass;
use bevy::{ecs::entity::EntityHashMap, prelude::*};
use nova_gameplay::prelude::{SectionInactiveMarker, SectionMarker};

use crate::prelude::{structural_arm, SectionCollider};

/// The `HullRadius` and `HullEnvelopeRadius` components.
pub mod prelude {
    pub use super::{HullEnvelopeRadius, HullRadius};
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

/// A hull's CONTAINMENT radius, world units: the smallest radius about the live
/// centre of mass that holds every live section collider whole.
///
/// Derived every tick beside [`HullRadius`], never authored, and deliberately
/// NOT the same number. The arm measures to a section's outer FACE along its
/// own radial ray, because that is the lever the attitude loop turns about and
/// the face the autopilot parks. This measures to the furthest COLLIDER POINT,
/// because a shell drawn at the arm would still have corners of the hull
/// standing through it.
///
/// The HUD's velocity and gravity shells are the readers: each stands a stated
/// clearance outside this, so the promise "the hull is inside the sphere" is
/// kept by a hull of any size.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub struct HullEnvelopeRadius(pub f32);

/// Publish every hull's [`HullRadius`] and [`HullEnvelopeRadius`] from its live
/// sections.
///
/// ONE pass over every live section rather than one pass per hull: both numbers
/// need each section's offset from its own root's centre of mass, and
/// re-filtering the section query per hull is quadratic in a busy scene. One
/// pass is also what keeps the two contracts agreeing on which sections are
/// live - a shell sized from a different section set than the arm is a shell
/// that can be wrong for a frame after a hit.
///
/// A root with no live sections left is not written at all: a wreck has no
/// arrival to plan and no attitude loop to feed, and zeroing it would hand
/// the envelope an infinite structural ceiling on the tick a hull dies.
pub(crate) fn publish_hull_radii(
    // Reused across ticks: this runs for every hull on every fixed tick and
    // must not allocate per ship per tick.
    mut radii: Local<EntityHashMap<(f32, f32)>>,
    mut commands: Commands,
    mut q_root: Query<(
        &ComputedCenterOfMass,
        Option<&mut HullRadius>,
        Option<&mut HullEnvelopeRadius>,
    )>,
    q_section: Query<
        (&Transform, Option<&SectionCollider>, &ChildOf),
        (With<SectionMarker>, Without<SectionInactiveMarker>),
    >,
) {
    radii.clear();
    for (transform, collider, &ChildOf(root)) in &q_section {
        let Ok((center_of_mass, _, _)) = q_root.get(root) else {
            continue;
        };
        let collider = collider.copied().unwrap_or_default();
        let arm = structural_arm(
            center_of_mass.0,
            [(
                transform.translation,
                transform.rotation,
                collider.aabb_half_extents(),
            )],
        );
        let envelope =
            collider.furthest_distance(transform.translation, transform.rotation, center_of_mass.0);
        let entry = radii.entry(root).or_insert((0.0, 0.0));
        entry.0 = entry.0.max(arm);
        entry.1 = entry.1.max(envelope);
    }

    for (&root, &(arm, envelope)) in radii.iter() {
        let Ok((_, radius, envelope_radius)) = q_root.get_mut(root) else {
            continue;
        };
        match radius {
            Some(mut radius) => {
                radius.set_if_neq(HullRadius(arm));
            }
            None => {
                commands.entity(root).try_insert(HullRadius(arm));
            }
        }
        match envelope_radius {
            Some(mut envelope_radius) => {
                envelope_radius.set_if_neq(HullEnvelopeRadius(envelope));
            }
            None => {
                commands
                    .entity(root)
                    .try_insert(HullEnvelopeRadius(envelope));
            }
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
        hull_about(world, section_offsets, Vec3::ZERO)
    }

    /// The same line of unit sections, with the centre of mass placed by hand -
    /// what an asymmetric build or a lost section leaves behind.
    fn hull_about(world: &mut World, section_offsets: &[f32], center_of_mass: Vec3) -> Entity {
        let ship = world
            .spawn((SpaceshipRootMarker, ComputedCenterOfMass(center_of_mass)))
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

        world.run_system_once(publish_hull_radii).unwrap();

        let radius = world
            .get::<HullRadius>(ship)
            .expect("the pass publishes it");
        assert!((**radius - 1.5).abs() < 1e-4, "got {}", **radius);
    }

    #[test]
    fn losing_the_outer_sections_shrinks_the_published_radius() {
        let mut world = World::new();
        let ship = hull(&mut world, &[-1.0, 0.0, 1.0]);
        world.run_system_once(publish_hull_radii).unwrap();

        let outer: Vec<Entity> = world
            .query_filtered::<(Entity, &Transform), With<SectionMarker>>()
            .iter(&world)
            .filter(|(_, transform)| transform.translation.z.abs() > 0.5)
            .map(|(entity, _)| entity)
            .collect();
        for section in outer {
            world.entity_mut(section).insert(SectionInactiveMarker);
        }

        world.run_system_once(publish_hull_radii).unwrap();

        // Only the centre section is left, and a section sitting on the centre
        // of mass offers its own furthest face.
        let radius = world.get::<HullRadius>(ship).expect("still published");
        assert!((**radius - 0.5).abs() < 1e-4, "got {}", **radius);
    }

    /// The envelope holds every live collider WHOLE, so it reaches the furthest
    /// corner where the arm stops at the face. Both come out of the one pass.
    #[test]
    fn the_envelope_contains_every_live_collider_corner() {
        let mut world = World::new();
        let ship = hull(&mut world, &[-1.0, 0.0, 1.0]);

        world.run_system_once(publish_hull_radii).unwrap();

        // The far corner of the unit cube centred at z = 1: (0.5, 0.5, 1.5).
        let expected = Vec3::new(0.5, 0.5, 1.5).length();
        let envelope = world
            .get::<HullEnvelopeRadius>(ship)
            .expect("the pass publishes it beside the arm");
        assert!((**envelope - expected).abs() < 1e-4, "got {}", **envelope);
        // And it is strictly outside the arm, which stops at the face.
        let radius = world.get::<HullRadius>(ship).expect("still published");
        assert!(**envelope > **radius, "{} vs {}", **envelope, **radius);
    }

    /// The envelope is measured about the LIVE centre of mass, not the root
    /// origin: an asymmetric build or a lost section moves the COM, and a shell
    /// centred anywhere else would leave the far end of the hull outside it.
    #[test]
    fn the_envelope_follows_a_moved_center_of_mass() {
        let mut world = World::new();
        let ship = hull_about(&mut world, &[-1.0, 0.0, 1.0], Vec3::new(0.0, 0.0, 1.0));

        world.run_system_once(publish_hull_radii).unwrap();

        // Measured from z = 1, the far corner is the one on the section at
        // z = -1: (0.5, 0.5, 2.5).
        let expected = Vec3::new(0.5, 0.5, 2.5).length();
        let envelope = world.get::<HullEnvelopeRadius>(ship).expect("published");
        assert!((**envelope - expected).abs() < 1e-4, "got {}", **envelope);
    }

    /// Severing the section that decided the envelope shrinks it, and leaves
    /// the arm exactly where the existing contract puts it.
    #[test]
    fn losing_the_outer_section_shrinks_the_envelope_and_not_the_arm() {
        let mut world = World::new();
        let ship = hull(&mut world, &[-1.0, 0.0, 1.0]);
        world.run_system_once(publish_hull_radii).unwrap();
        let intact_envelope = **world.get::<HullEnvelopeRadius>(ship).expect("published");

        let outer: Vec<Entity> = world
            .query_filtered::<(Entity, &Transform), With<SectionMarker>>()
            .iter(&world)
            .filter(|(_, transform)| transform.translation.z.abs() > 0.5)
            .map(|(entity, _)| entity)
            .collect();
        for section in outer {
            world.entity_mut(section).insert(SectionInactiveMarker);
        }

        world.run_system_once(publish_hull_radii).unwrap();

        // One unit cube on the centre of mass: its own corner, 0.5 on each
        // axis.
        let expected = Vec3::splat(0.5).length();
        let envelope = **world.get::<HullEnvelopeRadius>(ship).expect("published");
        assert!(
            envelope < intact_envelope,
            "{envelope} vs {intact_envelope}"
        );
        assert!((envelope - expected).abs() < 1e-4, "got {envelope}");
        // The arm is the number the existing test pins, unchanged by the
        // envelope riding along with it.
        let radius = world.get::<HullRadius>(ship).expect("still published");
        assert!((**radius - 0.5).abs() < 1e-4, "got {}", **radius);
    }
}
