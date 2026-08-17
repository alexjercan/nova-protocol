//! What an AI ship does with nothing hostile in range: fly its patrol legs
//! (steering around sized bodies via [`AIAvoidanceDetour`]), hold an orbit,
//! or station-keep. Drives the real autopilot rather than steering directly.

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

/// Per-ship override of [`AI_WAYPOINT_SLACK`]: how much slack this ship's
/// patrol adds on top of the autopilot's arrival standoff before calling a
/// waypoint reached. Small = the ship presses in close to each mark and the
/// loop reads deliberate (a nav drill hugging its beacons); the default 25
/// keeps combat patrols flowing. The autopilot still brakes toward rest at
/// `FlightSettings::arrival_standoff` from the mark, so slack below ~2 risks
/// asymptoting outside the advance gate - author small, not zero. Authored
/// via `AIControllerConfig::waypoint_slack`.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct AIWaypointSlack(pub f32);
/// Drift speed (u/s) above which a station-keeping ship burns to rest.
/// Holding position "loosely" means arresting drift, not chasing crumbs:
/// kept well above the autopilot's stop_speed_epsilon so a completed STOP
/// actually satisfies it and the helm rests between corrections.
const AI_IDLE_DRIFT_SPEED: f32 = 1.0;
/// Lateral clearance (m) beyond a body's geometric [`BodyRadius`] under
/// which a patrol leg counts as blocked. The autopilot itself has no
/// obstacle awareness ([`AutopilotAction::GotoPos`] flies a straight leg),
/// so this is the passive pilot's own margin; sized to a ship length plus
/// drift slack, NOT to the noise spread of asteroid meshes - the derived
/// BodyRadius already carries the real geometric extent.
const AI_AVOID_MARGIN: f32 = 20.0;
/// Extra clearance a DETOURING ship demands before it calls the direct leg
/// clear again: the clear-check margin is [`AI_AVOID_MARGIN`] plus this
/// band, so a leg that just cleared sits comfortably outside the
/// block-check and cannot re-block next tick. Without the band, a leg
/// grazing the margin flips blocked/clear every tick, and each flip swaps
/// the GOTO goal - autopilot churn that resets the maneuver to Align
/// forever.
const AI_AVOID_HYSTERESIS: f32 = 10.0;

/// The active avoidance detour: a STABLE intermediate GOTO goal, held until
/// the ship reaches it or the direct leg to the current waypoint clears.
/// Stability is the point - recomputing the corner from the live ship
/// position would move the goal every frame, and a moving goal re-engages
/// (churns) the autopilot back into its align phase forever.
#[derive(Component, Clone, Copy, Debug, Reflect)]
pub struct AIAvoidanceDetour(pub Vec3);

/// The first sized body blocking the leg `from -> to`: its center comes
/// within `body_radius + margin` of the leg, measured at the closest point
/// on the segment. Two kinds of body are ignored: one whose clearance
/// already contains `from` (the ship is inside its bubble; this geometry
/// cannot steer OUT of a sphere, and calling it a blocker would spin
/// corners around the ship's own position - the flown goal carries it out),
/// and one whose clearance contains `to` (fly-at-goal legs are pre-adjusted
/// outside every bubble by [`goal_outside_clearance`], so this only guards
/// degenerate geometry from looping). Returns (center, body_radius,
/// closest point).
fn first_leg_blocker(
    from: Vec3,
    to: Vec3,
    margin: f32,
    obstacles: impl Iterator<Item = (Vec3, f32)>,
) -> Option<(Vec3, f32, Vec3)> {
    let leg = to - from;
    let len_sq = leg.length_squared();
    let mut best: Option<(f32, (Vec3, f32, Vec3))> = None;
    for (center, radius) in obstacles {
        let clearance = radius + margin;
        let clearance_sq = clearance * clearance;
        if to.distance_squared(center) < clearance_sq
            || from.distance_squared(center) < clearance_sq
        {
            continue;
        }
        let t = if len_sq > f32::EPSILON {
            ((center - from).dot(leg) / len_sq).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let closest = from + leg * t;
        if closest.distance_squared(center) >= clearance_sq {
            continue;
        }
        if best.is_none_or(|(best_t, _)| t < best_t) {
            best = Some((t, (center, radius, closest)));
        }
    }
    best.map(|(_, blocker)| blocker)
}

/// `goal`, pushed out of any sized body's clearance it sits inside - to the
/// nearest point on the bubble's surface (plus a step of slack). A patrol
/// waypoint scattered against a rock face is a routine hazard in a dense
/// authored band, and skipping such rocks in the blocker scan (the old
/// behavior) flew the leg straight through them; flying AT the adjusted
/// goal instead keeps the pilot clear while the arrival check - which runs
/// on the RAW waypoint - still turns the route on time. Iterative because
/// the pushed-out point can land inside a neighboring bubble; bounded so
/// pathological nests cannot spin the loop.
fn goal_outside_clearance(goal: Vec3, margin: f32, obstacles: &[(Vec3, f32)]) -> Vec3 {
    let mut adjusted = goal;
    for _ in 0..4 {
        let Some((center, radius)) = obstacles.iter().copied().find(|(center, radius)| {
            let clearance = radius + margin;
            adjusted.distance_squared(*center) < clearance * clearance
        }) else {
            return adjusted;
        };
        let out = (adjusted - center).try_normalize().unwrap_or(Vec3::Y);
        adjusted = center + out * (radius + margin + 1.0);
    }
    adjusted
}

/// The corner goal that rounds `blocker`: pushed out from the body's center
/// through the leg's closest point. The push runs past the blocked
/// clearance by the hysteresis band PLUS the corner's own arrival window
/// (`arrive_radius`): anywhere the pilot can call the corner reached must
/// already see the direct leg comfortably clear, or the rounding stalls
/// hopping corner to corner inside its own arrival radius (observed on the
/// physics harness before the window was added). A body dead on the leg
/// line has no side to prefer; any perpendicular works and the pick only
/// has to be deterministic.
fn detour_around(from: Vec3, to: Vec3, blocker: (Vec3, f32, Vec3), arrive_radius: f32) -> Vec3 {
    let (center, radius, closest) = blocker;
    let side = (closest - center).try_normalize().unwrap_or_else(|| {
        Dir3::new(to - from).map_or(Vec3::X, |leg| leg.any_orthonormal_vector())
    });
    center + side * (radius + AI_AVOID_MARGIN + AI_AVOID_HYSTERESIS + arrive_radius)
}

/// Fly the passive states through the real autopilot (flight/) instead of
/// a parallel steering path: `Patrol` keeps a GOTO engaged toward the
/// current [`AIPatrolRoute`] waypoint - detouring around any sized body
/// blocking the leg (the GOTO itself flies blind straight lines; see
/// [`AIAvoidanceDetour`]) - and turns onto the next leg on
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
            Option<&AIAvoidanceDetour>,
            Option<&AIWaypointSlack>,
            Option<&FlightArrivalStandoff>,
        ),
        (With<SpaceshipRootMarker>, With<AISpaceshipMarker>),
    >,
    q_wells: Query<(Entity, &EntityId), With<GravityWell>>,
    q_obstacles: Query<(&Transform, &BodyRadius), Without<AISpaceshipMarker>>,
) {
    // Sized bodies (asteroids, planetoids - anything with a derived
    // geometric BodyRadius) are what patrol legs steer around. Collected
    // once; the field does not change per ship.
    let obstacles: Vec<(Vec3, f32)> = q_obstacles
        .iter()
        .map(|(transform, radius)| (transform.translation, **radius))
        .collect();
    for (ship, transform, velocity, state, route, orbit, autopilot, detour, slack, standoff) in
        &mut q_spaceship
    {
        let has_autopilot = autopilot.is_some();
        let waypoint_slack = slack.map_or(AI_WAYPOINT_SLACK, |slack| slack.0);
        // The gate mirrors the autopilot's own arrival rule, per-ship
        // override included: a ship authored to park closer must not have
        // its patrol turn early on the global standoff.
        let arrival_standoff = standoff.map_or(settings.arrival_standoff, |standoff| **standoff);
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
                let arrive_radius = arrival_standoff + waypoint_slack;
                let position = transform.translation;
                let mut detour = detour.map(|detour| detour.0);
                if position.distance(waypoint) <= arrive_radius {
                    route.advance();
                    // A detour belongs to the leg it was computed for.
                    if detour.take().is_some() {
                        commands.entity(ship).remove::<AIAvoidanceDetour>();
                    }
                }
                let Some(raw_goal) = route.current_waypoint() else {
                    continue;
                };
                // On station (a single-waypoint route, parked at it with the
                // drift killed) there is nothing left to fly; re-engaging
                // would churn engage/complete every frame.
                let on_station = position.distance(raw_goal) <= arrive_radius
                    && velocity.length() <= AI_IDLE_DRIFT_SPEED;
                // The FLOWN goal is the waypoint pushed outside any body's
                // bubble it was scattered/authored into; arrival above keeps
                // running on the raw waypoint, so the route still turns.
                let goal = goal_outside_clearance(raw_goal, AI_AVOID_MARGIN, &obstacles);
                // A held detour is flown out until the corner is reached or
                // the direct leg is comfortably clear (the hysteresis band) -
                // but its OWN leg is re-validated every tick: momentum and
                // neighbors a single corner never saw can put a body on the
                // way to the corner, and a corner flown blind is exactly the
                // crash the detour exists to prevent. A blocked corner leg
                // HOPS: the corner is replaced by one rounding that blocker
                // (a real goal change, so the churn guard below re-engages).
                if let Some(corner) = detour {
                    let clear = first_leg_blocker(
                        position,
                        goal,
                        AI_AVOID_MARGIN + AI_AVOID_HYSTERESIS,
                        obstacles.iter().copied(),
                    )
                    .is_none();
                    if clear || position.distance(corner) <= arrive_radius {
                        commands.entity(ship).remove::<AIAvoidanceDetour>();
                        detour = None;
                    } else if let Some(blocker) = first_leg_blocker(
                        position,
                        corner,
                        AI_AVOID_MARGIN,
                        obstacles.iter().copied(),
                    ) {
                        let hop = detour_around(position, corner, blocker, arrive_radius);
                        commands.entity(ship).insert(AIAvoidanceDetour(hop));
                        detour = Some(hop);
                    }
                }
                // Fresh decision: fly the leg, unless a body blocks it - then
                // hold a corner that rounds the first blocker. Reaching a
                // corner with the leg still blocked lands here too and picks
                // the corner around the NEXT blocker (the field is crossed
                // one rounding at a time).
                let flight_goal = detour.unwrap_or_else(|| {
                    match first_leg_blocker(
                        position,
                        goal,
                        AI_AVOID_MARGIN,
                        obstacles.iter().copied(),
                    ) {
                        Some(blocker) => {
                            let corner = detour_around(position, goal, blocker, arrive_radius);
                            commands.entity(ship).insert(AIAvoidanceDetour(corner));
                            corner
                        }
                        None => goal,
                    }
                });
                // (Re)engage when the flown goal changed or nothing is
                // engaged; a maneuver already flying the current goal is
                // left alone (re-engaging churns the autopilot phase).
                let engaged_goto = autopilot.and_then(|autopilot| match autopilot.action {
                    AutopilotAction::GotoPos { position } => Some(position),
                    _ => None,
                });
                if engaged_goto != Some(flight_goal) && !on_station {
                    commands
                        .entity(ship)
                        .insert(Autopilot::engage(AutopilotAction::GotoPos {
                            position: flight_goal,
                        }));
                }
            }
            AIBehaviorState::Orbit => {
                // The ORBIT autopilot plans its own ring; a leftover patrol
                // detour has no meaning here.
                if detour.is_some() {
                    commands.entity(ship).remove::<AIAvoidanceDetour>();
                }
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
                if detour.is_some() {
                    commands.entity(ship).remove::<AIAvoidanceDetour>();
                }
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
                if detour.is_some() {
                    commands.entity(ship).remove::<AIAvoidanceDetour>();
                }
            }
        }
    }
}

#[cfg(test)]
mod avoidance_tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    const W1: Vec3 = Vec3::new(0.0, 0.0, -400.0);
    const W2: Vec3 = Vec3::new(400.0, 0.0, -400.0);

    #[test]
    fn a_clear_leg_has_no_blocker() {
        // Off to the side by more than radius + margin: clear.
        let rocks = [(Vec3::new(0.0, 80.0, -200.0), 40.0)];
        assert!(
            first_leg_blocker(Vec3::ZERO, W1, AI_AVOID_MARGIN, rocks.iter().copied()).is_none()
        );
    }

    #[test]
    fn the_nearest_intruding_body_blocks_the_leg() {
        let near = (Vec3::new(10.0, 0.0, -150.0), 40.0);
        let far = (Vec3::new(-10.0, 0.0, -300.0), 40.0);
        let (center, radius, _) =
            first_leg_blocker(Vec3::ZERO, W1, AI_AVOID_MARGIN, [far, near].iter().copied())
                .expect("both intrude; the leg is blocked");
        assert_eq!((center, radius), near, "the FIRST body on the leg wins");
    }

    #[test]
    fn a_body_hugging_the_waypoint_is_not_a_blocker() {
        // A waypoint authored inside a body's clearance can never clear;
        // detouring around it would circle the author's mistake forever
        // instead of flying the route (the GOTO's own arrival standoff is
        // what keeps the ship off the rock).
        let rocks = [(W1 + Vec3::new(0.0, 30.0, 0.0), 40.0)];
        assert!(
            first_leg_blocker(Vec3::ZERO, W1, AI_AVOID_MARGIN, rocks.iter().copied()).is_none()
        );
    }

    #[test]
    fn the_detour_corner_clears_the_blocker() {
        let rocks = [(Vec3::new(15.0, 0.0, -200.0), 40.0)];
        let blocker = first_leg_blocker(Vec3::ZERO, W1, AI_AVOID_MARGIN, rocks.iter().copied())
            .expect("the rock sits on the leg");
        let corner = detour_around(Vec3::ZERO, W1, blocker, 75.0);
        assert!(
            corner.distance(rocks[0].0)
                > rocks[0].1 + AI_AVOID_MARGIN + AI_AVOID_HYSTERESIS + 75.0 - 1.0,
            "the corner sits past the clear-check band plus its arrival window"
        );
    }

    #[test]
    fn a_dead_center_body_still_yields_a_corner() {
        // The closest point coincides with the center: no side to prefer,
        // but the pick must be deterministic and outside the clearance,
        // not NaN.
        let rocks = [(Vec3::new(0.0, 0.0, -200.0), 40.0)];
        let blocker = first_leg_blocker(Vec3::ZERO, W1, AI_AVOID_MARGIN, rocks.iter().copied())
            .expect("dead on the leg");
        let corner = detour_around(Vec3::ZERO, W1, blocker, 75.0);
        assert!(corner.is_finite());
        assert!(corner.distance(rocks[0].0) > rocks[0].1 + AI_AVOID_MARGIN);
    }

    /// Run the passive-flight system alone (state is hand-set to Patrol).
    fn run_passive(world: &mut World) {
        world.run_system_once(update_passive_flight).unwrap();
    }

    fn blocked_patrol_world() -> (World, Entity, Vec3) {
        let mut world = World::new();
        world.init_resource::<FlightSettings>();
        world.init_resource::<Time>();
        let rock_center = Vec3::new(10.0, 0.0, -200.0);
        world.spawn((Transform::from_translation(rock_center), BodyRadius(50.0)));
        let ship = world
            .spawn((
                AISpaceshipMarker,
                AIBehaviorState::Patrol,
                AIPatrolRoute::new(vec![W1, W2]),
                Transform::default(),
                LinearVelocity(Vec3::ZERO),
            ))
            .id();
        (world, ship, rock_center)
    }

    #[test]
    fn a_blocked_leg_flies_a_detour_corner() {
        let (mut world, ship, rock_center) = blocked_patrol_world();

        run_passive(&mut world);

        let corner = world
            .entity(ship)
            .get::<AIAvoidanceDetour>()
            .expect("the blocked leg holds a detour")
            .0;
        assert!(
            corner.distance(rock_center) > 50.0 + AI_AVOID_MARGIN,
            "the corner rounds the rock outside its clearance"
        );
        assert_eq!(
            world.entity(ship).get::<Autopilot>().map(|ap| ap.action),
            Some(AutopilotAction::GotoPos { position: corner }),
            "the GOTO flies the corner, not the blocked waypoint"
        );
    }

    /// An authored [`AIWaypointSlack`] moves the patrol advance gate: a ship
    /// 60 u from its mark advances under the default slack (standoff 50 +
    /// 25) but holds the leg under a tight authored slack (50 + 5) - the
    /// knob a nav-drill scene uses to press in close to its marks.
    #[test]
    fn an_authored_waypoint_slack_moves_the_advance_gate() {
        for (slack, expect_first_leg_held) in [(None, false), (Some(5.0), true)] {
            let mut world = World::new();
            world.init_resource::<FlightSettings>();
            world.init_resource::<Time>();
            let first = Vec3::new(0.0, 0.0, -60.0);
            let ship = world
                .spawn((
                    AISpaceshipMarker,
                    AIBehaviorState::Patrol,
                    AIPatrolRoute::new(vec![first, Vec3::new(0.0, 0.0, 200.0)]),
                    Transform::default(),
                    LinearVelocity(Vec3::ZERO),
                ))
                .id();
            if let Some(slack) = slack {
                world.entity_mut(ship).insert(AIWaypointSlack(slack));
            }

            run_passive(&mut world);

            let current = world
                .entity(ship)
                .get::<AIPatrolRoute>()
                .unwrap()
                .current_waypoint()
                .unwrap();
            assert_eq!(
                current == first,
                expect_first_leg_held,
                "slack {slack:?}: expected first-leg-held {expect_first_leg_held}"
            );
        }
    }

    #[test]
    fn a_held_detour_does_not_churn_the_autopilot() {
        // The corner is computed from the ship's position at block time; a
        // recompute per tick would move the goal and re-engage (churn) the
        // autopilot every frame. Sentinel phase: churn resets it to Align.
        let (mut world, ship, _) = blocked_patrol_world();

        run_passive(&mut world);
        world.entity_mut(ship).get_mut::<Autopilot>().unwrap().phase = AutopilotPhase::Burn;
        // The ship advanced a little along the detour; the corner must hold.
        world
            .entity_mut(ship)
            .get_mut::<Transform>()
            .unwrap()
            .translation = Vec3::new(-5.0, 0.0, -20.0);
        run_passive(&mut world);

        assert_eq!(
            world.entity(ship).get::<Autopilot>().map(|ap| ap.phase),
            Some(AutopilotPhase::Burn),
            "an autopilot flying a held detour is untouched"
        );
    }

    #[test]
    fn a_cleared_leg_drops_the_detour_and_resumes_the_route() {
        let (mut world, ship, rock_center) = blocked_patrol_world();
        run_passive(&mut world);
        assert!(world.entity(ship).get::<AIAvoidanceDetour>().is_some());

        // The ship rounded the rock: from abeam the far side, the direct
        // leg to W1 is clear by more than margin + hysteresis.
        world
            .entity_mut(ship)
            .get_mut::<Transform>()
            .unwrap()
            .translation = rock_center + Vec3::new(120.0, 0.0, -10.0);
        run_passive(&mut world);

        assert!(
            world.entity(ship).get::<AIAvoidanceDetour>().is_none(),
            "a comfortably clear leg drops the detour"
        );
        assert_eq!(
            world.entity(ship).get::<Autopilot>().map(|ap| ap.action),
            Some(AutopilotAction::GotoPos { position: W1 }),
            "the route leg is engaged again"
        );
    }

    #[test]
    fn a_waypoint_inside_a_bubble_is_flown_to_its_boundary() {
        // A rock scattered onto the waypoint: the ship must not fly the raw
        // point (a straight leg into the rock); it flies the waypoint pushed
        // out of the bubble, and the arrival check still runs on the raw
        // waypoint so the route turns on time.
        let mut world = World::new();
        world.init_resource::<FlightSettings>();
        world.init_resource::<Time>();
        let rock_center = W1 + Vec3::new(0.0, 30.0, 0.0);
        world.spawn((Transform::from_translation(rock_center), BodyRadius(40.0)));
        let ship = world
            .spawn((
                AISpaceshipMarker,
                AIBehaviorState::Patrol,
                AIPatrolRoute::new(vec![W1, W2]),
                Transform::default(),
                LinearVelocity(Vec3::ZERO),
            ))
            .id();

        run_passive(&mut world);

        let Some(AutopilotAction::GotoPos { position }) =
            world.entity(ship).get::<Autopilot>().map(|ap| ap.action)
        else {
            panic!("the leg is engaged");
        };
        assert_ne!(position, W1, "the raw in-bubble waypoint is never flown");
        assert!(
            position.distance(rock_center) > 40.0 + AI_AVOID_MARGIN,
            "the flown goal sits outside the rock's clearance"
        );
    }

    #[test]
    fn a_blocked_corner_leg_hops_to_a_new_corner() {
        // Rock A blocks the route leg; the corner rounding A happens to have
        // rock B sitting on ITS leg. The held corner must not be flown
        // blind - the next pass replaces it with a corner rounding B.
        let mut world = World::new();
        world.init_resource::<FlightSettings>();
        world.init_resource::<Time>();
        world.spawn((
            Transform::from_translation(Vec3::new(10.0, 0.0, -200.0)),
            BodyRadius(50.0),
        ));
        let ship = world
            .spawn((
                AISpaceshipMarker,
                AIBehaviorState::Patrol,
                AIPatrolRoute::new(vec![W1, W2]),
                Transform::default(),
                LinearVelocity(Vec3::ZERO),
            ))
            .id();
        run_passive(&mut world);
        let first = world
            .entity(ship)
            .get::<AIAvoidanceDetour>()
            .expect("rock A blocks the leg")
            .0;

        // B appears on the corner leg (a body the first pick never saw).
        world.spawn((
            Transform::from_translation(Vec3::new(-80.0, 0.0, -100.0)),
            BodyRadius(40.0),
        ));
        run_passive(&mut world);

        let hopped = world
            .entity(ship)
            .get::<AIAvoidanceDetour>()
            .expect("still detouring")
            .0;
        assert_ne!(hopped, first, "the blocked corner is replaced, not held");
        assert_eq!(
            world.entity(ship).get::<Autopilot>().map(|ap| ap.action),
            Some(AutopilotAction::GotoPos { position: hopped }),
            "the autopilot re-engages onto the hop"
        );
    }

    #[test]
    fn arriving_at_the_waypoint_clears_the_detour_with_the_leg() {
        let (mut world, ship, _) = blocked_patrol_world();
        run_passive(&mut world);
        assert!(world.entity(ship).get::<AIAvoidanceDetour>().is_some());

        // Shoved onto W1 (however it got there): the route turns onto the
        // W2 leg and the stale detour goes with the old leg.
        world
            .entity_mut(ship)
            .get_mut::<Transform>()
            .unwrap()
            .translation = W1;
        run_passive(&mut world);

        assert_eq!(
            world.entity(ship).get::<AIPatrolRoute>().unwrap().current,
            1
        );
        assert_eq!(
            world.entity(ship).get::<Autopilot>().map(|ap| ap.action),
            Some(AutopilotAction::GotoPos { position: W2 }),
            "the new leg is clear and flown directly"
        );
        assert!(
            world.entity(ship).get::<AIAvoidanceDetour>().is_none(),
            "the detour belonged to the old leg"
        );
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

    /// The physics half of avoidance: a rock dead on the first leg, and the
    /// real autopilot (align + burn + brake on real sections) must round it
    /// - the ship reaches the waypoint WITHOUT its center ever entering the
    ///
    /// The rock's geometric radius. Pins the menu "asteroid weave" backdrop's
    /// acceptance: patrol routes survive rocks the author did not measure.
    #[test]
    fn a_patrol_ship_rounds_a_rock_on_its_leg() {
        let mut app = patrol_physics_app();

        let first = Vec3::new(0.0, 0.0, -300.0);
        let second = Vec3::new(0.0, 0.0, 300.0);
        let rock_center = Vec3::new(5.0, 0.0, -150.0);
        let rock_radius = 40.0;
        app.world_mut().spawn((
            Transform::from_translation(rock_center),
            BodyRadius(rock_radius),
        ));
        let ship = spawn_patrol_ship(&mut app, vec![first, second]);

        settle(&mut app);
        app.update();
        assert_eq!(
            *app.world().get::<AIBehaviorState>(ship).unwrap(),
            AIBehaviorState::Patrol
        );

        let mut min_clearance = f32::INFINITY;
        let mut turned = false;
        for _ in 0..4800 {
            app.update();
            let position = app.world().get::<Transform>(ship).unwrap().translation;
            min_clearance = min_clearance.min(position.distance(rock_center));
            if app.world().get::<AIPatrolRoute>(ship).unwrap().current == 1 {
                turned = true;
                break;
            }
        }
        assert!(
            turned,
            "the ship must round the rock and still reach its waypoint \
             (closest approach {min_clearance:.1}u)"
        );
        assert!(
            min_clearance >= rock_radius,
            "the ship's center must never enter the rock's geometric radius \
             ({rock_radius}u), got {min_clearance:.1}u"
        );
    }

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
