//! The GOTO and GotoPos verbs: the arrival curve, the surface-relative
//! standoff, the ORBIT handoff at a well body, and a quiet arrival.

use avian3d::prelude::*;
use bevy::prelude::*;

use super::support::*;
use crate::{integrity::test_support::settle, prelude::*};

#[test]
fn goto_arrives_at_standoff_and_disengages() {
    let mut app = flight_app();
    let (ship, _, _) = spawn_ship(&mut app);
    let target = app
        .world_mut()
        .spawn((
            Transform::from_xyz(0.0, 0.0, -300.0),
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, -300.0)),
        ))
        .id();
    settle(&mut app);
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::Goto { target }));

    // Run until the autopilot releases the ship (the slewed command
    // makes the terminal creep slower), then assert AT that moment -
    // the accepted below-deadband crumb may slowly drift the parked
    // ship afterwards, which is the twitch-fix tradeoff, not a missed
    // arrival.
    let mut released_at = None;
    for tick in 0..4800 {
        app.update();
        if app.world().get::<Autopilot>(ship).is_none() {
            released_at = Some(tick);
            break;
        }
    }
    assert!(
        released_at.is_some(),
        "GOTO must complete and disengage within the budget"
    );

    let standoff = app.world().resource::<FlightSettings>().arrival_standoff;
    let pos = app.world().get::<Position>(ship).unwrap().0;
    let distance = (Vec3::new(0.0, 0.0, -300.0) - pos).length();
    let speed = velocity_of(&app, ship).length();
    assert!(
        distance <= standoff + 6.0 && distance >= standoff - 45.0,
        "should arrive near the {standoff}u standoff, got {distance}"
    );
    assert!(speed < 0.5, "should arrive at rest, got {speed}");
}

#[test]
fn goto_pos_arrives_at_standoff_and_disengages() {
    // The position-goal twin of the entity GOTO (the AI patrol leg):
    // same arrival rule, no entity to track.
    let mut app = flight_app();
    let (ship, _, _) = spawn_ship(&mut app);
    settle(&mut app);
    let destination = Vec3::new(0.0, 0.0, -300.0);
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::GotoPos {
            position: destination,
        }));

    let mut released = false;
    for _ in 0..4800 {
        app.update();
        if app.world().get::<Autopilot>(ship).is_none() {
            released = true;
            break;
        }
    }
    assert!(released, "GotoPos must complete and disengage in budget");

    let standoff = app.world().resource::<FlightSettings>().arrival_standoff;
    let pos = app.world().get::<Position>(ship).unwrap().0;
    let distance = (destination - pos).length();
    let speed = velocity_of(&app, ship).length();
    assert!(
        distance <= standoff + 6.0 && distance >= standoff - 45.0,
        "should arrive near the {standoff}u standoff, got {distance}"
    );
    assert!(speed < 0.5, "should arrive at rest, got {speed}");
}

#[test]
fn goto_into_a_well_stops_at_the_standoff_instead_of_crashing() {
    // The playtest crash: GOTO a well body from outside the SOI at speed. A
    // gravity-blind plan flips on the vacuum curve, the well keeps feeding
    // speed through the descent, and the ship punches through the standoff into
    // the surface. The gravity-aware plan must keep the hull outside the body
    // the whole way and still park at the standoff.
    let mut app = orbit_app();
    let gravity = GravitySettings::default();
    // The strongest well the guardrail allows: surface pull 5 u/s^2 on
    // a 40u body (mu = 8000, SOI 320u).
    let well = app
        .world_mut()
        .spawn((
            RigidBody::Static,
            Transform::default(),
            crate::gravity::GravityWell::from_surface_gravity(5.0, 40.0, &gravity),
        ))
        .id();
    let (ship, _, _) = spawn_ship(&mut app);
    withhold_rcs(&mut app, ship);
    app.world_mut()
        .entity_mut(ship)
        .insert(Transform::from_xyz(0.0, 0.0, 500.0));
    settle(&mut app);
    app.world_mut()
        .entity_mut(ship)
        .insert(LinearVelocity(Vec3::new(0.0, 0.0, -25.0)));
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::Goto { target: well }));

    let body_radius = 40.0;
    let mut min_distance = f32::MAX;
    let mut parked = false;
    for _ in 0..6000 {
        app.update();
        let distance = app.world().get::<Position>(ship).unwrap().0.length();
        min_distance = min_distance.min(distance);
        match app.world().get::<Autopilot>(ship) {
            // A GOTO at a well body hands off to ORBIT instead of releasing.
            Some(autopilot) if matches!(autopilot.action, AutopilotAction::Orbit { .. }) => {
                parked = true;
                break;
            }
            Some(_) => {}
            None => panic!("a GOTO at a well body must park, not release"),
        }
    }
    assert!(parked, "GOTO must arrive and hand off to ORBIT in budget");
    // The done gate structurally guarantees the handoff residual is bounded by
    // the settle band (sub-band crumbs are accepted rather than hunted); assert
    // that contract to keep the old arrival-curve check.
    let handoff_speed = velocity_of(&app, ship).length();
    let settle_band = app.world().resource::<FlightSettings>().settle_deadband;
    assert!(
        handoff_speed < settle_band + 0.05,
        "the handoff happens within the settle band ({settle_band}), got {handoff_speed}"
    );
    assert!(
        min_distance > body_radius + gravity.surface_margin,
        "the hull must never dip below the surface, got {min_distance}"
    );
    // The handoff happens at the surface-relative park point: standoff +
    // body_radius from the center, with the flat-space tests' terminal-creep
    // lower bound.
    let standoff = app.world().resource::<FlightSettings>().arrival_standoff;
    let park = standoff + body_radius;
    let distance = app.world().get::<Position>(ship).unwrap().0.length();
    assert!(
        distance <= park + 6.0 && distance >= park - 45.0,
        "should hand off near {park}u from the center ({standoff}u above the \
         surface), got {distance}"
    );

    // ORBIT never completes: the computer station-keeps. Run on and
    // require the ship to stay engaged and above the surface while
    // the insertion pulls it onto the ring.
    for _ in 0..1200 {
        app.update();
        let distance = app.world().get::<Position>(ship).unwrap().0.length();
        assert!(
            distance > body_radius + gravity.surface_margin,
            "the parked orbit must keep the hull above the surface, got {distance}"
        );
    }
    let autopilot = app
        .world()
        .get::<Autopilot>(ship)
        .expect("ORBIT station-keeps; it never completes");
    let AutopilotAction::Orbit {
        plan: Some(plan), ..
    } = autopilot.action
    else {
        panic!("the parked autopilot flies a planned orbit");
    };
    // The insertion actually holds: after the settle window the ship
    // is on (or tight around) the planned ring, not slowly decaying
    // past it.
    let radius = app.world().get::<Position>(ship).unwrap().0.length();
    assert!(
        (radius - plan.radius).abs() < 15.0,
        "the ship should ride the {}u ring, got {radius}",
        plan.radius
    );
}

#[test]
fn goto_standoff_is_surface_relative_for_sized_targets() {
    // A big rock WITHOUT a well: the authored BodyRadius alone must
    // push the park point out, and the published telemetry distance
    // must read to the surface, not the center (the chip should never
    // say "50" while hovering over a mountain).
    let mut app = flight_app();
    let (ship, _, _) = spawn_ship(&mut app);
    let center = Vec3::new(0.0, 0.0, -300.0);
    let target = app
        .world_mut()
        .spawn((
            Transform::from_translation(center),
            GlobalTransform::from(Transform::from_translation(center)),
            BodyRadius(30.0),
        ))
        .id();
    settle(&mut app);
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::Goto { target }));

    // Mid-leg the telemetry distance is surface-relative: center
    // distance minus the radius.
    app.update();
    let telemetry = app
        .world()
        .get::<ManeuverTelemetry>(ship)
        .expect("engaged GOTO publishes telemetry");
    let center_distance = (center - app.world().get::<Position>(ship).unwrap().0).length();
    assert!(
        (telemetry.distance - (center_distance - 30.0)).abs() < 1.0,
        "telemetry reads to the surface: got {} for center distance {center_distance}",
        telemetry.distance
    );
    // The published park point budgets the radius too: standoff plus
    // radius from the center, on the closing line (the ribbon terminates
    // here).
    let standoff = app.world().resource::<FlightSettings>().arrival_standoff;
    assert!(
        (telemetry.park_point.distance(center) - (standoff + 30.0)).abs() < 1e-2,
        "the park point sits standoff + radius from the center, got {}",
        telemetry.park_point.distance(center)
    );

    let mut released = false;
    // Once inside the park envelope the park point degenerates to the
    // ship itself: the computer stops where it is, it never plans a
    // leg back out to the boundary - and the ribbon must not draw one.
    let mut inside_sample: Option<(Vec3, Vec3)> = None;
    for _ in 0..4800 {
        app.update();
        if let Some(numbers) = app.world().get::<ManeuverTelemetry>(ship) {
            if inside_sample.is_none() && numbers.distance <= standoff {
                inside_sample = Some((
                    numbers.park_point,
                    app.world().get::<Position>(ship).unwrap().0,
                ));
            }
        }
        if app.world().get::<Autopilot>(ship).is_none() {
            released = true;
            break;
        }
    }
    assert!(released, "GOTO must complete and disengage in budget");
    let (inside_park, inside_position) =
        inside_sample.expect("the leg passes through the park envelope before release");
    assert!(
        inside_park.distance(inside_position) < 2.0,
        "inside the envelope the park point pins to the ship, got {}u away",
        inside_park.distance(inside_position)
    );

    let park = standoff + 30.0;
    let distance = (center - app.world().get::<Position>(ship).unwrap().0).length();
    let speed = velocity_of(&app, ship).length();
    assert!(
        distance <= park + 6.0 && distance >= park - 45.0,
        "should park near {park}u from the center, got {distance}"
    );
    assert!(speed < 0.5, "should arrive at rest, got {speed}");
}

#[test]
fn handoff_ring_clears_the_geometric_radius() {
    // A well whose real collider (BodyRadius 70) reaches far past its
    // nominal physics radius (40): the parking handoff must ring at
    // the GEOMETRIC park radius (70 + 50 = 120), not clamp the crept
    // position against a band floored on the nominal sphere - that
    // ring could sit inside the actual rock.
    let mut app = orbit_app();
    let gravity = GravitySettings::default();
    let well = app
        .world_mut()
        .spawn((
            RigidBody::Static,
            Transform::default(),
            crate::gravity::GravityWell::from_surface_gravity(5.0, 40.0, &gravity),
            BodyRadius(70.0),
        ))
        .id();
    let (ship, _, _) = spawn_ship(&mut app);
    // At rest inside the park envelope: surface distance 115 - 70 =
    // 45 <= the 50u standoff, so the leg is immediately done and the
    // handoff fires.
    app.world_mut()
        .entity_mut(ship)
        .insert(Transform::from_xyz(0.0, 0.0, 115.0));
    settle(&mut app);
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::Goto { target: well }));

    let mut plan_radius = None;
    for _ in 0..600 {
        app.update();
        match app.world().get::<Autopilot>(ship) {
            Some(autopilot) => {
                if let AutopilotAction::Orbit {
                    plan: Some(plan), ..
                } = autopilot.action
                {
                    plan_radius = Some(plan.radius);
                    break;
                }
            }
            None => panic!("a GOTO at a well body must park, not release"),
        }
    }
    let radius = plan_radius.expect("handoff in budget");
    assert!(
        (radius - 120.0).abs() < 2.0,
        "ring at the geometric park radius, got {radius}"
    );
}

#[test]
fn goto_radius_resolution_prefers_the_larger_source() {
    // A target carrying BOTH an authored BodyRadius and a well whose
    // body_radius disagrees: the arrival must budget the larger of
    // the two (conservative if they ever drift apart).
    let mut app = flight_app();
    let (ship, _, _) = spawn_ship(&mut app);
    let center = Vec3::new(0.0, 0.0, -300.0);
    let gravity = GravitySettings::default();
    let target = app
        .world_mut()
        .spawn((
            Transform::from_translation(center),
            GlobalTransform::from(Transform::from_translation(center)),
            Position(center),
            BodyRadius(20.0),
            crate::gravity::GravityWell::from_surface_gravity(3.0, 40.0, &gravity),
        ))
        .id();
    settle(&mut app);
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::Goto { target }));

    app.update();
    let telemetry = app
        .world()
        .get::<ManeuverTelemetry>(ship)
        .expect("engaged GOTO publishes telemetry");
    let center_distance = (center - app.world().get::<Position>(ship).unwrap().0).length();
    assert!(
        (telemetry.distance - (center_distance - 40.0)).abs() < 1.0,
        "the larger well radius wins over BodyRadius(20): got {} for center distance \
         {center_distance}",
        telemetry.distance
    );
}

#[test]
fn goto_disengages_when_the_target_is_gone() {
    let mut app = flight_app();
    let (ship, _, _) = spawn_ship(&mut app);
    let target = app
        .world_mut()
        .spawn((
            Transform::from_xyz(0.0, 0.0, -300.0),
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, -300.0)),
        ))
        .id();
    settle(&mut app);
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::Goto { target }));
    run(&mut app, 5);

    app.world_mut().entity_mut(target).despawn();
    run(&mut app, 2);

    assert!(
        app.world().get::<Autopilot>(ship).is_none(),
        "a vanished destination disengages the autopilot"
    );
}

/// A GOTO leg must ARRIVE quietly. The feel-filtering work traced the playtest
/// "wobbles on GOTO" to a terminal attitude hunt: the endgame of a translation
/// leg lives in sub-u/s velocity errors, and
/// with the tight crumb band (attitude_deadband 0.4) plus its 8x
/// urgency denominator the computer chased them with visible attitude
/// swings (~0.6 rad/s) for seconds at every arrival - while STOP,
/// whose error passes the band exactly once nose-on, settled
/// perfectly. With the settle band (and the urgency denominator it
/// carries) scoped to rest legs, the terminal phase stays under
/// 0.15 rad/s and the hull releases essentially still. A/B: the
/// pre-fix config fails this at ~0.6 rad/s terminal spin.
///
/// Wiring history: the arrival dynamics are wiring-SENSITIVE. Under the older
/// Update-schedule command copy, the doorstep brake's spool-tail overshoot
/// happened to land under the settle band (accidental dither); the same-tick
/// handoff phase-locked it into a boundary-bounce limit cycle until the
/// spool-tail cutoff in autopilot_system removed the overshoot at its source.
/// This rig runs the harness wiring (same-tick copy), which matches production
/// now that the shipped copy runs in FixedUpdate - so a hunt reappearing under
/// EITHER the cutoff regressing or the wiring changing fails here.
#[test]
fn goto_arrival_settles_without_hunting() {
    let mut app = flight_app();
    let (ship, _controller) = diag_ship(&mut app);
    let goal = Vec3::new(300.0, 0.0, -600.0);
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::GotoPos {
            position: goal,
        }));
    let standoff = app.world().resource::<FlightSettings>().arrival_standoff;

    let mut max_spin_overall = 0.0f32;
    let mut max_spin_terminal = 0.0f32;
    let mut min_remaining = f32::MAX;
    let mut done = false;
    for _ in 0..4000 {
        app.update();
        let spin = app.world().get::<AngularVelocity>(ship).unwrap().length();
        let remaining = (goal - position_of(&app, ship)).length() - standoff;
        max_spin_overall = max_spin_overall.max(spin);
        min_remaining = min_remaining.min(remaining);
        if remaining < 15.0 {
            max_spin_terminal = max_spin_terminal.max(spin);
        }
        if app.world().get::<Autopilot>(ship).is_none() {
            done = true;
            break;
        }
    }

    // Delivery guards: the maneuver must actually have flown - a leg
    // that never engages, never flips, or never reaches the envelope
    // would pass the quiet-arrival bounds vacuously.
    assert!(done, "the GotoPos leg must complete and release in budget");
    assert!(
        min_remaining < 1.0,
        "the ship must actually reach the park envelope, got to {min_remaining}"
    );
    assert!(
        max_spin_overall > 0.5,
        "a real flip-and-burn must have happened (max spin {max_spin_overall})"
    );

    let release_spin = app.world().get::<AngularVelocity>(ship).unwrap().length();
    assert!(
        max_spin_terminal < 0.15,
        "the arrival must not hunt: terminal max spin {max_spin_terminal} rad/s"
    );
    assert!(
        release_spin < 0.1,
        "the hull must release still, not mid-swing: {release_spin} rad/s"
    );
}
