//! What a hull can still DO, published every tick from what is live on it:
//! one ship-level reading of linear and attitude authority, for the systems
//! that DECIDE where to fly.
//!
//! Deciders, not the autopilot. The autopilot plans each burn against the
//! particular cluster it would fire, in the direction it needs, with that
//! cluster's own rotation distance - a finer question than "what has this
//! hull got", and one no single number answers. This answers the coarse
//! question, for a caller choosing a goal rather than flying one.
//!
//! Engine units: avian mass and the thruster's per-tick impulse in, world
//! units per second squared and radians per second out.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_gameplay::prelude::{
    ControllerSectionMarker, SectionInactiveMarker, SpaceshipRootMarker, ThrusterSectionMarker,
    FORWARD_ALIGNMENT_COS,
};

use super::{guidance::ship_turn_rate, state::FlightSettings, thrusters::cluster_thrusters};
use crate::prelude::{PDController, ThrusterSectionMagnitude};

/// The `FlightAuthority` component.
pub mod prelude {
    pub use super::FlightAuthority;
}

/// What one hull's live drives and computers are worth, on the ship root.
///
/// Derived every tick, never authored, like
/// [`HullRadius`](crate::prelude::HullRadius): lose engines and the
/// acceleration falls, lose computers and the turn rate does. A decision made
/// on this therefore changes with the damage, instead of holding a figure the
/// hull can no longer fly.
///
/// Zero in both fields is the honest reading for a hulk, and for a root avian
/// has not weighed yet. A caller that would divide by either must say what it
/// does with nothing.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[reflect(Component)]
pub struct FlightAuthority {
    /// The acceleration (u/s2) the hull's strongest drive cluster delivers
    /// against its live mass.
    ///
    /// One figure for the burn and for the brake. A plan turns the cluster
    /// onto whichever direction it needs, so a hull whose retros are weaker
    /// than its mains still reads its mains here - and a decision made on it
    /// has assumed the flip, which is what an arrival rule's lead time pays
    /// for.
    pub linear_acceleration: f32,
    /// The rate (rad/s) the hull can hold a turn at: the slew rate its live
    /// computers' authority commands, never above the rate its structure
    /// takes sustained.
    ///
    /// The second bound is the one that separates a picket from a capital. A
    /// big hull can be given computers enough to command a fast slew and
    /// still not be able to CARRY the turn - the centripetal load at the tip
    /// of a long arm spends the structural budget on its own.
    pub turn_rate: f32,
    /// The seconds of turn the hull trails a command that is turning
    /// ([`PDController::tracking_lag`]), taking the slowest live computer's
    /// when several disagree.
    ///
    /// The turn rate says how fast a hull comes around; this says how late.
    /// A plan that budgets a flip needs both, and the pair of them is why
    /// this component exists rather than a bare acceleration.
    pub tracking_lag: f32,
}

/// Publish every ship's [`FlightAuthority`] from its live sections.
///
/// ONE pass over the live drives and one over the live computers, each sorted
/// into per-hull runs, for the reason `publish_ship_signatures` gives:
/// re-filtering a section query per hull is quadratic in a busy scene.
///
/// The drive directions are read in the HULL's frame rather than the world's.
/// Only the summed magnitude of the strongest group is published, and the
/// root's rotation is a common factor over every engine on it, so it changes
/// neither the grouping nor the sum - and reading a rotation here would pin
/// the pass to the physics frame for a number that has no frame.
pub(super) fn publish_flight_authority(
    // Reused across ticks: this runs for every hull on every fixed tick and
    // must not allocate per ship per tick.
    mut drives: Local<Vec<(Entity, Vec3, f32)>>,
    mut computers: Local<Vec<(Entity, f32, [f32; 2])>>,
    mut engines: Local<Vec<(Vec3, f32)>>,
    mut commands: Commands,
    time: Res<Time>,
    settings: Res<FlightSettings>,
    mut q_root: Query<
        (Entity, &ComputedMass, Option<&mut FlightAuthority>),
        With<SpaceshipRootMarker>,
    >,
    q_thruster: Query<
        (&Transform, &ThrusterSectionMagnitude, &ChildOf),
        (
            With<ThrusterSectionMarker>,
            Without<SectionInactiveMarker>,
            Without<SpaceshipRootMarker>,
        ),
    >,
    q_computer: Query<
        (&PDController, &ChildOf),
        (
            With<ControllerSectionMarker>,
            Without<SectionInactiveMarker>,
        ),
    >,
) {
    let dt = time.delta_secs();

    drives.clear();
    for (transform, magnitude, &ChildOf(root)) in &q_thruster {
        // A section's local rotation is the way its engine points; -Z is a
        // thruster's own thrust axis.
        let direction = transform.rotation.mul_vec3(Vec3::NEG_Z).normalize_or_zero();
        drives.push((root, direction, (**magnitude).max(0.0)));
    }
    drives.sort_unstable_by_key(|(root, _, _)| *root);

    computers.clear();
    for (pd, &ChildOf(root)) in &q_computer {
        computers.push((
            root,
            pd.max_angular_acceleration.max(0.0),
            [pd.sustained_angular_speed.max(0.0), pd.tracking_lag()],
        ));
    }
    computers.sort_unstable_by_key(|(root, _, _)| *root);

    for (root, mass, published) in &mut q_root {
        engines.clear();
        engines.extend(
            run_of(drives.as_slice(), root)
                .iter()
                .map(|&(_, direction, magnitude)| (direction, magnitude)),
        );
        // The strongest group is the main drive, whatever the hull calls it:
        // the planner would turn that one onto any burn worth the rotation.
        let authority = cluster_thrusters(&engines, FORWARD_ALIGNMENT_COS)
            .iter()
            .map(|group| group.authority)
            .fold(0.0f32, f32::max);
        // The magnitude is a per-tick IMPULSE (see `ThrusterSectionMagnitude`),
        // so the acceleration it stands for depends on the tick it lands in.
        let linear_acceleration = if dt > 0.0 && mass.value() > 0.0 {
            (authority / mass.value()) / dt
        } else {
            0.0
        };

        let stack = run_of(computers.as_slice(), root);
        let sustained = stack
            .iter()
            .map(|&(_, _, [sustained, _])| sustained)
            .fold(f32::INFINITY, f32::min);
        let turn_rate = ship_turn_rate(stack.iter().map(|&(_, alpha, _)| alpha), &settings)
            .map_or(0.0, |rate| rate.min(sustained));
        let tracking_lag = stack
            .iter()
            .map(|&(_, _, [_, lag])| lag)
            .fold(0.0f32, f32::max);

        let next = FlightAuthority {
            linear_acceleration,
            turn_rate,
            tracking_lag,
        };
        match published {
            Some(mut published) => {
                published.set_if_neq(next);
            }
            None => {
                commands.entity(root).try_insert(next);
            }
        }
    }
}

/// The contiguous run of `sorted` belonging to `root`, empty when the hull
/// carries none of that section kind.
fn run_of<A, B>(sorted: &[(Entity, A, B)], root: Entity) -> &[(Entity, A, B)] {
    let start = sorted.partition_point(|(other, _, _)| *other < root);
    let end = start + sorted[start..].partition_point(|(other, _, _)| *other == root);
    &sorted[start..end]
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    /// A hull with `mass` kilograms on it, a drive of `magnitude` per-tick
    /// impulse aft, and a computer of `alpha` rad/s2.
    fn authority_world(mass: f32, magnitude: f32, alpha: f32, sustained: f32) -> (World, Entity) {
        let mut world = World::new();
        world.init_resource::<FlightSettings>();
        world.init_resource::<Time>();
        let mut time = world.resource_mut::<Time>();
        time.advance_by(core::time::Duration::from_secs_f32(1.0 / 60.0));

        let ship = world
            .spawn((
                SpaceshipRootMarker,
                ComputedMass::new(mass),
                Transform::default(),
            ))
            .id();
        world.spawn((
            ChildOf(ship),
            ThrusterSectionMarker,
            ThrusterSectionMagnitude(magnitude),
            Transform::default(),
        ));
        world.spawn((
            ChildOf(ship),
            ControllerSectionMarker,
            PDController {
                frequency: 4.0,
                damping_ratio: 4.0,
                max_angular_acceleration: alpha,
                sustained_angular_speed: sustained,
            },
        ));
        (world, ship)
    }

    fn published(world: &World, ship: Entity) -> FlightAuthority {
        *world
            .entity(ship)
            .get::<FlightAuthority>()
            .expect("every ship root publishes its authority")
    }

    #[test]
    fn a_hull_publishes_what_its_drive_does_to_its_own_mass() {
        let (mut world, ship) = authority_world(4.0, 2.0, 1.0, f32::INFINITY);
        world.run_system_once(publish_flight_authority).unwrap();

        // 2.0 of per-tick impulse on 4 kg is 0.5 u/s of delta-v a tick, which
        // at 60 Hz is 30 u/s2.
        let authority = published(&world, ship);
        assert!(
            (authority.linear_acceleration - 30.0).abs() < 1e-3,
            "got {}",
            authority.linear_acceleration
        );
    }

    #[test]
    fn the_same_drive_moves_a_heavier_hull_slower() {
        let (mut world_light, light) = authority_world(4.0, 2.0, 1.0, f32::INFINITY);
        let (mut world_heavy, heavy) = authority_world(40.0, 2.0, 1.0, f32::INFINITY);
        world_light
            .run_system_once(publish_flight_authority)
            .unwrap();
        world_heavy
            .run_system_once(publish_flight_authority)
            .unwrap();

        assert!(
            (published(&world_light, light).linear_acceleration
                - 10.0 * published(&world_heavy, heavy).linear_acceleration)
                .abs()
                < 1e-2,
            "ten times the mass is a tenth of the acceleration"
        );
    }

    #[test]
    fn only_the_strongest_cluster_counts_not_the_whole_drive_bill() {
        // A retro is thrust the hull HAS and never adds to the burn it is
        // planning: the two point opposite ways.
        let (mut world, ship) = authority_world(4.0, 2.0, 1.0, f32::INFINITY);
        world.spawn((
            ChildOf(ship),
            ThrusterSectionMarker,
            ThrusterSectionMagnitude(2.0),
            Transform::from_rotation(Quat::from_rotation_y(core::f32::consts::PI)),
        ));
        world.run_system_once(publish_flight_authority).unwrap();

        assert!(
            (published(&world, ship).linear_acceleration - 30.0).abs() < 1e-3,
            "a retro pointing the other way is not more main drive"
        );
    }

    #[test]
    fn a_dead_drive_and_a_dead_computer_leave_nothing_to_decide_with() {
        let (mut world, ship) = authority_world(4.0, 2.0, 1.0, f32::INFINITY);
        let sections: Vec<Entity> = world
            .query_filtered::<Entity, With<ChildOf>>()
            .iter(&world)
            .collect();
        for section in sections {
            world.entity_mut(section).insert(SectionInactiveMarker);
        }
        world.run_system_once(publish_flight_authority).unwrap();

        assert_eq!(
            published(&world, ship),
            FlightAuthority::default(),
            "a hulk can neither burn nor turn"
        );
    }

    #[test]
    fn the_structure_caps_a_turn_the_computers_could_command() {
        let (mut world, ship) = authority_world(4.0, 2.0, 40.0, f32::INFINITY);
        world.run_system_once(publish_flight_authority).unwrap();
        let loose = published(&world, ship).turn_rate;

        let (mut world, ship) = authority_world(4.0, 2.0, 40.0, 0.1);
        world.run_system_once(publish_flight_authority).unwrap();
        let braced = published(&world, ship).turn_rate;

        assert!(loose > 0.1, "the computers alone command a fast slew");
        assert!(
            (braced - 0.1).abs() < 1e-6,
            "a hull that cannot CARRY the turn publishes what it can carry, got {braced}"
        );
    }
}
