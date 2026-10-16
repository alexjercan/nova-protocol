//! Escort flight: an AI ship that flies formation on a leader while nothing
//! hostile is close enough to fight. The player's hired escorts are the one
//! user ([`AIFollow`] is inserted by nova_scenario's fleet hire and thaw).
//!
//! The follower flies its leader's heading at a lagged speed plus a closing
//! speed toward its slot, handed to the flight computer as a held
//! [`AutopilotAction::MatchVelocity`] that refuses the RCS except to dodge
//! its leader. Its own drive and turn move it; nothing writes its pose or
//! velocity, and the lag makes it follow a leader that starts rather than
//! share the leader's motion.
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

/// Time constant (seconds) of the follower's lag on its leader's speed gains.
/// The follower flies the leader's heading at the lagged speed plus a closing
/// speed toward its goal, so a leader that starts or speeds up opens a gap
/// first and the follower then flies after it, instead of matching the
/// leader's acceleration the step it starts. A turn and a slow-down are not
/// lagged: a lagged turn carried an inside escort across the leader's new
/// path, and a lagged stop flew it past a stopped leader.
pub const AI_FOLLOW_FEED_LAG: f32 = 2.0;

/// The fastest (world units per second, 20 m/s) a follower closes on its goal
/// over the lagged leader speed it flies. Without it a slot hundreds of
/// meters off asked for the full speed the hull could brake from, and an
/// escort raced past a starting leader at three times its speed.
pub const AI_FOLLOW_CLOSING_SPEED: f32 = 2.0;

/// How near (world units, 60 m) its goal a follower trims on its RCS, and how
/// long (seconds) its leader must have held its velocity first. The trim
/// takes only errors inside the flight computer's crumb band, which the drive
/// leaves alone. A leader that starts, speeds up, stops or turns leaves the
/// follower to its drive: an RCS that tracked the leader's change slid the
/// escort along as if dragged.
const AI_FOLLOW_TRIM: (f32, f32) = (6.0, 1.0);

/// Time (seconds) ahead at which a follower closing on its leader may use its
/// RCS to dodge: about the time its drive takes to turn onto the dodge. A
/// leader that turned into an escort on the drive alone came within its hull.
const AI_FOLLOW_DODGE_LEAD: f32 = 2.0;

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
/// travel, -Z along it. The follower settles to the leader's velocity there,
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

/// What a follower keeps of its leader between frames.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub(super) struct FollowFeed {
    /// The leader speed (world units per second) the follower flies on: the
    /// leader's speed with its gains seen through a lag of
    /// [`AI_FOLLOW_FEED_LAG`]. Seeded from the follower's own speed when it
    /// starts to fly formation, so it takes up nothing it does not already
    /// have.
    speed: f32,
    /// The velocity the leader holds, within the flight computer's crumb
    /// band, and for how long (seconds), to tell a leader that changes its
    /// motion from one that holds it.
    leader_velocity: Vec3,
    held_for: f32,
    /// The follower undocked while its leader locks it. The leader's lock
    /// asks it to hold still for a dock only once: it resumes formation at
    /// the undock, until the leader locks something else.
    undocked: bool,
}

/// Flies every [`AIFollow`] ship that is not fighting to its formation goal.
///
/// The goal comes from [`formation_goal`], and the leg to it is planned in
/// the leader's frame by `flight::navigation`, round the leader and every
/// hull and sized body within [`AI_FOLLOW_CLEARANCE`] of the leg. The held
/// velocity is the leader's heading at the [`FollowFeed`] speed plus a
/// closing speed toward the leg's next point that the hull can still brake
/// from at the goal, as the combat pilot closes on its ring, and never more
/// than [`AI_FOLLOW_CLOSING_SPEED`]. A follower of a travelling leader never
/// holds a velocity back against the travel: a slot that lies behind it is
/// waited for, not flown back to. The flight computer reaches the held
/// velocity with the hull's drive alone: an RCS push turns no hull and lights
/// no plume, so an escort that took up its leader's start on it slid along as
/// if dragged. A follower may use its RCS only to get out of its leader's
/// way: inside [`AI_FOLLOW_CLEARANCE`] of it, or closing on it fast enough
/// to get there within [`AI_FOLLOW_DODGE_LEAD`]. Inside that clearance it
/// first flies straight out, since the leg planner cannot steer out of a
/// clearance it starts inside.
///
/// Runs after `update_combat_flight` in the AI chain, so a ship that broke off
/// this frame flies to its slot this frame.
pub(super) fn update_follow_flight(
    mut commands: Commands,
    time: Res<Time>,
    settings: Res<FlightSettings>,
    mut q_follower: Query<
        (
            Entity,
            &AIFollow,
            (
                &Transform,
                Option<&ComputedCenterOfMass>,
                Option<&LinearVelocity>,
            ),
            &AIBehaviorState,
            Option<&HullRadius>,
            Option<&FlightAuthority>,
            Option<&mut Autopilot>,
            Option<&mut AILeash>,
            Option<&mut FollowFeed>,
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
            Option<&TravelLock>,
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
    let lag = 1.0 - (-time.delta_secs() / AI_FOLLOW_FEED_LAG).exp();
    for (
        ship,
        follow,
        (transform, com, own_velocity),
        state,
        arm,
        authority,
        autopilot,
        leash,
        feed,
        docked,
    ) in &mut q_follower
    {
        let Ok((
            leader_transform,
            leader_com,
            leader_velocity,
            leader_arm,
            leader_docked,
            leader_lock,
        )) = q_leader.get(follow.leader)
        else {
            commands.entity(ship).remove::<AIFollow>();
            continue;
        };
        let leader_anchor = live_structure_anchor(leader_transform, leader_com);
        if let Some(mut leash) = leash {
            leash.center = leader_anchor;
        }
        let own_velocity = own_velocity.map_or(Vec3::ZERO, |velocity| **velocity);
        let leader_velocity = leader_velocity.map_or(Vec3::ZERO, |velocity| **velocity);
        let locked = leader_lock.is_some_and(|lock| lock.0 == Some(ship));
        if docked || state.engages() || !q_engine.iter().any(|&ChildOf(parent)| parent == ship) {
            // Formation resumes from the motion the follower has then.
            if let Some(mut feed) = feed {
                *feed = FollowFeed {
                    speed: own_velocity.length(),
                    leader_velocity,
                    held_for: 0.0,
                    undocked: docked || (feed.undocked && locked),
                };
            }
            // Docked, the follower holds the pair still, as a docked passive
            // driver does, instead of flying the pair to its slot on the
            // velocity it held when it docked.
            if docked
                && autopilot
                    .is_some_and(|autopilot| !matches!(autopilot.action, AutopilotAction::Stop))
            {
                commands
                    .entity(ship)
                    .insert(Autopilot::engage(AutopilotAction::Stop));
            }
            continue;
        }
        let leader_speed = leader_velocity.length();
        let last = feed.as_deref().copied().unwrap_or(FollowFeed {
            speed: own_velocity.length(),
            leader_velocity,
            held_for: 0.0,
            undocked: false,
        });
        let fed_speed = (last.speed + (leader_speed - last.speed) * lag).min(leader_speed);
        let crumb = settings.attitude_deadband;
        let next_feed = if leader_velocity.distance(last.leader_velocity) <= crumb {
            FollowFeed {
                speed: fed_speed,
                held_for: last.held_for + time.delta_secs(),
                undocked: last.undocked && locked,
                ..last
            }
        } else {
            FollowFeed {
                speed: fed_speed,
                leader_velocity,
                held_for: 0.0,
                undocked: last.undocked && locked,
            }
        };
        match feed {
            Some(mut feed) => *feed = next_feed,
            None => {
                commands.entity(ship).insert(next_feed);
            }
        }
        let fed = leader_velocity.normalize_or_zero() * fed_speed;
        let own_arm = arm.map_or(0.0, |arm| **arm);
        let leader_arm = leader_arm.map_or(0.0, |arm| **arm);
        let from = live_structure_anchor(transform, com);
        // A leader that locks its follower near it is coming to dock: the
        // follower coasts on its own velocity, with no dodge, no clearance
        // and no RCS, so the leader can match it and the ports can meet.
        let docking = locked
            && !next_feed.undocked
            && from.distance(leader_anchor) < leader_arm + own_arm + AI_FOLLOW_HOLD.1;
        let action = if docking {
            AutopilotAction::MatchVelocity {
                velocity: own_velocity,
                facing: None,
                rcs: false,
            }
        } else if leader_docked {
            AutopilotAction::MatchVelocity {
                velocity: fed,
                facing: None,
                rcs: false,
            }
        } else {
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
            // The leader's clearance, in the leader's frame. `plan_leg` ignores
            // a body whose clearance holds `from`, so inside it the leg runs
            // straight at the goal, across the leader's bow when the goal lies
            // past it. The follower flies straight out first.
            let offset = from - leader_anchor;
            let bubble = leader_arm + own_arm + AI_FOLLOW_CLEARANCE;
            let next = if offset.length() < bubble {
                offset.normalize_or(Vec3::X) * (bubble + AI_FOLLOW_SLACK)
            } else {
                plan.goal
            };
            // The drive must turn before it pushes, too slow to dodge a
            // leader that turns into the follower; the RCS may help only
            // when the follower is in the leader's clearance or will be
            // within AI_FOLLOW_DODGE_LEAD at the speed they close. Not round
            // the other hulls: two escorts holding side by side sit inside
            // each other's clearance, and their RCS took up the leader's
            // start again.
            let closing_on_leader = (leader_velocity - own_velocity)
                .dot(offset.normalize_or_zero())
                .max(0.0);
            let crowded = offset.length() - bubble < closing_on_leader * AI_FOLLOW_DODGE_LEAD;
            let to_next = next + leader_anchor - from;
            // The way left, round a body when the leg detours, so the closing
            // speed brakes for the goal and not for the corner.
            let distance = to_next.length() + (next + leader_anchor).distance(goal);
            let closing = if distance > AI_FOLLOW_SLACK {
                to_next.normalize_or_zero()
                    * ai_radial_speed(
                        distance - AI_FOLLOW_SLACK,
                        authority.copied().unwrap_or_default(),
                        &settings,
                    )
                    .min(AI_FOLLOW_CLOSING_SPEED)
            } else {
                Vec3::ZERO
            };
            let facing = if distance > AI_FOLLOW_FORM_UP {
                Dir3::new(to_next).ok()
            } else {
                Dir3::new(leader_transform.rotation * Vec3::NEG_Z).ok()
            };
            let mut velocity = fed + closing;
            if leader_velocity.length() >= AI_FOLLOW_TRAVEL_SPEED {
                let ahead = leader_velocity.normalize();
                velocity -= ahead * velocity.dot(ahead).min(0.0);
            }
            // Fine positioning: at the slot, on its velocity, beside a
            // leader that holds its motion.
            let (trim_radius, trim_settle) = AI_FOLLOW_TRIM;
            let trim = distance < trim_radius
                && next_feed.held_for >= trim_settle
                && leader_speed - fed_speed < crumb
                && (velocity - own_velocity).length() < crumb;
            AutopilotAction::MatchVelocity {
                velocity,
                facing,
                rcs: crowded || trim,
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
        /// The largest velocity change of a step past what its own drive and
        /// RCS delivered that step, world units per second.
        worst_excess: f32,
        /// Steps traced, and the first step its velocity changed and its own
        /// thrust fired, counted from the last reset.
        steps: u32,
        first_move: Option<u32>,
        first_thrust: Option<u32>,
        /// The velocity the RCS delivered, world units per second.
        rcs: f32,
    }

    impl StepTrace {
        /// Count the steps, the first move and the first burn afresh.
        fn reset(&mut self) {
            self.worst_excess = 0.0;
            self.steps = 0;
            self.first_move = None;
            self.first_thrust = None;
            self.rcs = 0.0;
        }
    }

    /// Semi-implicit Euler moves a body by its new velocity; what a step may
    /// leave over is the velocity change it made. A pose written from outside
    /// shows as a step its velocity does not explain. A velocity change past
    /// the step's own impulse - each live drive's per-tick impulse times its
    /// input, over the mass, plus the RCS push the magazine delivered - is
    /// velocity the follower did not make itself.
    fn trace_steps(
        time: Res<Time>,
        settings: Res<FlightSettings>,
        mut trace: ResMut<StepTrace>,
        q_body: Query<(
            &Position,
            &LinearVelocity,
            &ComputedMass,
            Option<&RcsBudget>,
        )>,
        q_drive: Query<
            (&ChildOf, &ThrusterSectionMagnitude, &ThrusterSectionInput),
            Without<SectionInactiveMarker>,
        >,
    ) {
        let Some(ship) = trace.follower else {
            return;
        };
        let Ok((position, velocity, mass, rcs)) = q_body.get(ship) else {
            return;
        };
        let dt = time.delta_secs();
        let impulse: f32 = q_drive
            .iter()
            .filter(|(&ChildOf(parent), ..)| parent == ship)
            .map(|(_, magnitude, input)| **magnitude * input.clamp(0.0, 1.0))
            .sum();
        let pushed = rcs.map_or(0.0, |rcs| rcs.applied) * settings.rcs_accel * dt;
        let own = impulse / mass.value() + pushed;
        trace.rcs += pushed;
        let (at, velocity) = (position.0, velocity.0);
        if let Some((last_at, last_velocity)) = trace.last {
            let unexplained = ((at - last_at) - velocity * dt).length()
                - (velocity - last_velocity).length() * dt;
            trace.worst_jump = trace.worst_jump.max(unexplained);
            let change = (velocity - last_velocity).length();
            trace.worst_excess = trace.worst_excess.max(change - own);
            let step = trace.steps;
            if change > 1e-5 && trace.first_move.is_none() {
                trace.first_move = Some(step);
            }
            if own > 0.0 && trace.first_thrust.is_none() {
                trace.first_thrust = Some(step);
            }
        }
        trace.steps += 1;
        trace.last = Some((at, velocity));
    }

    /// The real flight layer and sections under the AI chain with the follow
    /// driver, as the patrol's physics harness wires the passive pilot.
    fn follow_physics_app() -> App {
        follow_rig(false)
    }

    /// [`follow_physics_app`], with docking when `docking` is set.
    fn follow_rig(docking: bool) -> App {
        let mut app = unfinished_integrity_physics_app();
        if docking {
            app.add_plugins(DockingSectionPlugin { render: false });
        }
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

    /// An escort in formation keeps clear of a leader that turns hard into
    /// it: a right angle to starboard, the escort's side, at 36 degrees a
    /// second. The drive alone turned too late, and the escort, inside the
    /// leader's clearance, flew to its new slot across the leader's bow.
    #[test]
    fn an_escort_dodges_a_leader_that_turns_into_it() {
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
        let escort = spawn_follower(&mut app, Vec3::new(24.0, 0.0, 10.0), leader);
        // A heavy hull, slow to turn, as the corvette the player hires.
        let mut controllers = app.world_mut().query::<&mut PDController>();
        for mut controller in controllers.iter_mut(app.world_mut()) {
            controller.max_angular_acceleration = 0.5;
        }
        app.world_mut().resource_mut::<StepTrace>().follower = Some(escort);
        settle(&mut app);
        let cruise = |tick: u32| (tick as f32 / 60.0).min(5.0);
        for tick in 0..1800 {
            app.world_mut()
                .entity_mut(leader)
                .insert(LinearVelocity(Vec3::NEG_Z * cruise(tick)));
            app.update();
        }
        let (distance, speed) = formation_error(&app, escort, leader);
        assert!(
            distance < 2.0 * AI_FOLLOW_SLACK && speed < 0.5,
            "the escort flies its slot before the turn: {distance:.1} u off, {speed:.2} u/s apart"
        );

        let rate = 36f32.to_radians();
        let mut closest = f32::INFINITY;
        for tick in 0..1200 {
            let turned = (tick as f32 / 60.0 * rate).min(90f32.to_radians());
            let rotation = Quat::from_rotation_y(-turned);
            let mut entity = app.world_mut().entity_mut(leader);
            entity.insert(LinearVelocity(rotation * Vec3::NEG_Z * 5.0));
            entity.insert(AngularVelocity(if turned < 90f32.to_radians() {
                Vec3::NEG_Y * rate
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
                    .distance(world.get::<Transform>(leader).unwrap().translation)
                    - 2.0 * ARM,
            );
        }
        assert!(
            closest > AI_FOLLOW_CLEARANCE * 0.5,
            "the escort kept clear of the leader's hull: closest {closest:.1} u"
        );
    }

    /// A docking port on `ship` at `offset`, its mouth turned out along
    /// `outward` in the hull's frame.
    fn spawn_port(app: &mut App, ship: Entity, offset: Vec3, outward: Vec3) -> Entity {
        app.world_mut()
            .spawn((
                ChildOf(ship),
                nova_events::prelude::EntityId("port".to_string()),
                Transform::from_translation(offset)
                    .with_rotation(Quat::from_rotation_arc(Vec3::NEG_Z, outward)),
                docking_section(DockingSectionConfig::default()),
            ))
            .id()
    }

    /// The player docks with its own escort: a leader that locks its escort
    /// and closes on it at 4 m/s finds the escort coasting where it was, its
    /// RCS dark, and the ports meet. Docked, the escort holds the pair and
    /// flies to no slot; undocked, it flies formation again.
    #[test]
    fn an_escort_holds_still_for_its_leader_to_dock_and_resumes_formation_at_the_undock() {
        let mut app = follow_rig(true);
        let leader = app
            .world_mut()
            .spawn((
                RigidBody::Dynamic,
                Transform::default(),
                HullRadius(ARM),
                Allegiance::Player,
                PlayerSpaceshipMarker,
                SpaceshipRootMarker,
            ))
            .id();
        app.world_mut().spawn((
            ChildOf(leader),
            Transform::default(),
            Collider::cuboid(1.0, 1.0, 1.0),
            ColliderDensity(1.0),
        ));
        spawn_port(&mut app, leader, Vec3::ZERO, Vec3::X);
        let escort = spawn_follower(&mut app, Vec3::new(20.0, 0.0, 0.0), leader);
        spawn_port(&mut app, escort, Vec3::new(-1.0, 0.0, 0.0), Vec3::NEG_X);
        app.world_mut().resource_mut::<StepTrace>().follower = Some(escort);
        settle(&mut app);
        for _ in 0..120 {
            app.world_mut()
                .entity_mut(leader)
                .insert(LinearVelocity::ZERO);
            app.update();
        }
        app.world_mut().resource_mut::<StepTrace>().reset();
        let at = |app: &App, ship: Entity| app.world().get::<Transform>(ship).unwrap().translation;
        let held_at = at(&app, escort);

        // The leader locks the escort and closes on it at 0.4 u/s, asking
        // for the dock every step.
        app.world_mut()
            .entity_mut(leader)
            .insert(TravelLock(Some(escort)));
        let mut docked_at = None;
        // The RCS the escort spent and how far it drifted up to the capture;
        // from the capture on it flies the pair.
        let mut approach = (0.0, 0.0);
        for tick in 0..4000 {
            app.world_mut()
                .entity_mut(leader)
                .insert(LinearVelocity(Vec3::X * 0.4));
            app.world_mut().trigger(DockingConnectionRequest {
                entity: leader,
                target: escort,
            });
            app.update();
            if app.world().get::<DockedShip>(escort).is_some() {
                docked_at = Some(tick);
                break;
            }
            approach = (
                app.world().resource::<StepTrace>().rcs,
                at(&app, escort).distance(held_at),
            );
        }
        let (rcs, drift) = approach;
        assert!(
            docked_at.is_some(),
            "the leader docked with its escort: the escort drifted {drift:.2} u from where it held"
        );
        assert!(
            rcs == 0.0 && drift < 0.5,
            "the escort held still for the dock: RCS {rcs} u/s, drifted {drift:.2} u"
        );

        // Docked, the escort holds the pair and flies to no slot.
        for _ in 0..300 {
            app.update();
            let action = app
                .world()
                .get::<Autopilot>(escort)
                .map(|pilot| pilot.action);
            assert!(
                app.world().get::<DockedShip>(escort).is_some()
                    && !matches!(action, Some(AutopilotAction::MatchVelocity { .. })),
                "the escort stays docked and holds the pair, not its slot: {action:?}"
            );
        }

        // Undocked, it flies formation again though the leader still locks it.
        app.world_mut()
            .trigger(DockingReleaseRequest { entity: leader });
        let mut range = 0.0;
        for _ in 0..1200 {
            app.update();
            range = at(&app, escort).distance(at(&app, leader));
        }
        assert!(
            app.world().get::<DockedShip>(escort).is_none()
                && range > 2.0 * ARM + AI_FOLLOW_HOLD.0 * 0.5,
            "the escort flew back out to its formation gap after the undock: {range:.1} u off"
        );
    }

    /// How far (world units, 30 m) a leader starting from rest travels while
    /// its escort must stay under half its speed.
    const OPENED: f32 = 3.0;

    /// An escort beside a leader at rest does not take up the leader's
    /// motion when the leader starts: the leader opens a gap first, and the
    /// escort follows it on its drive, its RCS only trimming at the slot once
    /// the leader holds its cruise, never back against
    /// the travel and never faster than the leader by more than its closing
    /// speed. No step changes the escort's velocity by more than its own
    /// drive delivered that step, and it moves no sooner than its drive
    /// fires.
    #[test]
    fn an_escort_lets_its_leader_open_a_gap_then_follows_on_its_drive_not_its_rcs() {
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
        for _ in 0..120 {
            app.update();
        }
        app.world_mut().resource_mut::<StepTrace>().reset();
        let at = |app: &App, ship: Entity| app.world().get::<Transform>(ship).unwrap().translation;
        let leader_start = at(&app, leader);

        // The leader starts from rest along -Z at 1 u/s2 to 5 u/s. While it
        // opens its first `OPENED`, the escort must stay under half its
        // speed; and it never flies back against the travel.
        let (mut shared, mut backward, mut overtake) = (false, 0.0_f32, 0.0_f32);
        // The RCS the escort spent while the leader sped up.
        let mut started = 0.0;
        for tick in 1..=900 {
            let speed = (tick as f32 / 60.0).min(5.0);
            app.world_mut()
                .entity_mut(leader)
                .insert(LinearVelocity(Vec3::NEG_Z * speed));
            app.update();
            if tick == 300 {
                started = app.world().resource::<StepTrace>().rcs;
            }
            let velocity = app.world().get::<LinearVelocity>(escort).unwrap().0;
            if at(&app, leader).distance(leader_start) < OPENED {
                shared |= velocity.length() > 0.5 * speed;
            }
            backward = backward.max(velocity.z);
            overtake = overtake.max(velocity.length() - speed);
        }
        let trace = app.world().resource::<StepTrace>();
        assert!(
            !shared,
            "the escort took up half the leader's speed before the leader opened {OPENED} u: it \
             shares the leader's motion instead of following it"
        );
        assert!(
            backward < 0.05,
            "the escort flew {backward:.2} u/s against the leader's travel"
        );
        assert!(
            overtake < AI_FOLLOW_CLOSING_SPEED + 0.5,
            "the escort flew {overtake:.2} u/s faster than the leader"
        );
        assert!(
            trace.worst_excess < 1e-4,
            "a step changed the escort's velocity {} u/s past its own thrust",
            trace.worst_excess
        );
        assert!(
            started < 0.05,
            "the escort's RCS pushed it {started} u/s while the leader sped up: a torque-free \
             push with no plume reads as the leader dragging it"
        );
        assert!(
            trace.rcs > 0.0 && trace.rcs < 0.5,
            "the escort's RCS pushed it {} u/s in all: it trims at the slot, it does not follow",
            trace.rcs
        );
        let (first_thrust, first_move) = (trace.first_thrust, trace.first_move);
        assert!(
            first_thrust.is_some() && first_thrust <= first_move,
            "the escort moves no sooner than its own thrust fires: thrust at step \
             {first_thrust:?}, moved at step {first_move:?}"
        );
        assert!(
            app.world()
                .get::<LinearVelocity>(escort)
                .unwrap()
                .0
                .length()
                > 2.5,
            "the escort follows the leader under its own drive"
        );
    }
}
