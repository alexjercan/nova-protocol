//! The MATCH VELOCITY action: hold a commanded world velocity, hold a
//! requested facing while doing it, and never report itself finished.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_gameplay::test_support::settle;

use super::support::*;
use crate::prelude::*;

/// How close to the commanded velocity the hull has to get before the test
/// calls it matched, world units per second. The generic rig's single drive
/// is weak, so this is a band the burn can actually reach inside the frame
/// budget rather than a promise about the terminal residual.
const MATCHED: f32 = 0.4;

#[test]
fn a_commanded_velocity_is_flown_to_and_then_held() {
    let mut app = flight_app();
    let (ship, _, _) = spawn_ship(&mut app);
    withhold_rcs(&mut app, ship);
    settle(&mut app);

    let commanded = Vec3::new(0.0, 0.0, -3.0);
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::MatchVelocity {
            velocity: commanded,
            facing: None,
        }));

    run(&mut app, 900);

    let error = (velocity_of(&app, ship) - commanded).length();
    assert!(
        error < MATCHED,
        "the hull should be flying the commanded velocity, {error} u/s off it"
    );

    // And it STAYS there: a held velocity is not a leg that finishes and hands
    // the ship back drifting.
    run(&mut app, 300);
    let error = (velocity_of(&app, ship) - commanded).length();
    assert!(
        error < MATCHED,
        "the hull should still be holding the commanded velocity, {error} u/s off it"
    );
}

#[test]
fn a_held_velocity_never_completes_itself() {
    let mut app = flight_app();
    let (ship, _, _) = spawn_ship(&mut app);
    withhold_rcs(&mut app, ship);
    settle(&mut app);

    // The easiest possible goal: hold the rest the ship is already at. A leg
    // that ends at rest would disengage here on its first tick.
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::MatchVelocity {
            velocity: Vec3::ZERO,
            facing: None,
        }));

    run(&mut app, 600);

    assert!(
        app.world().get::<Autopilot>(ship).is_some(),
        "a held velocity is a state, not an errand: it holds until replaced"
    );
    assert!(
        app.world().get::<PlayerAutopilotCompleted>(ship).is_none(),
        "nothing completed, so nothing should be reported as completed"
    );
    assert_eq!(
        app.world().get::<Autopilot>(ship).map(|a| a.phase),
        Some(AutopilotPhase::Hold),
        "a hull sitting on its commanded velocity is holding it, not aligning"
    );
}

#[test]
fn a_held_velocity_publishes_no_leg_for_the_instruments() {
    let mut app = flight_app();
    let (ship, _, _) = spawn_ship(&mut app);
    withhold_rcs(&mut app, ship);
    settle(&mut app);

    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::MatchVelocity {
            velocity: Vec3::new(2.0, 0.0, 0.0),
            facing: None,
        }));

    run(&mut app, 60);

    assert!(
        app.world().get::<ManeuverTelemetry>(ship).is_none(),
        "there is no destination, so there is no leg to draw"
    );
}

#[test]
fn the_nose_goes_where_it_was_asked_once_the_velocity_is_held() {
    let mut app = flight_app();
    let (ship, _, _) = spawn_ship(&mut app);
    withhold_rcs(&mut app, ship);
    settle(&mut app);

    // Already on the commanded velocity, so the drive has nothing to ask the
    // hull for: the facing is the only thing left to fly.
    let commanded = Vec3::new(0.0, 0.0, -2.0);
    app.world_mut()
        .entity_mut(ship)
        .insert(LinearVelocity(commanded));
    let facing = Dir3::X;
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::MatchVelocity {
            velocity: commanded,
            facing: Some(facing),
        }));

    run(&mut app, 900);

    let nose = forward_of(&app, ship);
    let off = nose.angle_between(*facing).to_degrees();
    assert!(
        off < 10.0,
        "the nose should be on the requested facing, {off} deg off it"
    );
}

#[test]
fn a_velocity_error_worth_turning_for_takes_the_hull_off_its_facing() {
    let mut app = flight_app();
    let (ship, _, _) = spawn_ship(&mut app);
    withhold_rcs(&mut app, ship);
    settle(&mut app);

    // The facing and the burn are 90 degrees apart, and the ship has one
    // drive: it cannot have both, and the motion is what it was asked for.
    // The facing is a request, and this is the case that outranks it.
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::MatchVelocity {
            velocity: Vec3::new(0.0, 0.0, -3.0),
            facing: Some(Dir3::X),
        }));

    run(&mut app, 400);

    let closing = velocity_of(&app, ship).dot(Vec3::NEG_Z);
    assert!(
        closing > 1.0,
        "the burn owns the hull while the error is large; the ship made only \
         {closing} u/s of the velocity it was commanded"
    );
}
