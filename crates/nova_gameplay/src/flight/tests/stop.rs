//! The STOP verb: flip retrograde, brake to rest, and release without
//! chasing the crumbs it is allowed to accept.

use avian3d::prelude::*;
use bevy::prelude::*;

use super::support::*;
use crate::{integrity::test_support::settle, prelude::*};

#[test]
fn stop_flips_the_hull_and_kills_velocity_with_no_external_force() {
    let mut app = flight_app();
    let (ship, _, _) = spawn_ship(&mut app);
    withhold_rcs(&mut app, ship);
    settle(&mut app);
    // Coasting sideways: the nose (-Z) must physically swing ~90 degrees
    // to point retrograde before the drive can brake anything.
    app.world_mut()
        .entity_mut(ship)
        .insert(LinearVelocity(Vec3::new(6.0, 0.0, 0.0)));
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::Stop));

    run(&mut app, 900);

    let speed = velocity_of(&app, ship).length();
    assert!(speed < 0.5, "STOP should null the velocity, got {speed}");
    assert!(
        app.world().get::<Autopilot>(ship).is_none(),
        "a completed maneuver disengages"
    );
}

/// Reproduction attempt for the in-game report "autopilot rotates but never
/// thrusts": build the ship EXACTLY like the scenario does (base_section + kind
/// bundles, real config values from nova_assets/sections.rs) instead of the
/// hand-rolled test sections, and diagnose the thruster-query conditions
/// directly.
#[test]
fn scratch_scenario_built_ship_autopilot_thrusts() {
    let mut app = flight_app();
    let ship = app
        .world_mut()
        .spawn((
            RigidBody::Dynamic,
            Transform::default(),
            Visibility::Visible,
            SpaceshipRootMarker,
            FlightIntent::default(),
        ))
        .id();
    let base = |id: &str| BaseSectionConfig {
        id: id.to_string(),
        name: id.to_string(),
        description: String::new(),
        mass: 1.0,
        health: 100.0,
        ..default()
    };
    let controller = app
        .world_mut()
        .spawn((
            ChildOf(ship),
            base_section(base("controller")),
            controller_section(ControllerSectionConfig {
                frequency: 4.0,
                damping_ratio: 4.0,
                max_torque: 40.0,
                render_mesh: None,
                ..default()
            }),
            Transform::default(),
        ))
        .id();
    // The real game wires PDControllerTarget via the section observer;
    // mirror it manually like the other tests do.
    app.world_mut()
        .entity_mut(controller)
        .insert(PDControllerTarget(ship));
    let thruster = app
        .world_mut()
        .spawn((
            ChildOf(ship),
            base_section(base("thruster")),
            thruster_section(ThrusterSectionConfig {
                magnitude: 1.0,
                render_mesh: None,
                ..default()
            }),
            Transform::from_translation(Vec3::new(0.0, 0.0, 2.0)),
        ))
        .id();
    settle(&mut app);
    withhold_rcs(&mut app, ship);

    // Diagnose the exact conditions the autopilot's thruster query needs.
    println!(
        "thruster: rotation={:?} binding={} inactive={} marker={} magnitude={:?} childof={:?} ship={ship:?}",
        app.world().get::<Rotation>(thruster),
        app.world()
            .get::<SpaceshipThrusterInputBinding>(thruster)
            .is_some(),
        app.world().get::<SectionInactiveMarker>(thruster).is_some(),
        app.world().get::<ThrusterSectionMarker>(thruster).is_some(),
        app.world()
            .get::<ThrusterSectionMagnitude>(thruster)
            .map(|m| **m),
        app.world().get::<ChildOf>(thruster),
    );

    app.world_mut()
        .entity_mut(ship)
        .insert(LinearVelocity(Vec3::new(6.0, 0.0, 0.0)));
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::Stop));
    run(&mut app, 60);
    println!(
        "after 60 ticks: thruster_input={:?} velocity={:?} ap={:?}",
        app.world()
            .get::<ThrusterSectionInput>(thruster)
            .map(|i| **i),
        app.world().get::<LinearVelocity>(ship).map(|v| **v),
        app.world().get::<Autopilot>(ship),
    );
    run(&mut app, 840);

    let speed = app
        .world()
        .get::<LinearVelocity>(ship)
        .map(|v| v.length())
        .unwrap_or(f32::NAN);
    assert!(
        speed < 0.5,
        "scenario-built ship STOP should reach rest, got {speed}"
    );
}

/// The high-speed flip regression: braking from a hard burn used to
/// leave the PD limit-cycling (its torque clamp swamps the damping term
/// on a 180 setpoint). With the slewed command the maneuver completes
/// and the hull is parked - no residual tumble.
#[test]
fn high_speed_stop_settles_without_tumbling() {
    let mut app = flight_app();
    let (ship, _, _) = spawn_ship(&mut app);
    withhold_rcs(&mut app, ship);
    settle(&mut app);
    app.world_mut()
        .entity_mut(ship)
        .insert(LinearVelocity(Vec3::new(0.0, 0.0, -60.0)));
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::Stop));

    run(&mut app, 3000);

    let speed = velocity_of(&app, ship).length();
    assert!(
        speed < 0.5,
        "high-speed STOP should reach rest, got {speed}"
    );
    assert!(app.world().get::<Autopilot>(ship).is_none());
    // Mid-maneuver the slewed command keeps the hull steady (the old
    // wobble hit 2+ rad/s DURING the burn), and since bcs's inertia
    // frame composition fix the release parks the hull too - the old
    // ~1.5 rad/s corkscrew came from the mangled tensor (avian's eigen
    // sort hands even this axis-aligned ship a cyclic-permutation
    // local frame, which the pre-fix order composed wrongly).
    run(&mut app, 300);
    let spin = app
        .world()
        .get::<AngularVelocity>(ship)
        .map(|w| w.length())
        .unwrap_or(f32::NAN);
    assert!(
        spin < 0.5,
        "post-release residual spin regressed: {spin} rad/s"
    );
}

/// A retro-equipped ship must brake a small overspeed with the engine
/// already pointing the right way - zero hull rotation, no flip.
#[test]
fn retro_group_brakes_a_small_overspeed_without_flipping() {
    let mut app = flight_app();
    let (ship, _, _) = spawn_ship(&mut app);
    withhold_rcs(&mut app, ship);
    spawn_extra_thruster(
        &mut app,
        ship,
        0.25,
        Quat::from_rotation_y(std::f32::consts::PI),
    );
    settle(&mut app);
    app.world_mut()
        .entity_mut(ship)
        .insert(LinearVelocity(Vec3::new(0.0, 0.0, -2.0)));
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::Stop));

    run(&mut app, 600);

    let speed = velocity_of(&app, ship).length();
    assert!(speed < 0.5, "retro should brake to rest, got {speed}");
    assert!(
        app.world().get::<Autopilot>(ship).is_none(),
        "completed maneuver disengages"
    );
    let forward = forward_of(&app, ship);
    assert!(
        forward.dot(Vec3::NEG_Z) > 0.95,
        "a retro brake must not flip the hull, forward now {forward}"
    );
}

/// For a big burn the math flips: rotating the strong main drive around
/// beats a long slow burn on the little retro (the rotation-bias knob).
#[test]
fn large_burn_still_flips_to_the_main_drive() {
    let mut app = flight_app();
    let (ship, _, _) = spawn_ship(&mut app);
    withhold_rcs(&mut app, ship);
    spawn_extra_thruster(
        &mut app,
        ship,
        0.25,
        Quat::from_rotation_y(std::f32::consts::PI),
    );
    settle(&mut app);
    app.world_mut()
        .entity_mut(ship)
        .insert(LinearVelocity(Vec3::new(0.0, 0.0, -30.0)));
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::Stop));

    run(&mut app, 1800);

    let speed = velocity_of(&app, ship).length();
    assert!(speed < 0.5, "STOP should reach rest, got {speed}");
    let forward = forward_of(&app, ship);
    assert!(
        forward.dot(Vec3::NEG_Z) < 0.5,
        "a large brake should have swung the hull off the nose line, forward {forward}"
    );
}

/// Inside the deadband nothing rotates - but an engine that already
/// points at the crumb kills it instead of the residual being accepted.
#[test]
fn side_thruster_kills_a_lateral_crumb_in_the_deadband() {
    let mut app = flight_app();
    let (ship, _, _) = spawn_ship(&mut app);
    withhold_rcs(&mut app, ship);
    // Thrust toward -X (local -Z rotated +90 degrees about Y).
    spawn_extra_thruster(
        &mut app,
        ship,
        0.25,
        Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
    );
    settle(&mut app);
    app.world_mut()
        .entity_mut(ship)
        .insert(LinearVelocity(Vec3::new(0.3, 0.0, 0.0)));
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::Stop));

    run(&mut app, 300);

    let speed = velocity_of(&app, ship).length();
    assert!(
        speed < 0.25,
        "the side engine should kill the crumb, got {speed}"
    );
    assert!(app.world().get::<Autopilot>(ship).is_none());
    let forward = forward_of(&app, ship);
    assert!(
        forward.dot(Vec3::NEG_Z) > 0.95,
        "no rotation inside the deadband, forward now {forward}"
    );
}

/// Destroying the retro removes its group: the ship falls back to the
/// flip-and-burn it would have needed anyway.
#[test]
fn a_dead_retro_falls_back_to_the_flip() {
    let mut app = flight_app();
    let (ship, _, _) = spawn_ship(&mut app);
    withhold_rcs(&mut app, ship);
    let retro = spawn_extra_thruster(
        &mut app,
        ship,
        0.25,
        Quat::from_rotation_y(std::f32::consts::PI),
    );
    settle(&mut app);
    app.world_mut()
        .entity_mut(retro)
        .insert(SectionInactiveMarker);
    app.world_mut()
        .entity_mut(ship)
        .insert(LinearVelocity(Vec3::new(0.0, 0.0, -2.0)));
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::Stop));

    run(&mut app, 900);

    let speed = velocity_of(&app, ship).length();
    assert!(
        speed < 0.5,
        "main-drive fallback should still stop, got {speed}"
    );
    // The exact parking attitude wanders with the endgame crumbs; what
    // matters is that stopping required leaving the original facing (the
    // retro would have braked without turning at all).
    let forward = forward_of(&app, ship);
    assert!(
        forward.dot(Vec3::NEG_Z) < 0.5,
        "without the retro the hull must have turned away to brake, forward {forward}"
    );
}

/// The twitch fix: a residual drift below the attitude deadband, with the
/// nose nowhere near the retro direction, is a crumb - the autopilot must
/// accept it and let go instead of pirouetting the hull to chase it.
#[test]
fn stop_accepts_a_crumb_without_pirouetting() {
    let mut app = flight_app();
    let (ship, _, _) = spawn_ship(&mut app);
    withhold_rcs(&mut app, ship);
    settle(&mut app);
    // Slow lateral creep, below the deadband; killing it would need a
    // ~90 degree pirouette.
    app.world_mut()
        .entity_mut(ship)
        .insert(LinearVelocity(Vec3::new(0.3, 0.0, 0.0)));
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::Stop));

    run(&mut app, 120);

    assert!(
        app.world().get::<Autopilot>(ship).is_none(),
        "a crumb residual must be accepted, not chased"
    );
    let forward = app
        .world()
        .get::<Rotation>(ship)
        .unwrap()
        .mul_vec3(Vec3::NEG_Z);
    assert!(
        forward.dot(Vec3::NEG_Z) > 0.98,
        "the hull must not pirouette for a crumb, forward now {forward}"
    );
    let v = velocity_of(&app, ship);
    assert!(
        (v - Vec3::new(0.3, 0.0, 0.0)).length() < 0.05,
        "the crumb is accepted as-is, got {v}"
    );
}
