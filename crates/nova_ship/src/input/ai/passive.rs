//! What an AI ship does with nothing hostile in range: fly its patrol legs,
//! hold an orbit, or station-keep. Drives the real autopilot rather than
//! steering directly; the autopilot itself routes each GOTO leg around any
//! sized body blocking it.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_gameplay::prelude::*;

#[cfg(test)]
use super::acquisition::update_ai_target;
#[cfg(test)]
use super::{behavior::update_behavior_state, maneuver::update_combat_flight};
#[cfg(test)]
#[cfg(test)]
use crate::input::targeting::update_sensor_contacts;
use crate::{
    flight::{LiveWells, WellTargetFault},
    prelude::*,
};

/// Arrival slack (world units, 250 m) on top of the autopilot's arrival
/// standoff for calling a patrol waypoint reached and turning onto the next leg. Turning early,
/// while the arrival curve is still braking, keeps the loop flowing instead
/// of stop-and-go at every corner.
const AI_WAYPOINT_SLACK: f32 = 25.0;

/// Per-ship override of [`AI_WAYPOINT_SLACK`]: how much slack this ship's
/// patrol adds on top of the autopilot's arrival standoff before calling a
/// waypoint reached. Small = the ship presses in close to each mark and the
/// loop reads deliberate (a nav drill hugging its beacons); the default 250 m
/// keeps combat patrols flowing. The autopilot still brakes toward rest one
/// resolved margin off this hull's own face, and the gate counts both and
/// measures from the same centre of mass the leg parks - so the slack has only
/// the arrival's own terminal drift left to cover. Author small, not zero.
/// Authored in meters via `AIControllerConfig::waypoint_slack`; this component
/// holds the world units the patrol compares against.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct AIWaypointSlack(pub f32);
/// Drift speed (u/s) above which a station-keeping ship burns to rest.
/// Holding position "loosely" means arresting drift, not chasing crumbs: kept
/// above the speed a completed STOP can hand back, so the helm rests between
/// corrections instead of re-engaging for ever. That bound is
/// `FlightSettings::settle_deadband` (0.75 u/s), not `stop_speed_epsilon`: a
/// STOP releases a residual it would have to turn the hull for. The margin is
/// therefore 1.33x, so raising `settle_deadband` past this constant is what
/// would break the guarantee.
const AI_IDLE_DRIFT_SPEED: f32 = 1.0;
/// Fly the passive states through the real autopilot (flight/) instead of
/// a parallel steering path: `Patrol` keeps a GOTO engaged toward the
/// current [`AIPatrolRoute`] waypoint (the autopilot itself routes the leg
/// around any sized body blocking it) and turns onto the next leg on
/// arrival; `Orbit` keeps an ORBIT engaged on its directive's well (the
/// autopilot plans its own insertion on the first tick and never
/// self-completes, so one engage holds the ring); `Idle` engages a STOP
/// burn while drifting faster than
/// [`AI_IDLE_DRIFT_SPEED`] (station keeping - the drift is arrested, not
/// rewound). The engaging states drop the autopilot: the AI's own actuator
/// systems own the helm and engines in combat, and a leftover passive
/// maneuver would fight them. Runs right after the state transition so a
/// flip takes effect the same frame.
///
/// Idle and Orbit let an already-engaged maneuver finish before taking
/// over, so a ship whose route is removed mid-leg (not a supported flow
/// today) flies out its stale GOTO once before settling into its routine.
pub(super) fn update_passive_flight(
    time: Res<Time>,
    settings: Res<FlightSettings>,
    mut commands: Commands,
    mut q_spaceship: Query<
        (
            Entity,
            &Transform,
            &LinearVelocity,
            &AIBehaviorState,
            Option<&mut AIPatrolRoute>,
            Option<&AIOrbitDirective>,
            Option<&Autopilot>,
            Option<&AIWaypointSlack>,
            Option<&FlightArrivalStandoff>,
            Option<&HullRadius>,
            Option<&ComputedCenterOfMass>,
            Option<&DominantWell>,
            (Option<&DockedShip>, Option<&DockedAssembly>),
        ),
        // A ship under a scenario helm order does not fly its own routine:
        // the order owns the helm until it is interrupted or reaches a
        // terminal outcome, and two writers on one hull's autopilot would
        // churn the maneuver between them every frame.
        // A follower's passive helm is the follow order's (`follow`).
        (
            With<SpaceshipRootMarker>,
            With<AISpaceshipMarker>,
            Without<ShipOrderHelmAuthority>,
            Without<AIFollow>,
            // A neutralized hull's crew is gone: its own RCS brake holds the
            // helm (`flight::neutralized_brake`).
            Without<NeutralizedMarker>,
        ),
    >,
    wells: LiveWells,
    // A hull built of sections flies its routine only while one of them is a
    // live thruster, as `update_combat_flight` asks. A station never had one
    // and a hulk lost them: the autopilot would disengage on both and this
    // driver re-engage the next frame, and on a station the maneuver's turn
    // would fight the controller that holds it level.
    q_section: Query<
        (&ChildOf, Has<ThrusterSectionMarker>),
        (With<SectionMarker>, Without<SectionInactiveMarker>),
    >,
) {
    let mut built = bevy::platform::collections::HashSet::new();
    let mut driven = bevy::platform::collections::HashSet::new();
    for (&ChildOf(parent), thruster) in &q_section {
        built.insert(parent);
        if thruster {
            driven.insert(parent);
        }
    }
    for (
        ship,
        transform,
        velocity,
        state,
        route,
        orbit,
        autopilot,
        slack,
        standoff,
        hull_radius,
        center_of_mass,
        dominant,
        (docked, assembly),
    ) in &mut q_spaceship
    {
        // A docked hull that does not drive its pair keeps its maneuver
        // frozen until it drives again or undocks. A driver holds the pair
        // on its centre of mass and velocity, the numbers the autopilot flies
        // it on. With no assembly it plans nothing: the flight writers log
        // that state once they are asked to move it.
        if docked.is_some_and(|docked| !docked.drives || assembly.is_none()) {
            continue;
        }
        if built.contains(&ship) && !driven.contains(&ship) {
            continue;
        }
        let has_autopilot = autopilot.is_some();
        let waypoint_slack = slack.map_or(AI_WAYPOINT_SLACK, |slack| slack.0);
        let hull_arm = hull_radius.map_or(0.0, |radius| **radius);
        // The gate mirrors the autopilot's own arrival rule, per-ship override
        // and hull size included: the leg comes to rest one resolved margin off
        // this hull's own face, so a gate that counted only the margin would
        // sit INSIDE the rest point on a big hull with tight slack and the
        // route would never turn.
        let rest_radius = resolved_arrival_standoff(standoff, &settings) + hull_arm;
        // WHERE THE SHIP IS, in the frame the flight computer answers in.
        // Every radius this routine spends is measured from the centre of
        // mass - `rest_radius` is `HullRadius`, which is COM-to-face, and the
        // leg the autopilot flies closes `goal - com_world` - so the position
        // they are compared against has to be the centre of mass too. On a
        // hull whose origin is well forward of it (49 m aft on the block
        // warship) the origin sits that much further from the waypoint than
        // the COM does: with the default 250 m slack the offset is absorbed,
        // but the menu weave authors 50 m (`main_menu/weave.rs`), which leaves
        // about a metre of headroom - and under the offset the route never
        // advances, `on_station` never latches, and the ship re-runs the same
        // GOTO forever, which is the exact churn the gate exists to prevent.
        let (position, velocity) = match assembly {
            Some(assembly) => (assembly.center_of_mass, assembly.linear_velocity),
            None => (
                transform.translation
                    + center_of_mass.map_or(Vec3::ZERO, |com| transform.rotation.mul_vec3(com.0)),
                velocity.0,
            ),
        };
        // A docked driver holds the pair still for its partner: no patrol,
        // orbit or stop runs until the undock, and the route picks up where it
        // was. A maneuver left from the routine is replaced by a STOP.
        if docked.is_some() {
            let routine = autopilot
                .is_some_and(|autopilot| !matches!(autopilot.action, AutopilotAction::Stop));
            if routine || (!has_autopilot && velocity.length() > AI_IDLE_DRIFT_SPEED) {
                commands
                    .entity(ship)
                    .insert(Autopilot::engage(AutopilotAction::Stop));
            }
            continue;
        }
        match *state {
            AIBehaviorState::Patrol => {
                // Patrol without a route cannot happen through the
                // transition function (the fallback picks Patrol only with
                // one), but a hand-set state should idle, not panic.
                let Some(mut route) = route else {
                    continue;
                };
                let Some(waypoint) = route.current_waypoint() else {
                    continue;
                };
                // Arrived: turn onto the next leg. The check runs on the
                // ship's position, not on autopilot completion, so a ship
                // shoved onto its waypoint (or re-entering Patrol on top of
                // one) advances too.
                let arrive_radius = rest_radius + waypoint_slack;
                // A running stop: station-keep at the waypoint until it runs
                // out, then turn onto the next leg. An engaged GOTO is left to
                // park on the waypoint it was flying to, as Idle lets a
                // maneuver finish.
                if let Some(hold) = route.hold.as_mut() {
                    hold.tick(time.delta_secs());
                    if hold.ready() {
                        route.hold = None;
                        route.advance();
                    } else {
                        if !has_autopilot && velocity.length() > AI_IDLE_DRIFT_SPEED {
                            commands
                                .entity(ship)
                                .insert(Autopilot::engage(AutopilotAction::Stop));
                        }
                        continue;
                    }
                } else if position.distance(waypoint) <= arrive_radius {
                    let stop = route.current_stop();
                    if stop > 0.0 {
                        route.hold = Some(Cooldown::started(stop));
                        continue;
                    }
                    route.advance();
                }
                let Some(raw_goal) = route.current_waypoint() else {
                    continue;
                };
                // On station (a single-waypoint route, parked at it with the
                // drift killed) there is nothing left to fly; re-engaging
                // would churn engage/complete every frame.
                let on_station = position.distance(raw_goal) <= arrive_radius
                    && velocity.length() <= AI_IDLE_DRIFT_SPEED;
                // (Re)engage when the goal changed or nothing is engaged; a
                // maneuver already flying the current goal is left alone
                // (re-engaging churns the autopilot phase). The autopilot
                // itself routes this leg around any sized body blocking it.
                let engaged_goto = autopilot.and_then(|autopilot| match autopilot.action {
                    AutopilotAction::GotoPos { position } => Some(position),
                    _ => None,
                });
                if engaged_goto != Some(raw_goal) && !on_station {
                    commands
                        .entity(ship)
                        .insert(Autopilot::engage(AutopilotAction::GotoPos {
                            position: raw_goal,
                        }));
                }
            }
            AIBehaviorState::Orbit => {
                drop_hold(route);
                // Orbit without a directive cannot happen through the
                // transition function; a hand-set state drifts, not panics.
                let Some(directive) = orbit else {
                    continue;
                };
                // A non-ORBIT maneuver (e.g. a stale patrol GOTO after a
                // hot-inserted directive) flies out first, same as Idle's
                // let-it-finish policy - and skips the well scan entirely.
                let engaged_well = match autopilot.map(|autopilot| autopilot.action) {
                    Some(AutopilotAction::Orbit { well, .. }) => Some(well),
                    Some(_) => continue,
                    None => None,
                };
                // The nearest well is picked when the ORBIT engages: ranking
                // again every frame would hop between two wells whose
                // surfaces a ring passes equally close to.
                if engaged_well.is_some() && directive.well == WellTargetType::NearestToShip {
                    continue;
                }
                // The ORBIT autopilot self-plans on its first engaged tick
                // and disengages itself if the well dies, so a bare engage
                // is enough; re-resolve and retry every calm frame (also
                // covers a well that spawns or streams in later than the
                // ship).
                let well = match wells.resolve(
                    &directive.well,
                    transform.translation,
                    dominant.map(|dominant| **dominant),
                ) {
                    Ok(well) => well,
                    Err(fault @ WellTargetFault::Missing(_)) => {
                        debug_once!(
                            "update_passive_flight: {fault}; ship {ship:?} drifts until one \
                             loads"
                        );
                        continue;
                    }
                    Err(fault @ WellTargetFault::Ambiguous { .. }) => {
                        warn_once!(
                            "update_passive_flight: {fault}; ship {ship:?} refuses to pick \
                             one and drifts"
                        );
                        continue;
                    }
                };
                // (Re)engage when nothing is engaged or the directive was
                // retargeted to another well - the ORBIT analogue of the
                // patrol arm's leg_changed; an autopilot already circling the
                // right well is left alone.
                if engaged_well != Some(well) {
                    commands
                        .entity(ship)
                        .insert(Autopilot::engage(AutopilotAction::Orbit {
                            well,
                            plan: None,
                        }));
                }
            }
            AIBehaviorState::Idle => {
                drop_hold(route);
                if !has_autopilot && velocity.length() > AI_IDLE_DRIFT_SPEED {
                    commands
                        .entity(ship)
                        .insert(Autopilot::engage(AutopilotAction::Stop));
                }
            }
            // Combat: `update_combat_flight` owns the helm, and takes it from
            // whatever passive leg was still flying. Clearing the maneuver here
            // would only churn the component it is about to write. Combat
            // also ends a running stop: the ship holds it again once it is
            // back at the waypoint.
            _ => {
                drop_hold(route);
            }
        }
    }
}

/// End a running patrol stop. Written only when one runs, so a route is not
/// marked changed every frame a ship spends off its patrol.
fn drop_hold(route: Option<Mut<AIPatrolRoute>>) {
    if let Some(mut route) = route {
        if route.hold.is_some() {
            route.hold = None;
        }
    }
}

#[cfg(test)]
mod patrol_idle_tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    const W1: Vec3 = Vec3::new(0.0, 0.0, -400.0);
    const W2: Vec3 = Vec3::new(400.0, 0.0, -400.0);

    /// Run the acquisition -> transition -> passive-flight pipeline once.
    fn run_pipeline(world: &mut World) {
        crate::input::ai::sense_and_pick(world);
        world.run_system_once(update_behavior_state).unwrap();
        world.run_system_once(update_combat_flight).unwrap();
        world.run_system_once(update_passive_flight).unwrap();
    }

    /// An engine and a flight computer: what the combat driver checks for
    /// before it hands a hull a maneuver, so a ship that has to fight in a
    /// test has to carry them.
    pub(super) fn make_flyable(world: &mut World, ship: Entity) {
        world.spawn((
            ChildOf(ship),
            ThrusterSectionMarker,
            ThrusterSectionInput(0.0),
        ));
        world.spawn((
            ChildOf(ship),
            ControllerSectionMarker,
            ControllerSectionRotationInput::default(),
            PDController {
                frequency: 4.0,
                damping_ratio: 4.0,
                max_angular_acceleration: 10.0,
                sustained_angular_speed: f32::INFINITY,
            },
        ));
    }

    fn patrol_world() -> (World, Entity) {
        let mut world = crate::input::ai::ai_test_world();
        world.init_resource::<FlightSettings>();
        world.init_resource::<Time>();
        let ship = world
            .spawn((
                AISpaceshipMarker,
                RigidBody::Dynamic,
                AIPatrolRoute::new(vec![W1, W2]),
                Transform::default(),
                LinearVelocity(Vec3::ZERO),
            ))
            .id();
        make_flyable(&mut world, ship);
        (world, ship)
    }

    #[test]
    fn a_patrol_ship_engages_a_goto_toward_its_waypoint() {
        // No hostile in the world: the route makes the fallback Patrol, and
        // Patrol flies the first leg through the real autopilot.
        let (mut world, ship) = patrol_world();

        run_pipeline(&mut world);

        assert_eq!(
            *world.entity(ship).get::<AIBehaviorState>().unwrap(),
            AIBehaviorState::Patrol,
            "a routed ship without a hostile patrols instead of idling"
        );
        assert_eq!(
            world.entity(ship).get::<Autopilot>().map(|ap| ap.action),
            Some(AutopilotAction::GotoPos { position: W1 }),
            "Patrol engages the GOTO autopilot toward the current waypoint"
        );
    }

    /// A neutralized patrol ship's crew is gone: once its maneuver is taken
    /// off, the routine never flies the route again.
    #[test]
    fn a_neutralized_patrol_ship_never_flies_its_route_again() {
        let (mut world, ship) = patrol_world();
        run_pipeline(&mut world);
        assert!(world.entity(ship).get::<Autopilot>().is_some());

        world
            .entity_mut(ship)
            .insert(NeutralizedMarker)
            .remove::<Autopilot>();
        run_pipeline(&mut world);
        run_pipeline(&mut world);

        assert!(
            world.entity(ship).get::<Autopilot>().is_none(),
            "neither the patrol nor the combat writer engages a maneuver"
        );
    }

    #[test]
    fn arrival_turns_onto_the_next_leg() {
        let (mut world, ship) = patrol_world();
        // Parked just short of W1, inside standoff + slack: arrived.
        world
            .entity_mut(ship)
            .get_mut::<Transform>()
            .unwrap()
            .translation = W1 + Vec3::new(0.0, 0.0, 60.0);

        run_pipeline(&mut world);

        assert_eq!(
            world.entity(ship).get::<AIPatrolRoute>().unwrap().current,
            1,
            "reaching a waypoint advances the loop"
        );
        assert_eq!(
            world.entity(ship).get::<Autopilot>().map(|ap| ap.action),
            Some(AutopilotAction::GotoPos { position: W2 }),
            "the new leg is engaged immediately"
        );
    }

    #[test]
    fn a_patrol_stop_holds_the_waypoint_before_the_next_leg() {
        let (mut world, ship) = patrol_world();
        world
            .entity_mut(ship)
            .get_mut::<AIPatrolRoute>()
            .unwrap()
            .stops = vec![2.0, 0.0];
        world
            .entity_mut(ship)
            .get_mut::<Transform>()
            .unwrap()
            .translation = W1 + Vec3::new(0.0, 0.0, 60.0);
        let current = |world: &World| world.entity(ship).get::<AIPatrolRoute>().unwrap().current;

        run_pipeline(&mut world);
        assert_eq!(
            current(&world),
            0,
            "arrival starts the stop, not the next leg"
        );
        assert!(
            world
                .entity(ship)
                .get::<AIPatrolRoute>()
                .unwrap()
                .hold
                .is_some(),
            "the stop runs at the waypoint"
        );

        world
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(1.5));
        run_pipeline(&mut world);
        assert_eq!(current(&world), 0, "the ship holds until the stop runs out");

        world
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(1.0));
        run_pipeline(&mut world);
        assert_eq!(
            current(&world),
            1,
            "a finished stop turns onto the next leg"
        );
        assert_eq!(
            world.entity(ship).get::<Autopilot>().map(|ap| ap.action),
            Some(AutopilotAction::GotoPos { position: W2 }),
            "the next leg is engaged as the stop ends"
        );
    }

    /// A docked AI ship that drives its pair holds it still for its partner
    /// instead of flying its patrol.
    #[test]
    fn a_docked_driver_holds_the_pair_instead_of_patrolling() {
        let (mut world, ship) = patrol_world();
        run_pipeline(&mut world);
        assert_eq!(
            world.entity(ship).get::<Autopilot>().map(|ap| ap.action),
            Some(AutopilotAction::GotoPos { position: W1 }),
            "the ship was flying its patrol when the dock was made"
        );
        world.entity_mut(ship).insert((
            DockedShip {
                connection: Entity::PLACEHOLDER,
                helm: Quat::IDENTITY,
                drives: true,
            },
            DockedAssembly {
                mass: 1.0,
                center_of_mass: Vec3::ZERO,
                linear_velocity: Vec3::ZERO,
                inertia: ComputedAngularInertia::default(),
                reach: 1.0,
            },
        ));

        run_pipeline(&mut world);

        assert_eq!(
            world.entity(ship).get::<Autopilot>().map(|ap| ap.action),
            Some(AutopilotAction::Stop),
            "the docked driver stops rather than flying to its waypoint"
        );
    }

    /// Regression: the gate spends a COM-relative radius, so it must measure
    /// a COM-relative position. The menu weave authors 50 m of slack
    /// (`main_menu/weave.rs`) and the block warship's centre of mass is about
    /// 49 m off its origin - an origin-measured gate leaves a metre of
    /// headroom, and under it the route never turns.
    #[test]
    fn a_hull_whose_origin_is_off_its_centre_of_mass_still_turns_onto_the_next_leg() {
        let (mut world, ship) = patrol_world();
        let standoff = world.resource::<FlightSettings>().arrival_standoff;
        // The weave's own slack, in the world units the component holds.
        let slack = 5.0;
        let offset = 4.9;
        // At the arrival the ship is braking retrograde, so its nose points
        // back up the approach and the hull's AFT centre of mass lies on the
        // waypoint side of the origin: the origin is the far one.
        world.entity_mut(ship).insert((
            AIWaypointSlack(slack),
            ComputedCenterOfMass(Vec3::new(0.0, 0.0, offset)),
        ));
        let mut entity = world.entity_mut(ship);
        let mut transform = entity.get_mut::<Transform>().unwrap();
        transform.rotation = Quat::from_rotation_y(std::f32::consts::PI);
        // Centre of mass just inside the gate, origin just outside it.
        transform.translation = W1 + Vec3::new(0.0, 0.0, standoff + slack - 1.0 + offset);

        run_pipeline(&mut world);

        assert_eq!(
            world.entity(ship).get::<AIPatrolRoute>().unwrap().current,
            1,
            "the gate measures the centre of mass the arrival parks, not the origin"
        );
    }

    #[test]
    fn the_loop_wraps_back_to_the_first_waypoint() {
        let (mut world, ship) = patrol_world();
        world
            .entity_mut(ship)
            .get_mut::<AIPatrolRoute>()
            .unwrap()
            .current = 1;
        world
            .entity_mut(ship)
            .get_mut::<Transform>()
            .unwrap()
            .translation = W2;

        run_pipeline(&mut world);

        assert_eq!(
            world.entity(ship).get::<AIPatrolRoute>().unwrap().current,
            0,
            "the route is a loop, not a one-way trip"
        );
    }

    #[test]
    fn an_out_of_range_index_self_heals() {
        // Both route fields are inspector-editable: shrinking the waypoint
        // list below `current` must wrap, not strand the patrol.
        let (mut world, ship) = patrol_world();
        world
            .entity_mut(ship)
            .get_mut::<AIPatrolRoute>()
            .unwrap()
            .current = 7; // 7 % 2 waypoints = W2

        run_pipeline(&mut world);

        assert_eq!(
            world.entity(ship).get::<Autopilot>().map(|ap| ap.action),
            Some(AutopilotAction::GotoPos { position: W2 }),
            "an out-of-range index wraps instead of stranding the route"
        );
    }

    #[test]
    fn a_mid_leg_maneuver_is_left_alone() {
        // Re-running the pipeline mid-leg must not re-engage (churning the
        // autopilot would reset its phase every frame). A re-engage is bit-
        // identical to the first engage (autopilot_system never runs here),
        // so plant a sentinel phase: churn resets it to Align, a left-alone
        // maneuver keeps burning (hardened alongside of).
        let (mut world, ship) = patrol_world();

        run_pipeline(&mut world);
        world.entity_mut(ship).get_mut::<Autopilot>().unwrap().phase = AutopilotPhase::Burn;
        run_pipeline(&mut world);

        assert_eq!(
            world.entity(ship).get::<Autopilot>().map(|ap| ap.phase),
            Some(AutopilotPhase::Burn),
            "an autopilot already flying the current leg is untouched"
        );
    }

    #[test]
    fn a_single_waypoint_station_holds_without_churn() {
        // Parked on a one-waypoint route with the drift killed: nothing to
        // fly, so nothing is engaged (re-engaging would churn
        // engage/complete every frame).
        let mut world = crate::input::ai::ai_test_world();
        world.init_resource::<FlightSettings>();
        world.init_resource::<Time>();
        let ship = world
            .spawn((
                AISpaceshipMarker,
                RigidBody::Dynamic,
                AIPatrolRoute::new(vec![W1]),
                Transform::from_translation(W1 + Vec3::new(0.0, 0.0, 60.0)),
                LinearVelocity(Vec3::ZERO),
            ))
            .id();

        run_pipeline(&mut world);

        assert!(
            world.entity(ship).get::<Autopilot>().is_none(),
            "on station at rest: no maneuver to fly"
        );
    }

    #[test]
    fn an_idle_drifter_burns_to_rest_and_then_rests() {
        // No route, no hostile: Idle. Drifting engages a STOP burn...
        let mut world = crate::input::ai::ai_test_world();
        world.init_resource::<FlightSettings>();
        world.init_resource::<Time>();
        let ship = world
            .spawn((
                AISpaceshipMarker,
                RigidBody::Dynamic,
                Transform::default(),
                LinearVelocity(Vec3::new(5.0, 0.0, 0.0)),
            ))
            .id();

        run_pipeline(&mut world);

        assert_eq!(
            *world.entity(ship).get::<AIBehaviorState>().unwrap(),
            AIBehaviorState::Idle
        );
        assert_eq!(
            world.entity(ship).get::<Autopilot>().map(|ap| ap.action),
            Some(AutopilotAction::Stop),
            "station keeping kills the drift through the real autopilot"
        );

        // ...while a ship already at rest is left alone (the autopilot
        // disengaged itself; below the drift threshold nothing re-engages).
        world.entity_mut(ship).remove::<Autopilot>();
        **world.entity_mut(ship).get_mut::<LinearVelocity>().unwrap() = Vec3::new(0.1, 0.0, 0.0);
        run_pipeline(&mut world);
        assert!(
            world.entity(ship).get::<Autopilot>().is_none(),
            "sub-threshold drift is accepted, not chased"
        );
    }

    /// A hull built of sections drifts without a maneuver once none of them
    /// is a live thruster - a station that never had one, or a hulk that lost
    /// them - and burns to rest again while one works.
    #[test]
    fn an_idle_drifter_with_no_live_thruster_section_flies_no_maneuver() {
        let mut world = crate::input::ai::ai_test_world();
        world.init_resource::<FlightSettings>();
        world.init_resource::<Time>();
        let ship = world
            .spawn((
                AISpaceshipMarker,
                RigidBody::Dynamic,
                Transform::default(),
                LinearVelocity(Vec3::new(5.0, 0.0, 0.0)),
            ))
            .id();
        world.spawn((ChildOf(ship), SectionMarker, ControllerSectionMarker));
        let thruster = world
            .spawn((
                ChildOf(ship),
                SectionMarker,
                ThrusterSectionMarker,
                SectionInactiveMarker,
            ))
            .id();

        run_pipeline(&mut world);
        assert!(
            world.entity(ship).get::<Autopilot>().is_none(),
            "no live thruster: nothing to burn with"
        );

        world.entity_mut(thruster).remove::<SectionInactiveMarker>();
        run_pipeline(&mut world);
        assert_eq!(
            world.entity(ship).get::<Autopilot>().map(|ap| ap.action),
            Some(AutopilotAction::Stop),
            "a live thruster flies the station keeping again"
        );
    }

    #[test]
    fn a_hostile_in_detection_range_interrupts_the_patrol() {
        let (mut world, ship) = patrol_world();
        run_pipeline(&mut world);
        assert!(world.entity(ship).get::<Autopilot>().is_some());

        // A hostile pops inside detection range: Engage, and the combat
        // driver takes the helm off the patrol leg.
        world.spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            RigidBody::Dynamic,
            Transform::from_translation(Vec3::new(300.0, 0.0, 0.0)),
        ));
        run_pipeline(&mut world);

        assert_eq!(
            *world.entity(ship).get::<AIBehaviorState>().unwrap(),
            AIBehaviorState::Engage
        );
        assert!(
            matches!(
                world.entity(ship).get::<Autopilot>().map(|ap| ap.action),
                Some(AutopilotAction::MatchVelocity { .. })
            ),
            "engaging replaces the patrol leg with a held velocity"
        );
    }

    #[test]
    fn a_hostile_beyond_detection_range_leaves_the_patrol_flying() {
        // Settle onto the patrol first: ships spawn in the default Engage
        // state, and a combat state holds on ANY acquired target - only the
        // passive states gate on detection range.
        let (mut world, ship) = patrol_world();
        run_pipeline(&mut world);

        // Acquired (inside the picket's own 2000u reach, and a 10u arm
        // returns 83u which gates at 2490u) but outside detection range.
        world.spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            RigidBody::Dynamic,
            HullRadius(10.0),
            Transform::from_translation(Vec3::new(1500.0, 0.0, 0.0)),
        ));
        run_pipeline(&mut world);

        assert!(
            world.entity(ship).get::<AITarget>().unwrap().is_some(),
            "the hostile is acquired"
        );
        assert_eq!(
            *world.entity(ship).get::<AIBehaviorState>().unwrap(),
            AIBehaviorState::Patrol,
            "but too far to abort the patrol for"
        );
    }
}

#[cfg(test)]
mod orbit_directive_tests {
    use bevy::ecs::system::RunSystemOnce;
    use nova_events::prelude::*;

    use super::*;

    const WELL_ID: &str = "planetoid";

    /// Run the acquisition -> transition -> passive-flight pipeline once.
    fn run_pipeline(world: &mut World) {
        crate::input::ai::sense_and_pick(world);
        world.run_system_once(update_behavior_state).unwrap();
        world.run_system_once(update_combat_flight).unwrap();
        world.run_system_once(update_passive_flight).unwrap();
    }

    /// A calm world with one orbit-directed AI ship; the well is spawned
    /// separately so tests can omit or delay it.
    fn orbit_world() -> (World, Entity) {
        let mut world = crate::input::ai::ai_test_world();
        world.init_resource::<FlightSettings>();
        world.init_resource::<Time>();
        let ship = world
            .spawn((
                AISpaceshipMarker,
                RigidBody::Dynamic,
                AIOrbitDirective {
                    well: WellTargetType::Authored(WELL_ID.to_string()),
                },
                Transform::default(),
                LinearVelocity(Vec3::ZERO),
            ))
            .id();
        super::patrol_idle_tests::make_flyable(&mut world, ship);
        (world, ship)
    }

    fn spawn_well(world: &mut World) -> Entity {
        world
            .spawn((
                GravityWell {
                    mu: 2400.0,
                    body_radius: 20.0,
                    soi_radius: 400.0,
                },
                EntityId::new(WELL_ID),
                ScenarioAddressableMarker,
                Position(Vec3::new(0.0, 0.0, -200.0)),
            ))
            .id()
    }

    #[test]
    fn an_orbit_ship_engages_the_orbit_autopilot_on_its_well() {
        let (mut world, ship) = orbit_world();
        let well = spawn_well(&mut world);

        run_pipeline(&mut world);

        assert_eq!(
            *world.entity(ship).get::<AIBehaviorState>().unwrap(),
            AIBehaviorState::Orbit,
            "a directed ship without a hostile orbits instead of idling"
        );
        assert_eq!(
            world.entity(ship).get::<Autopilot>().map(|ap| ap.action),
            Some(AutopilotAction::Orbit { well, plan: None }),
            "Orbit engages the ORBIT autopilot on the resolved well (the \
             autopilot plans the ring itself on its first tick)"
        );
    }

    #[test]
    fn the_directive_beats_a_patrol_route() {
        let (mut world, ship) = orbit_world();
        spawn_well(&mut world);
        world
            .entity_mut(ship)
            .insert(AIPatrolRoute::new(vec![Vec3::new(0.0, 0.0, -400.0)]));

        run_pipeline(&mut world);

        assert_eq!(
            *world.entity(ship).get::<AIBehaviorState>().unwrap(),
            AIBehaviorState::Orbit,
            "passive precedence: orbit > patrol"
        );
        assert!(
            matches!(
                world.entity(ship).get::<Autopilot>().map(|ap| ap.action),
                Some(AutopilotAction::Orbit { .. })
            ),
            "the ORBIT maneuver is engaged, not the patrol GOTO"
        );
    }

    #[test]
    fn an_unresolvable_well_id_drifts_without_panicking() {
        // No well entity in the world: the state still becomes Orbit (the
        // directive is present), but nothing is engaged - the ship drifts
        // until the well appears (spawn-order tolerance)...
        let (mut world, ship) = orbit_world();

        run_pipeline(&mut world);
        assert_eq!(
            *world.entity(ship).get::<AIBehaviorState>().unwrap(),
            AIBehaviorState::Orbit
        );
        assert!(
            world.entity(ship).get::<Autopilot>().is_none(),
            "no live well matches the id: nothing to engage"
        );

        // ...and the same pipeline engages once it does (delivery guard for
        // the nothing-happens half above).
        let well = spawn_well(&mut world);
        run_pipeline(&mut world);
        assert_eq!(
            world.entity(ship).get::<Autopilot>().map(|ap| ap.action),
            Some(AutopilotAction::Orbit { well, plan: None }),
            "a late-spawned well is picked up by the retry"
        );
    }

    #[test]
    fn a_mid_flight_orbit_is_left_alone() {
        // Re-running the pipeline must not re-engage (churn would reset the
        // autopilot's plan every frame). A re-engage produces a component
        // bit-identical to the first (autopilot_system never runs here), so
        // plant a sentinel plan the real autopilot would have computed: a
        // churn resets it to None, a left-alone maneuver keeps it.
        let (mut world, ship) = orbit_world();
        let well = spawn_well(&mut world);

        run_pipeline(&mut world);
        let sentinel = Some(OrbitPlan {
            radius: 123.0,
            normal: Vec3::Y,
        });
        world
            .entity_mut(ship)
            .get_mut::<Autopilot>()
            .unwrap()
            .action = AutopilotAction::Orbit {
            well,
            plan: sentinel,
        };
        run_pipeline(&mut world);

        assert_eq!(
            world.entity(ship).get::<Autopilot>().map(|ap| ap.action),
            Some(AutopilotAction::Orbit {
                well,
                plan: sentinel
            }),
            "an autopilot already circling the right well is untouched"
        );
    }

    #[test]
    fn a_retargeted_directive_re_engages_on_the_new_well() {
        // Editing the directive's well while an ORBIT is engaged must take
        // effect: ORBIT never self-completes, so waiting for the autopilot to
        // clear would ignore the retarget forever.
        let (mut world, ship) = orbit_world();
        spawn_well(&mut world);
        let other = world
            .spawn((
                GravityWell {
                    mu: 2400.0,
                    body_radius: 20.0,
                    soi_radius: 400.0,
                },
                EntityId::new("moon"),
                ScenarioAddressableMarker,
                Position(Vec3::new(0.0, 0.0, 300.0)),
            ))
            .id();

        run_pipeline(&mut world);
        world
            .entity_mut(ship)
            .get_mut::<AIOrbitDirective>()
            .unwrap()
            .well = WellTargetType::Authored("moon".to_string());
        run_pipeline(&mut world);

        assert_eq!(
            world.entity(ship).get::<Autopilot>().map(|ap| ap.action),
            Some(AutopilotAction::Orbit {
                well: other,
                plan: None
            }),
            "the retargeted directive re-engages on the new well"
        );
    }

    /// A nearest-well routine ranks once, when the ORBIT engages. A nearer
    /// well loading afterwards does not pull the ship off the ring it is
    /// flying, or a ring passing between two wells would hop forever.
    #[test]
    fn a_nearest_well_routine_keeps_the_well_it_engaged() {
        let (mut world, ship) = orbit_world();
        world
            .entity_mut(ship)
            .get_mut::<AIOrbitDirective>()
            .unwrap()
            .well = WellTargetType::NearestToShip;
        // Generated, not addressable: the nearest target takes any loaded
        // orbitable well.
        let first = world
            .spawn((
                GravityWell {
                    mu: 2400.0,
                    body_radius: 20.0,
                    soi_radius: 400.0,
                },
                EntityId::new("sector_0_0_0_planetoid_0"),
                Position(Vec3::new(0.0, 0.0, -200.0)),
            ))
            .id();

        run_pipeline(&mut world);
        assert_eq!(
            world.entity(ship).get::<Autopilot>().map(|ap| ap.action),
            Some(AutopilotAction::Orbit {
                well: first,
                plan: None
            }),
            "the only loaded well is the nearest"
        );

        world.spawn((
            GravityWell {
                mu: 2400.0,
                body_radius: 20.0,
                soi_radius: 400.0,
            },
            EntityId::new("sector_0_0_0_planetoid_1"),
            Position(Vec3::new(0.0, 0.0, 100.0)),
        ));
        run_pipeline(&mut world);
        assert_eq!(
            world.entity(ship).get::<Autopilot>().map(|ap| ap.action),
            Some(AutopilotAction::Orbit {
                well: first,
                plan: None
            }),
            "a nearer well loading later does not retarget the engaged ring"
        );
    }

    #[test]
    fn combat_interrupts_the_orbit_and_calm_resumes_it() {
        let (mut world, ship) = orbit_world();
        let well = spawn_well(&mut world);
        run_pipeline(&mut world);
        assert!(world.entity(ship).get::<Autopilot>().is_some());

        // A hostile inside detection range: Engage, and the combat driver
        // takes the helm off the ring.
        let hostile = world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                RigidBody::Dynamic,
                Transform::from_translation(Vec3::new(300.0, 0.0, 0.0)),
            ))
            .id();
        run_pipeline(&mut world);
        assert_eq!(
            *world.entity(ship).get::<AIBehaviorState>().unwrap(),
            AIBehaviorState::Engage
        );
        assert!(
            matches!(
                world.entity(ship).get::<Autopilot>().map(|ap| ap.action),
                Some(AutopilotAction::MatchVelocity { .. })
            ),
            "engaging replaces the ring with a held velocity"
        );

        // The hostile gone, the ship returns to its ring in the SAME frame:
        // the combat driver releases the helm before the passive pilot reads
        // it, which is what that ordering is for.
        world.despawn(hostile);
        run_pipeline(&mut world);
        assert_eq!(
            *world.entity(ship).get::<AIBehaviorState>().unwrap(),
            AIBehaviorState::Orbit
        );
        assert_eq!(
            world.entity(ship).get::<Autopilot>().map(|ap| ap.action),
            Some(AutopilotAction::Orbit { well, plan: None }),
            "calm re-engages the orbit"
        );
    }
}

#[cfg(test)]
mod patrol_physics_tests {
    // The full patrol loop on the physics harness: no hostile -> Patrol ->
    // GotoPos engaged -> the real autopilot swings the hull and burns the
    // real thruster -> the ship physically reaches its first waypoint and
    // turns onto the next leg. The acceptance: an AI ship placed
    // in a scenario flies its route before combat starts.
    use nova_gameplay::test_support::{settle, unfinished_integrity_physics_app};

    use super::*;
    use crate::sections::{
        controller_section::{
            sync_controller_section_forces, update_controller_section_rotation_input,
        },
        thruster_section::thruster_impulse_system,
    };

    /// The shared physics harness: real flight plugin, real sections, the AI
    /// passive pipeline chained in front.
    fn patrol_physics_app() -> App {
        let mut app = unfinished_integrity_physics_app();
        app.add_plugins(PDControllerPlugin);
        // The real flight layer: autopilot_system flies what
        // update_passive_flight engages.
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
        // The sensor pass reads the shipped lock settings.
        app.init_resource::<TargetingSettings>();
        app.add_systems(
            FixedUpdate,
            (
                crate::sections::signature::publish_ship_signatures,
                update_sensor_contacts,
                update_ai_target,
                update_behavior_state,
                update_passive_flight,
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
        app.finish();
        app
    }

    /// One AI ship with the minimum real section set (hull, thruster,
    /// controller) the autopilot needs to fly.
    fn spawn_patrol_ship(app: &mut App, waypoints: Vec<Vec3>) -> Entity {
        let ship = app
            .world_mut()
            .spawn((
                RigidBody::Dynamic,
                Transform::default(),
                AISpaceshipMarker,
                AIPatrolRoute::new(waypoints),
            ))
            .id();
        app.world_mut().spawn((
            ChildOf(ship),
            Name::new("hull"),
            Transform::from_xyz(0.0, 0.0, -1.0),
            Collider::cuboid(1.0, 1.0, 1.0),
            ColliderDensity(1.0),
        ));
        app.world_mut().spawn((
            ChildOf(ship),
            Name::new("thruster"),
            ThrusterSectionMarker,
            ThrusterSectionMagnitude(1.0),
            ThrusterSectionInput(0.0),
            Transform::from_xyz(0.0, 0.0, 1.0),
            Collider::cuboid(1.0, 1.0, 1.0),
            ColliderDensity(1.0),
        ));
        app.world_mut().spawn((
            ChildOf(ship),
            Name::new("controller"),
            ControllerSectionMarker,
            ControllerSectionRotationInput::default(),
            PDController {
                frequency: 4.0,
                damping_ratio: 4.0,
                max_angular_acceleration: 40.0,
                sustained_angular_speed: f32::INFINITY,
            },
            PDControllerTarget(ship),
            Transform::from_xyz(0.0, 0.0, 0.0),
            Collider::cuboid(1.0, 1.0, 1.0),
            ColliderDensity(1.0),
        ));
        ship
    }

    #[test]
    fn a_patrol_ship_flies_its_first_leg_and_turns_onto_the_next() {
        let mut app = patrol_physics_app();

        let first = Vec3::new(0.0, 0.0, -300.0);
        let second = Vec3::new(0.0, 0.0, 300.0);
        let ship = spawn_patrol_ship(&mut app, vec![first, second]);

        settle(&mut app);

        // No hostile anywhere: the routed ship must patrol, not idle.
        app.update();
        assert_eq!(
            *app.world().get::<AIBehaviorState>(ship).unwrap(),
            AIBehaviorState::Patrol
        );

        // Fly until the route turns onto the second leg (arrival at the
        // first waypoint), with a generous budget for align + burn + brake.
        let mut turned_at = None;
        for tick in 0..4800 {
            app.update();
            if app.world().get::<AIPatrolRoute>(ship).unwrap().current == 1 {
                turned_at = Some(tick);
                break;
            }
        }
        assert!(
            turned_at.is_some(),
            "the ship must physically reach its first waypoint and turn \
             onto the next leg within the budget"
        );

        // Still on the routine, and already flying the second leg.
        assert_eq!(
            *app.world().get::<AIBehaviorState>(ship).unwrap(),
            AIBehaviorState::Patrol
        );
        assert_eq!(
            app.world().get::<Autopilot>(ship).map(|ap| ap.action),
            Some(AutopilotAction::GotoPos { position: second }),
        );
    }
}
