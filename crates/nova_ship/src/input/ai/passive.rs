//! What an AI ship does with nothing hostile in range: fly its patrol legs,
//! hold an orbit, or station-keep. Drives the real autopilot rather than
//! steering directly.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;

#[cfg(test)]
use super::acquisition::update_ai_target;
#[cfg(test)]
use super::behavior::update_behavior_state;
#[cfg(test)]
use super::maneuver::on_thruster_input;
use crate::prelude::*;

/// Arrival slack (m) on top of the autopilot's arrival standoff for calling
/// a patrol waypoint reached and turning onto the next leg. Turning early,
/// while the arrival curve is still braking, keeps the loop flowing instead
/// of stop-and-go at every corner.
const AI_WAYPOINT_SLACK: f32 = 25.0;
/// Drift speed (u/s) above which a station-keeping ship burns to rest.
/// Holding position "loosely" means arresting drift, not chasing crumbs:
/// kept well above the autopilot's stop_speed_epsilon so a completed STOP
/// actually satisfies it and the helm rests between corrections.
const AI_IDLE_DRIFT_SPEED: f32 = 1.0;

/// Fly the passive states through the real autopilot (flight/) instead of
/// a parallel steering path: `Patrol` keeps a GOTO engaged toward the
/// current [`AIPatrolRoute`] waypoint and turns onto the next leg on
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
        ),
        (With<SpaceshipRootMarker>, With<AISpaceshipMarker>),
    >,
    q_wells: Query<(Entity, &EntityId), With<GravityWell>>,
) {
    for (ship, transform, velocity, state, route, orbit, autopilot) in &mut q_spaceship {
        let has_autopilot = autopilot.is_some();
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
                let arrive_radius = settings.arrival_standoff + AI_WAYPOINT_SLACK;
                if transform.translation.distance(waypoint) <= arrive_radius {
                    route.advance();
                }
                let Some(goal) = route.current_waypoint() else {
                    continue;
                };
                // On station (a single-waypoint route, parked at it with the
                // drift killed) there is nothing left to fly; re-engaging
                // would churn engage/complete every frame.
                let on_station = transform.translation.distance(goal) <= arrive_radius
                    && velocity.length() <= AI_IDLE_DRIFT_SPEED;
                // (Re)engage when the leg changed or nothing is engaged; a
                // maneuver already flying the current leg is left alone.
                let leg_changed = goal != waypoint;
                if (leg_changed || !has_autopilot) && !on_station {
                    commands
                        .entity(ship)
                        .insert(Autopilot::engage(AutopilotAction::GotoPos {
                            position: goal,
                        }));
                }
            }
            AIBehaviorState::Orbit => {
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
                // The ORBIT autopilot self-plans on its first engaged tick
                // and disengages itself if the well dies, so a bare engage
                // is enough; re-resolve and retry every calm frame (also
                // covers a well that spawns later than the ship).
                let Some(well) = q_wells
                    .iter()
                    .find(|(_, id)| ***id == *directive.well)
                    .map(|(entity, _)| entity)
                else {
                    debug_once!(
                        "update_passive_flight: orbit directive well '{}' matches no live \
                         GravityWell entity; ship {ship:?} drifts until it appears",
                        *directive.well
                    );
                    continue;
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
                if !has_autopilot && velocity.length() > AI_IDLE_DRIFT_SPEED {
                    commands
                        .entity(ship)
                        .insert(Autopilot::engage(AutopilotAction::Stop));
                }
            }
            // Combat: the AI actuator systems own the ship.
            _ => {
                if has_autopilot {
                    commands.entity(ship).remove::<Autopilot>();
                }
            }
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
        world.run_system_once(update_ai_target).unwrap();
        world.run_system_once(update_behavior_state).unwrap();
        world.run_system_once(update_passive_flight).unwrap();
    }

    fn patrol_world() -> (World, Entity) {
        let mut world = World::new();
        world.init_resource::<FlightSettings>();
        world.init_resource::<Time>();
        let ship = world
            .spawn((
                AISpaceshipMarker,
                AIPatrolRoute::new(vec![W1, W2]),
                Transform::default(),
                LinearVelocity(Vec3::ZERO),
            ))
            .id();
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
        let mut world = World::new();
        world.init_resource::<FlightSettings>();
        world.init_resource::<Time>();
        let ship = world
            .spawn((
                AISpaceshipMarker,
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
        let mut world = World::new();
        world.init_resource::<FlightSettings>();
        world.init_resource::<Time>();
        let ship = world
            .spawn((
                AISpaceshipMarker,
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

    #[test]
    fn a_hostile_in_detection_range_interrupts_the_patrol() {
        let (mut world, ship) = patrol_world();
        run_pipeline(&mut world);
        assert!(world.entity(ship).get::<Autopilot>().is_some());

        // A hostile pops inside detection range: Engage, and the passive
        // maneuver is dropped so the combat actuators own the ship.
        world.spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Transform::from_translation(Vec3::new(300.0, 0.0, 0.0)),
        ));
        run_pipeline(&mut world);

        assert_eq!(
            *world.entity(ship).get::<AIBehaviorState>().unwrap(),
            AIBehaviorState::Engage
        );
        assert!(
            world.entity(ship).get::<Autopilot>().is_none(),
            "engaging drops the passive-state autopilot"
        );
    }

    #[test]
    fn a_hostile_beyond_detection_range_leaves_the_patrol_flying() {
        // Settle onto the patrol first: ships spawn in the default Engage
        // state, and a combat state holds on ANY acquired target - only the
        // passive states gate on detection range.
        let (mut world, ship) = patrol_world();
        run_pipeline(&mut world);

        // Acquired (inside the 2000 m scan) but outside detection range.
        world.spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
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

    #[test]
    fn an_engaged_autopilot_owns_the_engines() {
        // While the flight computer flies a passive maneuver the AI thrust
        // system must not touch the throttles - not even to zero them.
        let (mut world, ship) = patrol_world();
        world
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Stop));
        let thruster = world
            .spawn((
                ThrusterSectionMarker,
                ThrusterSectionInput(0.7),
                GlobalTransform::IDENTITY,
                ChildOf(ship),
            ))
            .id();

        world.run_system_once(on_thruster_input).unwrap();
        assert_eq!(
            **world
                .entity(thruster)
                .get::<ThrusterSectionInput>()
                .unwrap(),
            0.7,
            "an engaged autopilot owns the throttles"
        );

        // Without a maneuver the passive state cuts the burn, as before.
        world.entity_mut(ship).remove::<Autopilot>();
        world.entity_mut(ship).insert(AIBehaviorState::Patrol);
        world.run_system_once(on_thruster_input).unwrap();
        assert_eq!(
            **world
                .entity(thruster)
                .get::<ThrusterSectionInput>()
                .unwrap(),
            0.0,
            "no autopilot: the passive state zeroes the throttles"
        );
    }
}

#[cfg(test)]
mod orbit_directive_tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    const WELL_ID: &str = "planetoid";

    /// Run the acquisition -> transition -> passive-flight pipeline once.
    fn run_pipeline(world: &mut World) {
        world.run_system_once(update_ai_target).unwrap();
        world.run_system_once(update_behavior_state).unwrap();
        world.run_system_once(update_passive_flight).unwrap();
    }

    /// A calm world with one orbit-directed AI ship; the well is spawned
    /// separately so tests can omit or delay it.
    fn orbit_world() -> (World, Entity) {
        let mut world = World::new();
        world.init_resource::<FlightSettings>();
        world.init_resource::<Time>();
        let ship = world
            .spawn((
                AISpaceshipMarker,
                AIOrbitDirective {
                    well: EntityId::new(WELL_ID),
                },
                Transform::default(),
                LinearVelocity(Vec3::ZERO),
            ))
            .id();
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
                Transform::from_translation(Vec3::new(0.0, 0.0, -200.0)),
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
                Transform::from_translation(Vec3::new(0.0, 0.0, 300.0)),
            ))
            .id();

        run_pipeline(&mut world);
        world
            .entity_mut(ship)
            .get_mut::<AIOrbitDirective>()
            .unwrap()
            .well = EntityId::new("moon");
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

    #[test]
    fn combat_interrupts_the_orbit_and_calm_resumes_it() {
        let (mut world, ship) = orbit_world();
        let well = spawn_well(&mut world);
        run_pipeline(&mut world);
        assert!(world.entity(ship).get::<Autopilot>().is_some());

        // A hostile inside detection range: Engage, and the passive
        // maneuver is dropped so the combat actuators own the ship.
        let hostile = world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::from_translation(Vec3::new(300.0, 0.0, 0.0)),
            ))
            .id();
        run_pipeline(&mut world);
        assert_eq!(
            *world.entity(ship).get::<AIBehaviorState>().unwrap(),
            AIBehaviorState::Engage
        );
        assert!(
            world.entity(ship).get::<Autopilot>().is_none(),
            "engaging drops the passive-state autopilot"
        );

        // The hostile gone, the ship returns to its ring.
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
    // turns onto the next leg. Pins the task's acceptance: an AI ship placed
    // in a scenario flies its route before combat starts.
    use nova_gameplay::test_support::{settle, unfinished_integrity_physics_app};

    use super::*;
    use crate::sections::{
        controller_section::{
            sync_controller_section_forces, update_controller_section_rotation_input,
        },
        thruster_section::thruster_impulse_system,
    };
    #[test]
    fn a_patrol_ship_flies_its_first_leg_and_turns_onto_the_next() {
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
        app.add_systems(
            FixedUpdate,
            (
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

        let first = Vec3::new(0.0, 0.0, -300.0);
        let second = Vec3::new(0.0, 0.0, 300.0);
        let ship = app
            .world_mut()
            .spawn((
                RigidBody::Dynamic,
                Transform::default(),
                AISpaceshipMarker,
                AIPatrolRoute::new(vec![first, second]),
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
                max_torque: 40.0,
            },
            PDControllerTarget(ship),
            Transform::from_xyz(0.0, 0.0, 0.0),
            Collider::cuboid(1.0, 1.0, 1.0),
            ColliderDensity(1.0),
        ));

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
