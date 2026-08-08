//! `ManeuverTelemetry`: the live numbers a translation leg publishes for
//! the HUD, and the teardown paths that must clear them.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_gameplay::{prelude::*, test_log::CapturedLog, test_support::settle};

use super::support::*;
use crate::{flight::state::remove_maneuver_telemetry, prelude::*};
#[test]
fn stop_publishes_its_rest_point_and_settling_clears_it() {
    let mut app = flight_app();
    let (ship, _, _) = spawn_ship(&mut app);
    settle(&mut app);
    app.world_mut()
        .entity_mut(ship)
        .insert(LinearVelocity(Vec3::new(6.0, 0.0, 0.0)));
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::Stop));
    run(&mut app, 5);

    let telemetry = app
        .world()
        .get::<ManeuverTelemetry>(ship)
        .expect("a moving STOP publishes telemetry");
    // The rest point lies ahead along the velocity.
    assert!(
        (telemetry.goal - position_of(&app, ship)).dot(Vec3::X) > 0.0,
        "rest point ahead of the drift"
    );
    assert!(telemetry.eta.expect("eta while braking") > 0.0);

    // The maneuver completes: velocity nulled, autopilot gone, and the
    // telemetry with it (observer path).
    run(&mut app, 900);
    assert!(app.world().get::<Autopilot>(ship).is_none());
    assert!(app.world().get::<ManeuverTelemetry>(ship).is_none());
}

#[test]
fn goto_publishes_telemetry_and_disengaging_clears_it() {
    let mut app = flight_app();
    let (ship, _, _) = spawn_ship(&mut app);
    settle(&mut app);
    let goal = Vec3::new(0.0, 0.0, -300.0);
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::GotoPos {
            position: goal,
        }));

    // Sample early in the burn, while the flip (which now includes the
    // rotation lead, so it comes sooner) is still ahead.
    run(&mut app, 120);
    let telemetry = app
        .world()
        .get::<ManeuverTelemetry>(ship)
        .expect("an engaged GOTO publishes telemetry");
    assert_eq!(telemetry.goal, goal);
    assert_eq!(telemetry.goal_entity, None, "GotoPos tracks no entity");
    assert!(telemetry.closing_speed > 0.5, "the ship closes on the goal");
    let flip = telemetry.flip_point.expect("flip ahead while coasting");
    // The flip point sits on the segment between ship and goal.
    let ship_position = position_of(&app, ship);
    let along = (flip - ship_position).dot(goal - ship_position);
    assert!(along > 0.0, "flip is ahead of the ship");
    assert!(
        flip.distance(goal) < ship_position.distance(goal),
        "flip is short of the goal"
    );
    assert!(telemetry.eta.expect("eta while closing") > 0.0);
    // The park point sits exactly one standoff short of the goal on
    // the closing line (GotoPos has no target radius).
    let standoff = app.world().resource::<FlightSettings>().arrival_standoff;
    assert!(
        (telemetry.park_point.distance(goal) - standoff).abs() < 1e-3,
        "the park point is one standoff short of the goal, got {}",
        telemetry.park_point.distance(goal)
    );
    assert!(
        (goal - telemetry.park_point)
            .normalize()
            .dot((goal - ship_position).normalize())
            > 0.999,
        "the park point lies on the closing line"
    );

    // Switching verbs (insert-overwrite: OnRemove does NOT fire, the
    // in-system path must carry it) republishes for the new leg: a
    // moving ship on STOP reports its predicted rest point, with no
    // flip (the retrograde alignment is inside the lead window).
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::Stop));
    run(&mut app, 2);
    let stop_telemetry = app
        .world()
        .get::<ManeuverTelemetry>(ship)
        .expect("a moving STOP leg publishes its rest point");
    assert_ne!(stop_telemetry.goal, goal, "the goal is now the rest point");
    assert_eq!(stop_telemetry.flip_point, None);
    assert_eq!(
        stop_telemetry.park_point, stop_telemetry.goal,
        "a STOP has no standoff: the rest point is the park point"
    );

    // Breakout clears the numbers with the maneuver.
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::GotoPos {
            position: goal,
        }));
    run(&mut app, 60);
    assert!(app.world().get::<ManeuverTelemetry>(ship).is_some());
    app.world_mut().entity_mut(ship).remove::<Autopilot>();
    run(&mut app, 2);
    assert!(
        app.world().get::<ManeuverTelemetry>(ship).is_none(),
        "telemetry dies with the leg"
    );
}

/// The playtest warn: scenario teardown despawns a ship with an engaged
/// autopilot, `On<Remove, Autopilot>` fires mid-flush, and the remove it queues
/// lands after the despawn in the same queue - "Encountered an error in command
/// ... Entity despawned". The test drives that exact path (a QUEUED despawn; a
/// direct `World::despawn` does not reproduce it - the entity is already gone
/// at observer-queue time, `get_entity` bails, nothing is queued) and asserts
/// the warn does not fire, with two delivery guards: the observer's command
/// demonstrably lands on a live disengage, and the log capture demonstrably
/// sees this exact warn class.
#[test]
fn despawning_an_autopiloting_ship_queues_no_stale_telemetry_command() {
    use bevy::log::tracing_subscriber::{self, util::SubscriberInitExt};

    let log = CapturedLog::default();
    let writer = log.clone();
    let _guard = tracing_subscriber::fmt()
        .with_writer(move || writer.clone())
        .set_default();

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_observer(remove_maneuver_telemetry);

    let telemetry = ManeuverTelemetry {
        goal: Vec3::new(0.0, 0.0, -300.0),
        goal_entity: None,
        park_point: Vec3::new(0.0, 0.0, -290.0),
        distance: 300.0,
        closing_speed: 5.0,
        brake_accel: 1.0,
        flip_point: None,
        seconds_to_flip: None,
        eta: Some(60.0),
    };

    // Delivery guard 1: the capture sees exactly this warn class - a
    // deliberately stale plain `remove` must log "Entity despawned".
    let stale = app.world_mut().spawn_empty().id();
    app.world_mut().entity_mut(stale).despawn();
    app.world_mut()
        .commands()
        .entity(stale)
        .remove::<ManeuverTelemetry>();
    app.update();
    assert!(
        log.contents().contains("Entity despawned"),
        "the log capture must see a deliberate stale-command warn; got: {}",
        log.contents()
    );
    log.clear();

    // Delivery guard 2: on a LIVE ship the observer fires and its queued
    // command really lands.
    let live = app
        .world_mut()
        .spawn((
            SpaceshipRootMarker,
            Autopilot::engage(AutopilotAction::Stop),
            telemetry,
        ))
        .id();
    app.update();
    app.world_mut().entity_mut(live).remove::<Autopilot>();
    app.update();
    assert!(
        app.world().get::<ManeuverTelemetry>(live).is_none(),
        "the observer clears telemetry on a live disengage"
    );

    // The race: despawn the ship WITH the autopilot engaged, through a
    // QUEUED despawn, the way the unload sweep does. Pre-fix the
    // observer's remove lands on the despawned ship and warns.
    let doomed = app
        .world_mut()
        .spawn((
            SpaceshipRootMarker,
            Autopilot::engage(AutopilotAction::Stop),
            telemetry,
        ))
        .id();
    app.update();
    log.clear();
    app.world_mut().commands().entity(doomed).despawn();
    app.update();
    assert!(
        !log.contents().contains("Entity despawned"),
        "teardown must not race a stale telemetry remove; got: {}",
        log.contents()
    );
}
