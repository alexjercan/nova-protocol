//! Escort flight: an AI ship that flies formation on a leader while nothing
//! hostile is close enough to fight. The player's hired escorts are the one
//! user ([`AIFollow`] is inserted by nova_scenario's fleet hire and thaw).
//!
//! The follower flies the combat pilot's frame: a velocity relative to its
//! leader, handed to the flight computer as a held
//! [`AutopilotAction::MatchVelocity`]. Thrust and the hull's own turn move it;
//! nothing writes its pose.
//!
//! The order owns the passive helm: `update_passive_flight` skips a follower,
//! and `update_combat_flight` leaves a held velocity to this driver when the
//! fight ends, so the two never hand the maneuver back and forth.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_gameplay::prelude::*;

use super::maneuver::ai_radial_speed;
use crate::prelude::*;

/// Gap (world units, 200 m) beside the leader at the formation slot, between
/// the leader's hull and the follower's.
pub const AI_FOLLOW_GAP: f32 = 20.0;

/// How far (world units, 100 m) the formation slot sits behind the leader's
/// anchor, along the leader's travel.
pub const AI_FOLLOW_TRAIL: f32 = 10.0;

/// The leader's speed (world units per second, 10 m/s) at and above which it
/// counts as travelling, and its travel sets the formation's frame. Below it
/// the follower holds where it is.
pub const AI_FOLLOW_TRAVEL_SPEED: f32 = 1.0;

/// The nearest and furthest gaps (world units, 150 m and 400 m) between the
/// two hulls at which a follower of a leader that does not travel holds where
/// it is. Outside them it closes to, or opens to, the nearer edge.
pub const AI_FOLLOW_HOLD: (f32, f32) = (15.0, 40.0);

/// Daylight (world units, 100 m) a follower keeps between its own hull and
/// the leader, or any hull or sized body on its way.
const AI_FOLLOW_CLEARANCE: f32 = 10.0;

/// Distance (world units, 30 m) inside which the follower counts as at its
/// goal and only matches the leader's velocity. Without it the closing speed
/// hunts back and forth across the goal.
const AI_FOLLOW_SLACK: f32 = 3.0;

/// Distance (world units, 200 m) from its goal inside which a follower points
/// its nose where the leader points; further out it points at its goal, so
/// the drive it closes with faces the way it closes.
const AI_FOLLOW_FORM_UP: f32 = 20.0;

/// Where a follower at `from` flies to keep formation on its leader.
///
/// `leader` is the leader's anchor, `velocity` its velocity and `up` its up
/// axis; `side` is the follower's [`AIFollow::side`] and `arms` the two hull
/// radii summed.
///
/// A travelling leader (at or above [`AI_FOLLOW_TRAVEL_SPEED`]) sets the
/// formation's frame with its velocity: the slot is `side` times both arms
/// plus [`AI_FOLLOW_GAP`], [`AI_FOLLOW_TRAIL`] behind the leader. The frame
/// is the travel and not the nose, so a leader that turns its nose without
/// turning its travel does not swing the slot round itself. A leader that
/// does not travel leaves the follower where it is, pulled into the
/// [`AI_FOLLOW_HOLD`] gaps, so a hull turning in place keeps its escort still.
pub fn formation_goal(
    from: Vec3,
    leader: Vec3,
    velocity: Vec3,
    up: Vec3,
    side: Vec3,
    arms: f32,
) -> Vec3 {
    if velocity.length() < AI_FOLLOW_TRAVEL_SPEED {
        let offset = from - leader;
        let (near, far) = AI_FOLLOW_HOLD;
        let distance = offset.length().clamp(arms + near, arms + far);
        return leader + offset.normalize_or(Vec3::X) * distance;
    }
    let ahead = velocity.normalize();
    let up = up
        .reject_from(ahead)
        .try_normalize()
        .unwrap_or_else(|| ahead.any_orthonormal_vector());
    let frame = Mat3::from_cols(ahead.cross(up), up, -ahead);
    leader + frame * (side * (arms + AI_FOLLOW_GAP) + Vec3::Z * AI_FOLLOW_TRAIL)
}

/// A standing follow order: fly formation on `leader`, on `side`.
///
/// `side` is a unit vector in the formation's frame, which
/// [`formation_goal`] builds from the leader's travel: +X is starboard of the
/// travel, -Z along it. The follower matches the leader's velocity there,
/// its nose on the leader's, and its [`AILeash`] centre rides the leader, so
/// a fight breaks off when it strays from the leader rather than from a
/// spawn point.
///
/// While the leader is docked the follower holds still beside it. A leader
/// that no longer exists ends the order.
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Component)]
pub struct AIFollow {
    /// The ship root the follower flies formation on.
    pub leader: Entity,
    /// The slot direction in the formation's frame, unit length.
    pub side: Vec3,
}

/// Flies every [`AIFollow`] ship that is not fighting to its formation goal.
///
/// The goal comes from [`formation_goal`], and the leg to it is planned in
/// the leader's frame by `flight::navigation`, round the leader and every
/// hull and sized body within [`AI_FOLLOW_CLEARANCE`] of the leg. The held
/// velocity is the leader's plus a closing speed toward the leg's next point
/// that the hull can still brake from at the goal, as the combat pilot closes
/// on its ring.
///
/// Runs after `update_combat_flight` in the AI chain, so a ship that broke off
/// this frame flies to its slot this frame.
pub(super) fn update_follow_flight(
    mut commands: Commands,
    settings: Res<FlightSettings>,
    mut q_follower: Query<
        (
            Entity,
            &AIFollow,
            &Transform,
            Option<&ComputedCenterOfMass>,
            &AIBehaviorState,
            Option<&HullRadius>,
            Option<&FlightAuthority>,
            Option<&mut Autopilot>,
            Option<&mut AILeash>,
            Has<DockedShip>,
        ),
        (
            With<SpaceshipRootMarker>,
            With<AISpaceshipMarker>,
            Without<ShipOrderHelmAuthority>,
        ),
    >,
    q_leader: Query<
        (
            &Transform,
            Option<&ComputedCenterOfMass>,
            Option<&LinearVelocity>,
            Option<&HullRadius>,
            Has<DockedShip>,
        ),
        Without<AIFollow>,
    >,
    // Every other hull on the leg: stations, the rest of the fleet, traffic.
    q_hull: Query<
        (
            Entity,
            &Transform,
            Option<&ComputedCenterOfMass>,
            &HullRadius,
        ),
        With<SpaceshipRootMarker>,
    >,
    // Sized solid bodies, as the patrol rounds them; a beacon is a mark.
    q_body: Query<(&Transform, &BodyRadius), (Without<AISpaceshipMarker>, Without<BeaconMarker>)>,
    // As `update_combat_flight` asks: a hull with no live thruster would see
    // the autopilot disengage and this driver re-engage every frame.
    q_engine: Query<&ChildOf, (With<ThrusterSectionMarker>, Without<SectionInactiveMarker>)>,
) {
    for (ship, follow, transform, com, state, arm, authority, autopilot, leash, docked) in
        &mut q_follower
    {
        let Ok((leader_transform, leader_com, leader_velocity, leader_arm, leader_docked)) =
            q_leader.get(follow.leader)
        else {
            commands.entity(ship).remove::<AIFollow>();
            continue;
        };
        let leader_anchor = live_structure_anchor(leader_transform, leader_com);
        if let Some(mut leash) = leash {
            leash.center = leader_anchor;
        }
        if docked || state.engages() || !q_engine.iter().any(|&ChildOf(parent)| parent == ship) {
            continue;
        }
        let leader_velocity = leader_velocity.map_or(Vec3::ZERO, |velocity| **velocity);
        let action = if leader_docked {
            AutopilotAction::MatchVelocity {
                velocity: leader_velocity,
                facing: None,
            }
        } else {
            let own_arm = arm.map_or(0.0, |arm| **arm);
            let leader_arm = leader_arm.map_or(0.0, |arm| **arm);
            let from = live_structure_anchor(transform, com);
            let goal = formation_goal(
                from,
                leader_anchor,
                leader_velocity,
                leader_transform.rotation * Vec3::Y,
                follow.side,
                own_arm + leader_arm,
            );
            // The leg in the leader's frame, where the leader stands still
            // and the formation goal holds.
            let obstacles: Vec<(Vec3, f32)> = std::iter::once((Vec3::ZERO, leader_arm))
                .chain(
                    q_hull
                        .iter()
                        .filter(|(hull, ..)| *hull != ship && *hull != follow.leader)
                        .map(|(_, hull, hull_com, radius)| {
                            (
                                live_structure_anchor(hull, hull_com) - leader_anchor,
                                **radius,
                            )
                        }),
                )
                .chain(
                    q_body
                        .iter()
                        .map(|(body, radius)| (body.translation - leader_anchor, **radius)),
                )
                .collect();
            let plan = plan_leg(
                from - leader_anchor,
                goal - leader_anchor,
                None,
                DetourPolicy {
                    clearance: own_arm + AI_FOLLOW_CLEARANCE,
                    hysteresis: AI_FOLLOW_SLACK,
                    arrive_radius: AI_FOLLOW_SLACK,
                },
                &obstacles,
            );
            let to_next = plan.goal + leader_anchor - from;
            // The way left, round a body when the leg detours, so the closing
            // speed brakes for the goal and not for the corner.
            let distance = to_next.length() + (plan.goal + leader_anchor).distance(goal);
            let closing = if distance > AI_FOLLOW_SLACK {
                to_next.normalize_or_zero()
                    * ai_radial_speed(
                        distance - AI_FOLLOW_SLACK,
                        authority.copied().unwrap_or_default(),
                        &settings,
                    )
            } else {
                Vec3::ZERO
            };
            let facing = if distance > AI_FOLLOW_FORM_UP {
                Dir3::new(to_next).ok()
            } else {
                Dir3::new(leader_transform.rotation * Vec3::NEG_Z).ok()
            };
            AutopilotAction::MatchVelocity {
                velocity: leader_velocity + closing,
                facing,
            }
        };
        match autopilot {
            // Retarget a held velocity in place: engaging afresh every tick
            // resets the phase the computer reports.
            Some(mut autopilot)
                if matches!(autopilot.action, AutopilotAction::MatchVelocity { .. }) =>
            {
                autopilot.action = action;
            }
            _ => {
                commands.entity(ship).insert(Autopilot::engage(action));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A travelling leader puts the slot starboard of its travel and behind
    /// it, whatever its nose does; a leader that does not travel leaves the
    /// follower where it stands inside the hold gaps and pulls it in from
    /// outside them.
    #[test]
    fn the_formation_follows_the_leaders_travel_and_holds_still_round_a_leader_at_rest() {
        let (leader, up, arms) = (Vec3::new(100.0, 0.0, 0.0), Vec3::Y, 6.0);

        let flying_north = formation_goal(Vec3::ZERO, leader, Vec3::NEG_Z * 5.0, up, Vec3::X, arms);
        assert_eq!(
            flying_north,
            leader + Vec3::new(arms + AI_FOLLOW_GAP, 0.0, AI_FOLLOW_TRAIL)
        );
        let flying_east = formation_goal(Vec3::ZERO, leader, Vec3::X * 5.0, up, Vec3::NEG_X, arms);
        assert!(
            flying_east.abs_diff_eq(
                leader + Vec3::new(-AI_FOLLOW_TRAIL, 0.0, -(arms + AI_FOLLOW_GAP)),
                1e-4
            ),
            "port of an eastward travel is north, behind is west: {flying_east}"
        );

        let (near, far) = AI_FOLLOW_HOLD;
        let inside = leader + Vec3::new(0.0, 0.0, arms + (near + far) * 0.5);
        assert_eq!(
            formation_goal(inside, leader, Vec3::X * 0.5, up, Vec3::X, arms),
            inside,
            "a slow leader leaves a follower inside the gaps where it is"
        );
        let outside = leader + Vec3::new(0.0, 0.0, 500.0);
        assert!(
            formation_goal(outside, leader, Vec3::ZERO, up, Vec3::X, arms)
                .abs_diff_eq(leader + Vec3::new(0.0, 0.0, arms + far), 1e-4),
            "a far follower closes to the far gap on its own bearing"
        );
    }
}

#[cfg(test)]
mod flight_tests {
    use nova_gameplay::test_support::{settle, unfinished_integrity_physics_app};

    use super::*;
    use crate::{
        input::{
            ai::{acquisition::update_ai_target, behavior::update_behavior_state},
            targeting::update_sensor_contacts,
        },
        sections::{
            controller_section::{
                sync_controller_section_forces, update_controller_section_rotation_input,
            },
            thruster_section::thruster_impulse_system,
        },
    };

    /// Both hulls' arm in the rig, world units.
    const ARM: f32 = 2.0;

    /// What every physics step of the follower showed.
    #[derive(Resource, Default)]
    struct StepTrace {
        follower: Option<Entity>,
        last: Option<(Vec3, Vec3)>,
        /// The largest step its own velocity does not explain, world units.
        worst_jump: f32,
    }

    /// Semi-implicit Euler moves a body by its new velocity; what a step may
    /// leave over is the velocity change it made. A pose written from outside
    /// shows as a step its velocity does not explain.
    fn trace_steps(
        time: Res<Time>,
        mut trace: ResMut<StepTrace>,
        q_body: Query<(&Position, &LinearVelocity)>,
    ) {
        let Some((position, velocity)) = trace.follower.and_then(|ship| q_body.get(ship).ok())
        else {
            return;
        };
        let (at, velocity) = (position.0, velocity.0);
        if let Some((last_at, last_velocity)) = trace.last {
            let unexplained = ((at - last_at) - velocity * time.delta_secs()).length()
                - (velocity - last_velocity).length() * time.delta_secs();
            trace.worst_jump = trace.worst_jump.max(unexplained);
        }
        trace.last = Some((at, velocity));
    }

    /// The real flight layer and sections under the AI chain with the follow
    /// driver, as the patrol's physics harness wires the passive pilot.
    fn follow_physics_app() -> App {
        let mut app = unfinished_integrity_physics_app();
        app.add_plugins(PDControllerPlugin);
        app.add_plugins(NovaFlightPlugin);
        app.configure_sets(
            FixedUpdate,
            (
                crate::input::SpaceshipInputSystems,
                NovaFlightSystems,
                PDControllerSystems::Sync,
                SpaceshipSectionSystems,
            )
                .chain(),
        );
        app.init_resource::<crate::input::targeting::prelude::TargetingSettings>();
        app.init_resource::<StepTrace>();
        app.add_systems(
            FixedUpdate,
            (
                crate::sections::signature::publish_ship_signatures,
                update_sensor_contacts,
                update_ai_target,
                update_behavior_state,
                update_follow_flight,
            )
                .chain()
                .in_set(crate::input::SpaceshipInputSystems),
        );
        app.add_systems(
            FixedUpdate,
            update_controller_section_rotation_input
                .after(NovaFlightSystems)
                .before(PDControllerSystems::Sync),
        );
        app.add_systems(
            FixedUpdate,
            (sync_controller_section_forces, thruster_impulse_system)
                .in_set(SpaceshipSectionSystems),
        );
        app.add_systems(FixedLast, trace_steps);
        app.finish();
        app
    }

    /// A follower with one drive aft, a hull and a flight computer, holding
    /// formation on `leader`'s starboard.
    fn spawn_follower(app: &mut App, at: Vec3, leader: Entity) -> Entity {
        let ship = app
            .world_mut()
            .spawn((
                RigidBody::Dynamic,
                Transform::from_translation(at),
                AISpaceshipMarker,
                Allegiance::Player,
                HullRadius(ARM),
                AIFollow {
                    leader,
                    side: Vec3::X,
                },
            ))
            .id();
        app.world_mut().spawn((
            ChildOf(ship),
            Transform::from_xyz(0.0, 0.0, -1.0),
            Collider::cuboid(1.0, 1.0, 1.0),
            ColliderDensity(1.0),
        ));
        app.world_mut().spawn((
            ChildOf(ship),
            ThrusterSectionMarker,
            ThrusterSectionMagnitude(10.0),
            ThrusterSectionInput(0.0),
            Transform::from_xyz(0.0, 0.0, 1.0),
            Collider::cuboid(1.0, 1.0, 1.0),
            ColliderDensity(1.0),
        ));
        app.world_mut().spawn((
            ChildOf(ship),
            ControllerSectionMarker,
            ControllerSectionRotationInput::default(),
            PDController {
                frequency: 4.0,
                damping_ratio: 4.0,
                max_angular_acceleration: 40.0,
                sustained_angular_speed: f32::INFINITY,
            },
            PDControllerTarget(ship),
            Transform::default(),
            Collider::cuboid(1.0, 1.0, 1.0),
            ColliderDensity(1.0),
        ));
        ship
    }

    /// The follower's anchor, its goal, and its speed relative to the leader.
    fn formation_error(app: &App, follower: Entity, leader: Entity) -> (f32, f32) {
        let world = app.world();
        let at = world.get::<Transform>(follower).unwrap().translation;
        let leader_transform = world.get::<Transform>(leader).unwrap();
        let leader_velocity = world.get::<LinearVelocity>(leader).unwrap().0;
        let goal = formation_goal(
            at,
            leader_transform.translation,
            leader_velocity,
            leader_transform.rotation * Vec3::Y,
            Vec3::X,
            2.0 * ARM,
        );
        let velocity = world.get::<LinearVelocity>(follower).unwrap().0;
        (at.distance(goal), (velocity - leader_velocity).length())
    }

    /// The follower's bearing round the leader in the XZ plane, radians.
    fn bearing(app: &App, follower: Entity, leader: Entity) -> f32 {
        let world = app.world();
        let offset = world.get::<Transform>(follower).unwrap().translation
            - world.get::<Transform>(leader).unwrap().translation;
        offset.x.atan2(-offset.z)
    }

    /// An escort holds still while its leader turns in place, flies to its
    /// slot behind and beside the leader's travel under its own drive when
    /// the leader accelerates away, keeps up with it, and follows it through
    /// a right-angle turn without passing near its hull. No physics step
    /// moves the escort further than its own velocity explains.
    #[test]
    fn an_escort_flies_formation_on_its_leader_under_its_own_drive() {
        let mut app = follow_physics_app();
        let leader = app
            .world_mut()
            .spawn((
                RigidBody::Kinematic,
                Transform::default(),
                LinearVelocity::ZERO,
                AngularVelocity::ZERO,
                HullRadius(ARM),
                Allegiance::Player,
            ))
            .id();
        let escort = spawn_follower(&mut app, Vec3::new(24.0, 0.0, 0.0), leader);
        app.world_mut().resource_mut::<StepTrace>().follower = Some(escort);
        settle(&mut app);

        // The leader turns its nose at 45 degrees a second, at rest.
        let before = bearing(&app, escort, leader);
        app.world_mut()
            .entity_mut(leader)
            .insert(AngularVelocity(Vec3::Y * 45f32.to_radians()));
        for _ in 0..480 {
            app.update();
        }
        app.world_mut()
            .entity_mut(leader)
            .insert(AngularVelocity::ZERO);
        let swing = (bearing(&app, escort, leader) - before).abs();
        assert!(
            swing < 2f32.to_radians(),
            "the escort holds still round a leader turning in place, it swung {:.1} degrees",
            swing.to_degrees()
        );

        // The leader accelerates along -Z to 5 u/s at 1 u/s2 and cruises.
        let mut closest = f32::INFINITY;
        let cruise = |tick: u32| (tick as f32 / 60.0).min(5.0);
        for tick in 0..1800 {
            app.world_mut()
                .entity_mut(leader)
                .insert(LinearVelocity(Vec3::NEG_Z * cruise(tick)));
            app.update();
            let world = app.world();
            closest = closest.min(
                world
                    .get::<Transform>(escort)
                    .unwrap()
                    .translation
                    .distance(world.get::<Transform>(leader).unwrap().translation),
            );
        }
        let (distance, speed) = formation_error(&app, escort, leader);
        assert!(
            distance < 2.0 * AI_FOLLOW_SLACK && speed < 0.5,
            "the escort keeps up on its slot behind and beside the travel: {distance:.1} u off, \
             {speed:.2} u/s apart"
        );
        assert!(
            app.world()
                .get::<LinearVelocity>(escort)
                .unwrap()
                .0
                .length()
                > 4.5,
            "the escort's own velocity carries it at the leader's cruise"
        );

        // The leader's travel turns a right angle to -X at 15 degrees a
        // second, its nose with it.
        for tick in 0..1800 {
            let turned = (tick as f32 / 60.0 * 15f32.to_radians()).min(90f32.to_radians());
            let rotation = Quat::from_rotation_y(turned);
            let mut entity = app.world_mut().entity_mut(leader);
            entity.insert(LinearVelocity(rotation * Vec3::NEG_Z * 5.0));
            entity.insert(AngularVelocity(if turned < 90f32.to_radians() {
                Vec3::Y * 15f32.to_radians()
            } else {
                Vec3::ZERO
            }));
            app.update();
            let world = app.world();
            closest = closest.min(
                world
                    .get::<Transform>(escort)
                    .unwrap()
                    .translation
                    .distance(world.get::<Transform>(leader).unwrap().translation),
            );
        }
        let (distance, speed) = formation_error(&app, escort, leader);
        assert!(
            distance < 2.0 * AI_FOLLOW_SLACK && speed < 0.5,
            "the escort is back on its slot after the turn: {distance:.1} u off, {speed:.2} u/s \
             apart"
        );
        assert!(
            closest > 2.0 * ARM + AI_FOLLOW_CLEARANCE * 0.5,
            "the escort never came near the leader's hull: closest {closest:.1} u"
        );
        let worst_jump = app.world().resource::<StepTrace>().worst_jump;
        assert!(
            worst_jump < 1e-2,
            "every step of the escort is its own velocity, the worst left {worst_jump} u over"
        );
    }
}
