//! GOTO and GotoPos legs around sized bodies: the leg the autopilot plans
//! through `flight::navigation` on the real actuator chain.
//!
//! Reproduces the straight-leg-into-rock defect: before the planner moved into
//! `autopilot_step`, a GotoPos through a body on the leg flew into it
//! (`/tmp/nova-goto-repro`, scenario `goto_through_body`). The rocks here
//! carry no collider, so a hull that flies through one does so silently and
//! the daylight census is the proof.

use avian3d::prelude::*;
use bevy::{prelude::*, time::TimeUpdateStrategy};
use nova_gameplay::{prelude::*, test_support::settle};

use super::support::*;
use crate::{flight::prediction::predict_flight_path, prelude::*};

/// The two reference arms: a skiff's and a carrier's.
const ARMS: [f32; 2] = [4.0, 18.3];

/// The leg: from the origin to a mark 300 u down -Z.
const MARK: Vec3 = Vec3::new(0.0, 0.0, -300.0);

/// A 40 u rock sitting 5 u off the leg's line, halfway down it.
const ROCK: Vec3 = Vec3::new(5.0, 0.0, -150.0);
const ROCK_RADIUS: f32 = 40.0;

/// The closest the hull's SKIN came to the rock's surface over a flight, with
/// the tick the leg released at. Panics when the leg does not release inside
/// the budget.
fn fly_until_released(app: &mut App, ship: Entity, arm: f32, rock: Vec3) -> (f32, usize) {
    let mut min_daylight = f32::INFINITY;
    for tick in 0..9600 {
        app.update();
        let com = com_of(app, ship);
        min_daylight = min_daylight.min(com.distance(rock) - ROCK_RADIUS - arm);
        if app.world().get::<Autopilot>(ship).is_none() {
            return (min_daylight, tick);
        }
    }
    panic!("the {arm}u hull must round the rock and release at its mark (closest daylight {min_daylight:.1}u)");
}

fn com_of(app: &App, ship: Entity) -> Vec3 {
    let position = app.world().get::<Position>(ship).unwrap().0;
    let rotation = app.world().get::<Rotation>(ship).unwrap();
    let com = app
        .world()
        .get::<ComputedCenterOfMass>(ship)
        .map_or(Vec3::ZERO, |com| com.0);
    rotation.mul_vec3(com) + position
}

/// A hull of `arm`, at rest at the origin, engaged on a GotoPos at [`MARK`].
fn engaged_ship(app: &mut App, arm: f32) -> Entity {
    let (ship, _, _) = spawn_ship(app);
    size_hull(app, ship, arm);
    settle(app);
    assert_eq!(
        app.world().get::<HullRadius>(ship).map(|radius| **radius),
        Some(arm),
        "the rig publishes the arm it was asked for"
    );
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::GotoPos {
            position: MARK,
        }));
    ship
}

/// The reproduced defect, held down: a body on the leg is rounded with the
/// whole hull, not flown through. Run on both reference hulls because the
/// hull arm is exactly what a margin judged against the centre line spends.
/// The flip the instruments mark is the final leg's: braking toward a corner
/// never consumes it.
#[test]
fn a_goto_leg_blocked_by_a_rock_is_flown_around_it_with_its_whole_hull() {
    for arm in ARMS {
        let mut app = flight_app();
        app.world_mut()
            .spawn((Transform::from_translation(ROCK), BodyRadius(ROCK_RADIUS)));
        let ship = engaged_ship(&mut app, arm);
        let margin = app.world().resource::<FlightSettings>().avoid_margin;
        let standoff = app.world().resource::<FlightSettings>().arrival_standoff;

        let mut min_daylight = f32::INFINITY;
        let mut detoured = false;
        let mut released = None;
        for tick in 0..9600 {
            app.update();
            let com = com_of(&app, ship);
            min_daylight = min_daylight.min(com.distance(ROCK) - ROCK_RADIUS - arm);
            match app.world().get::<Autopilot>(ship) {
                Some(autopilot) => {
                    if autopilot.detour.is_some() {
                        detoured = true;
                        assert!(
                            !autopilot.flip_marker_consumed,
                            "braking toward a corner must not spend the leg's flip"
                        );
                        assert_eq!(
                            app.world()
                                .get::<ManeuverTelemetry>(ship)
                                .map(|t| t.goal_entity),
                            Some(None),
                            "a corner is a mark of its own, not the target"
                        );
                    }
                }
                None => {
                    released = Some(tick);
                    break;
                }
            }
        }
        assert!(
            released.is_some(),
            "the {arm}u hull must round the rock and release at its mark (closest daylight \
             {min_daylight:.1}u)"
        );
        assert!(
            detoured,
            "the {arm}u hull planned a corner for the rock on its leg"
        );
        assert!(
            min_daylight >= margin,
            "the {arm}u hull's SKIN must keep the {margin}u margin off the rock, got \
             {min_daylight:.1}u"
        );
        let gap = com_of(&app, ship).distance(MARK) - arm;
        assert!(
            gap <= standoff + 6.0,
            "the {arm}u hull parks at its mark, not at a corner: gap {gap:.1}u"
        );
    }
}

/// A hull parked inside a body's margin leaves it without crossing the rock:
/// the margin is already spent, so the leg is judged on contact and the corner
/// steers the hull out before it goes around.
#[test]
fn a_hull_inside_a_rocks_margin_leaves_without_crossing_it() {
    let arm = ARMS[0];
    // 55 u off the centre of a 40 u rock: inside its 40 + 4 + 20 bubble, with
    // the mark dead behind the rock.
    let rock = Vec3::new(0.0, 0.0, -55.0);
    let mut app = flight_app();
    app.world_mut()
        .spawn((Transform::from_translation(rock), BodyRadius(ROCK_RADIUS)));
    let ship = engaged_ship(&mut app, arm);

    let (min_daylight, _) = fly_until_released(&mut app, ship, arm, rock);
    assert!(
        min_daylight > 0.0,
        "the hull never crosses the rock's surface, got {min_daylight:.1}u of daylight"
    );
    let gap = com_of(&app, ship).distance(MARK) - arm;
    let standoff = app.world().resource::<FlightSettings>().arrival_standoff;
    assert!(
        gap <= standoff + 6.0,
        "the leg still parks at its mark: gap {gap:.1}u"
    );
}

/// A nav beacon is a mark to fly to, not a rock to fly around: its published
/// size never blocks a leg, even dead on the line.
#[test]
fn a_beacon_dead_on_the_leg_is_flown_through_not_around() {
    let arm = ARMS[0];
    let beacon = Vec3::new(0.0, 0.0, -150.0);
    let mut app = flight_app();
    app.world_mut().spawn((
        Transform::from_translation(beacon),
        BodyRadius(ROCK_RADIUS),
        BeaconMarker,
    ));
    let ship = engaged_ship(&mut app, arm);
    for tick in 0..9600 {
        app.update();
        match app.world().get::<Autopilot>(ship) {
            Some(autopilot) => assert_eq!(
                autopilot.detour, None,
                "a beacon is no obstacle: no corner on tick {tick}"
            ),
            None => break,
        }
    }
    let gap = com_of(&app, ship).distance(MARK) - arm;
    let standoff = app.world().resource::<FlightSettings>().arrival_standoff;
    assert!(
        gap <= standoff + 6.0,
        "the leg parks at its mark through the beacon: gap {gap:.1}u"
    );
}

/// Cancelling the maneuver drops the corner with it, and a new engage starts
/// with no corner and replans against the live field.
#[test]
fn cancelling_a_detouring_goto_drops_the_corner_and_a_new_engage_replans() {
    let mut app = flight_app();
    app.world_mut()
        .spawn((Transform::from_translation(ROCK), BodyRadius(ROCK_RADIUS)));
    let ship = engaged_ship(&mut app, ARMS[0]);
    app.update();
    let held = app
        .world()
        .get::<Autopilot>(ship)
        .and_then(|autopilot| autopilot.detour)
        .expect("the blocked leg holds a corner after its first tick");
    assert_eq!(
        app.world().get::<ManeuverTelemetry>(ship).map(|t| t.goal),
        Some(held),
        "the published leg flies the corner"
    );

    app.world_mut().entity_mut(ship).remove::<Autopilot>();
    app.update();
    assert!(
        app.world().get::<ManeuverTelemetry>(ship).is_none(),
        "a cancelled leg publishes nothing, corner included"
    );

    let fresh = Autopilot::engage(AutopilotAction::GotoPos { position: MARK });
    assert_eq!(fresh.detour, None, "a new engage holds no corner");
    app.world_mut().entity_mut(ship).insert(fresh);
    app.update();
    assert!(
        app.world()
            .get::<Autopilot>(ship)
            .is_some_and(|autopilot| autopilot.detour.is_some()),
        "the re-engaged leg replans its corner against the rock"
    );
}

/// The prediction flies the same planner against the bodies it saw at its
/// seed, so the drawn path bends around the rock instead of running through
/// it.
#[test]
fn a_prediction_of_a_blocked_leg_bends_around_the_rock() {
    let mut app = flight_app();
    let fixed_step = app.world().resource::<Time<Fixed>>().timestep();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(fixed_step));
    app.add_systems(
        FixedPostUpdate,
        predict_flight_path.after(PhysicsSystems::Writeback),
    );
    app.world_mut()
        .spawn((Transform::from_translation(ROCK), BodyRadius(ROCK_RADIUS)));
    let arm = ARMS[0];
    let ship = engaged_ship(&mut app, arm);
    app.world_mut()
        .entity_mut(ship)
        .insert(PlayerSpaceshipMarker);
    let margin = app.world().resource::<FlightSettings>().avoid_margin;

    let mut prediction = None;
    for _ in 0..400 {
        app.update();
        if let Some(published) = app.world().get::<FlightPrediction>(ship) {
            prediction = Some(published.clone());
            break;
        }
    }
    let prediction = prediction.expect("a GotoPos on the player ship publishes a prediction");
    let closest = prediction
        .points
        .iter()
        .map(|point| point.distance(ROCK) - ROCK_RADIUS - arm)
        .fold(f32::INFINITY, f32::min);
    assert!(
        closest >= margin,
        "the predicted path keeps the margin off the rock, closest {closest:.1}u"
    );
    let bend = prediction
        .points
        .iter()
        .map(|point| Vec2::new(point.x, point.y).length())
        .fold(0.0f32, f32::max);
    assert!(
        bend > ROCK_RADIUS,
        "the predicted path leaves the straight line to round the rock, bend {bend:.1}u"
    );
}

/// A body that is not finite plans a goal that is not finite. The leg is not
/// flown: the autopilot disengages and publishes nothing.
#[test]
fn a_non_finite_body_on_the_leg_refuses_the_leg_instead_of_flying_it() {
    let mut app = flight_app();
    app.world_mut().spawn((
        Transform::from_translation(Vec3::new(f32::NAN, 0.0, -150.0)),
        BodyRadius(ROCK_RADIUS),
    ));
    let ship = engaged_ship(&mut app, ARMS[0]);
    app.update();
    assert!(
        app.world().get::<Autopilot>(ship).is_none(),
        "a leg planned against a non-finite body disengages"
    );
    assert!(
        app.world().get::<ManeuverTelemetry>(ship).is_none(),
        "and publishes no leg"
    );
    assert_eq!(
        velocity_of(&app, ship),
        Vec3::ZERO,
        "the hull did not fly the tick"
    );
}
