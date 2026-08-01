//! RCS fine-adjustment: the capped, torque-free COM push both the pilot
//! and the autopilot's terminal settle drive.

use avian3d::prelude::*;
use bevy::prelude::*;

use super::support::*;
use crate::{
    flight::{accumulate_rcs_axis, RcsReference},
    integrity::test_support::settle,
    prelude::*,
};

/// The virtual-joystick accumulator integrates the held offset and clamps it to
/// the unit range the primitive expects, so a sustained push saturates at 1 and
/// pulling back walks it toward the other rail rather than running away.
#[test]
fn accumulate_rcs_axis_integrates_and_clamps_to_the_unit_range() {
    // Integration accumulates across calls (held-direction persistence).
    let a = accumulate_rcs_axis(0.0, 0.3);
    let b = accumulate_rcs_axis(a, 0.3);
    assert!((b - 0.6).abs() < 1e-6, "offsets add up: {b}");
    // Saturates at the rails, never past.
    assert_eq!(accumulate_rcs_axis(0.9, 0.5), 1.0);
    assert_eq!(accumulate_rcs_axis(-0.9, -0.5), -1.0);
    // Pulling back from a rail walks toward the other one.
    assert!((accumulate_rcs_axis(1.0, -0.4) - 0.6).abs() < 1e-6);
}

/// The player's `RcsIntent` is delta-driven: with `RcsActive` and no fresh
/// input, it fades to zero over ticks, so the ship stops nudging when the mouse
/// stops instead of coasting a held joystick. An autopilot ship (no
/// `RcsActive`) is NOT decayed - it rewrites its own intent each tick.
#[test]
fn player_rcs_intent_decays_when_input_stops_but_autopilot_intent_does_not() {
    let mut app = flight_app();
    let (player, _, _) = spawn_ship(&mut app);
    let (auto, _, _) = spawn_ship(&mut app);
    settle(&mut app);
    // Player: RcsActive + a held intent from a mouse frame that then stops.
    app.world_mut()
        .entity_mut(player)
        .insert((RcsIntent(Vec3::new(0.8, 0.0, 0.0)), RcsActive));
    // Autopilot-style: an intent WITHOUT RcsActive (nothing rewrites it here).
    app.world_mut()
        .entity_mut(auto)
        .insert(RcsIntent(Vec3::new(0.8, 0.0, 0.0)));

    for _ in 0..30 {
        app.update();
    }

    assert!(
        app.world().get::<RcsIntent>(player).unwrap().0.length() < 1e-3,
        "the player's held intent decays to ~zero without fresh input (got {:?})",
        app.world().get::<RcsIntent>(player).unwrap().0
    );
    assert!(
        app.world().get::<RcsIntent>(auto).unwrap().0.length() > 0.5,
        "a non-RcsActive (autopilot) intent is NOT decayed (got {:?})",
        app.world().get::<RcsIntent>(auto).unwrap().0
    );
}

/// A held RCS nudge builds the along-axis speed up toward the cap and then
/// levels off - never past it - and, applied at the COM, never spins the
/// hull or drifts off-axis. Identity frame, so ship-local +X is world +X.
#[test]
fn rcs_builds_to_the_cap_then_levels_off_without_torque() {
    let mut app = flight_app();
    let cap = 2.0;
    let (ship, _controller) = spawn_rcs_ship(&mut app, cap);
    set_rcs(&mut app, ship, Vec3::X);
    for _ in 0..600 {
        app.update();
        let vx = velocity_of(&app, ship).x;
        assert!(
            vx <= cap + 1e-2,
            "RCS must never push past the cap (vx={vx})"
        );
    }
    let v = velocity_of(&app, ship);
    assert!(
        v.x > cap - 0.1,
        "a held nudge should reach the cap (vx={})",
        v.x
    );
    assert!(
        v.y.abs() < 1e-2 && v.z.abs() < 1e-2,
        "no off-axis drift ({v:?})"
    );
    assert!(
        angular_speed_of(&app, ship) < 1e-3,
        "an impulse at the COM must not spin the hull"
    );
}

/// The cap is directional: at `+cap` a forward command adds nothing, but the
/// opposite command still accelerates the ship down to `-cap` - the user's
/// "moving forward, RCS forward does nothing, backward still works" rule.
#[test]
fn rcs_holds_the_cap_forward_but_reverses_freely() {
    let mut app = flight_app();
    let cap = 2.0;
    let (ship, _controller) = spawn_rcs_ship(&mut app, cap);
    set_rcs(&mut app, ship, Vec3::X);
    for _ in 0..600 {
        app.update();
    }
    let at_cap = velocity_of(&app, ship).x;
    assert!(at_cap > cap - 0.1, "should be at the cap (vx={at_cap})");
    // Holding +X longer adds no further speed.
    for _ in 0..200 {
        app.update();
    }
    let still = velocity_of(&app, ship).x;
    assert!(
        (still - at_cap).abs() < 1e-2,
        "at the cap, more +X buys nothing ({at_cap} -> {still})"
    );
    // The opposite command decelerates through zero toward -cap.
    set_rcs(&mut app, ship, -Vec3::X);
    for _ in 0..900 {
        app.update();
    }
    let reversed = velocity_of(&app, ship).x;
    assert!(
        reversed < -(cap - 0.1),
        "reverse RCS still works down to -cap (vx={reversed})"
    );
}

/// RCS is a controller verb: a ship whose controller withholds `Rcs` does
/// not move, even with an intent written on it.
#[test]
fn rcs_does_nothing_without_the_verb() {
    let mut app = flight_app();
    let (ship, controller) = spawn_rcs_ship(&mut app, 2.0);
    app.world_mut()
        .entity_mut(controller)
        .insert(WithheldVerbs([FlightVerb::Rcs].into_iter().collect()));
    set_rcs(&mut app, ship, Vec3::X);
    for _ in 0..300 {
        app.update();
    }
    assert!(
        velocity_of(&app, ship).length() < 1e-3,
        "no RCS verb, no fine-adjust"
    );
}

/// The push is in the ship's LOCAL frame: with the hull yawed 90 degrees, a
/// local +X command drives the ship along the rotated world axis, not world
/// +X, with no off-axis drift and no spin (the `degenerate-inertia-frames`
/// lesson - exercise a non-identity frame).
#[test]
fn rcs_pushes_along_the_ship_local_axis_in_a_rotated_frame() {
    let mut app = flight_app();
    let cap = 2.0;
    let (ship, _thruster, controller) = spawn_ship(&mut app);
    app.world_mut()
        .entity_mut(ship)
        .insert((RcsIntent::default(), RcsSpeedCap(cap)));
    let yaw = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
    app.world_mut()
        .get_mut::<ControllerSectionRotationInput>(controller)
        .unwrap()
        .0 = yaw;
    // Let the PD swing the hull to the yaw and come to rest there.
    for _ in 0..400 {
        app.update();
    }
    assert!(
        angular_speed_of(&app, ship) < 1e-2,
        "hull should be settled before the RCS push"
    );
    // Command local +X; capture the ACTUAL hull frame so the test tolerates
    // any residual PD error.
    let world_axis = app.world().get::<Rotation>(ship).unwrap().mul_vec3(Vec3::X);
    assert!(
        world_axis.dot(Vec3::X).abs() < 0.05,
        "the hull really is yawed away from world +X ({world_axis:?})"
    );
    set_rcs(&mut app, ship, Vec3::X);
    for _ in 0..600 {
        app.update();
    }
    let v = velocity_of(&app, ship);
    let along = v.dot(world_axis);
    let off = (v - world_axis * along).length();
    assert!(
        along > cap - 0.15,
        "reaches the cap along the rotated local +X (along={along})"
    );
    assert!(off < 0.05, "no world off-axis drift (off={off})");
    assert!(
        angular_speed_of(&app, ship) < 5e-2,
        "still no meaningful spin from the COM push"
    );
}

/// The error-relative primitive: a ship already moving FASTER than the cap
/// can still be trimmed by a sub-cap delta when an `RcsReference` rebases the
/// cap. At 5 u/s with a matching 5 u/s reference, a prograde nudge pushes
/// (residual is zero, full headroom) and climbs until `v - reference` hits
/// the cap. WITHOUT the reference the same command gates to zero - the plain
/// absolute cap (2 u/s) is already exceeded. Deleting the reference term in
/// rcs_burn_system collapses the two cases, failing the "pushed" assertion.
#[test]
fn rcs_relative_cap_trims_a_fast_moving_reference() {
    // With the reference: prograde trim acts despite |v| > cap.
    let mut app = flight_app();
    let (ship, _, _) = spawn_ship(&mut app);
    settle(&mut app);
    app.world_mut().entity_mut(ship).insert((
        LinearVelocity(Vec3::new(5.0, 0.0, 0.0)),
        RcsReference(Vec3::new(5.0, 0.0, 0.0)),
        RcsIntent(Vec3::new(0.5, 0.0, 0.0)),
    ));
    run(&mut app, 300);
    let with_ref = velocity_of(&app, ship).x;
    assert!(
        with_ref > 5.1,
        "an error-relative trim pushes prograde past the reference despite |v| > cap (v.x = {with_ref})"
    );
    assert!(
        with_ref <= 5.0 + 2.0 + 0.3,
        "but only up to cap ABOVE the reference (5 + cap = 7), got {with_ref}"
    );

    // Without the reference: the same command at |v| > cap gates to zero.
    let mut app = flight_app();
    let (ship, _, _) = spawn_ship(&mut app);
    settle(&mut app);
    app.world_mut().entity_mut(ship).insert((
        LinearVelocity(Vec3::new(5.0, 0.0, 0.0)),
        RcsIntent(Vec3::new(0.5, 0.0, 0.0)),
    ));
    run(&mut app, 300);
    let no_ref = velocity_of(&app, ship).x;
    assert!(
        no_ref < 5.05,
        "the plain absolute cap is already exceeded, so the prograde command does nothing (v.x = {no_ref})"
    );
}

/// The error-relative reference is cleared on disengage
/// (`shared-primitive-clear-on-handoff`): an orbit leaves a fast `RcsReference`
/// behind, and if it lingered it would silently rebase the player's next
/// absolute-cap nudge. After the orbit disengages both the intent and the
/// reference must be zero.
#[test]
fn orbit_rcs_reference_clears_on_disengage() {
    let mut app = orbit_app();
    let well = spawn_orbit_well(&mut app);
    let (ship, _, _) = spawn_ship(&mut app);
    app.world_mut()
        .entity_mut(ship)
        .insert(Transform::from_xyz(50.0, 0.0, 0.0));
    settle(&mut app);
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::Orbit {
            well,
            plan: None,
        }));
    // Fly until the trim is live (a non-zero reference is written).
    let mut got_reference = false;
    for _ in 0..1500 {
        app.update();
        if app
            .world()
            .get::<RcsReference>(ship)
            .is_some_and(|r| r.0.length() > 1e-3)
        {
            got_reference = true;
            break;
        }
    }
    assert!(
        got_reference,
        "the orbit trim should write a live RcsReference"
    );

    app.world_mut().entity_mut(ship).remove::<Autopilot>();
    run(&mut app, 3);
    let reference = app
        .world()
        .get::<RcsReference>(ship)
        .map(|r| r.0.length())
        .unwrap_or(0.0);
    let intent = app
        .world()
        .get::<RcsIntent>(ship)
        .map(|i| i.0.length())
        .unwrap_or(0.0);
    assert!(
        reference < 1e-3,
        "the reference is cleared on disengage (got {reference})"
    );
    assert!(
        intent < 1e-3,
        "the intent is cleared on disengage (got {intent})"
    );
}

/// A STOP settling from below the RCS cap hands the brake to the torque-free
/// RCS primitive: `RcsIntent` goes non-zero, the main thruster stays cold,
/// and the ship still reaches rest. Delete the RCS branch and the main drive
/// brakes instead (thruster fires), failing the cold-drive assertion.
#[test]
fn stop_terminal_brakes_via_rcs() {
    let mut app = flight_app();
    let (ship, thruster, _controller) = spawn_ship(&mut app);
    settle(&mut app);
    // Below the cap, so RCS can act. STOP's goal is rest (desired == 0).
    app.world_mut()
        .entity_mut(ship)
        .insert(LinearVelocity(Vec3::new(1.5, 0.0, 0.0)));
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::Stop));

    let mut saw_rcs = false;
    let mut max_thruster = 0.0f32;
    for _ in 0..600 {
        app.update();
        if let Some(intent) = app.world().get::<RcsIntent>(ship) {
            if intent.0.length() > 1e-3 {
                saw_rcs = true;
            }
        }
        max_thruster =
            max_thruster.max(**app.world().get::<ThrusterSectionInput>(thruster).unwrap());
        if app.world().get::<Autopilot>(ship).is_none() {
            break;
        }
    }
    assert!(saw_rcs, "STOP's terminal drove RCS (non-zero RcsIntent)");
    assert!(
        max_thruster < 0.05,
        "the main drive stayed cold - RCS did the braking (max input {max_thruster})"
    );
    // Settles to WITHIN the autopilot's settle_deadband (0.75) - the same
    // "bounded creep is the contract" release the main drive gets. RCS
    // currently releases at the deadband rather than driving to
    // stop_speed_epsilon (the disengage reads no aligned main engine while
    // in RCS mode); tightening that terminal creep is a rework item.
    assert!(
        velocity_of(&app, ship).length() < 0.8,
        "STOP settled to within the deadband via RCS (v = {})",
        velocity_of(&app, ship).length()
    );
}

/// After an RCS-settled STOP disengages, the ship must STAY at rest: the
/// autopilot's residual `RcsIntent` has to be cleared on disengage, or
/// `rcs_burn_system` (which acts on any non-zero intent, autopilot or not)
/// keeps pushing and the ship drifts off to the RCS cap. Runs PAST the
/// disengage; fails if the on-remove clear is missing.
#[test]
fn rcs_settled_autopilot_leaves_the_ship_at_rest_after_disengage() {
    let mut app = flight_app();
    let (ship, _thruster, _controller) = spawn_ship(&mut app);
    settle(&mut app);
    app.world_mut()
        .entity_mut(ship)
        .insert(LinearVelocity(Vec3::new(1.5, 0.0, 0.0)));
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::Stop));

    // Settle until the autopilot releases.
    let mut disengaged = false;
    for _ in 0..1200 {
        app.update();
        if app.world().get::<Autopilot>(ship).is_none() {
            disengaged = true;
            break;
        }
    }
    assert!(disengaged, "the STOP should self-complete");
    let at_release = velocity_of(&app, ship).length();

    // Coast well past release: a leftover RcsIntent would accelerate the
    // ship toward the cap here.
    for _ in 0..400 {
        app.update();
    }
    let after = velocity_of(&app, ship).length();
    assert!(
        after <= at_release + 0.05,
        "the ship must stay at rest after disengage, not drift on a residual \
         RcsIntent (v {at_release} -> {after})"
    );
}

/// Without the `Rcs` verb the autopilot must NOT write `RcsIntent`; the same
/// STOP settles on the main drive instead (the mainline-campaign path while
/// RCS is disabled pending rework).
#[test]
fn stop_terminal_without_rcs_verb_uses_the_main_drive() {
    let mut app = flight_app();
    let (ship, _thruster, controller) = spawn_ship(&mut app);
    app.world_mut()
        .entity_mut(controller)
        .insert(WithheldVerbs([FlightVerb::Rcs].into_iter().collect()));
    settle(&mut app);
    app.world_mut()
        .entity_mut(ship)
        .insert(LinearVelocity(Vec3::new(1.5, 0.0, 0.0)));
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::Stop));

    for _ in 0..900 {
        app.update();
        if let Some(intent) = app.world().get::<RcsIntent>(ship) {
            assert!(
                intent.0.length() < 1e-3,
                "no Rcs verb: the autopilot must not write RcsIntent, got {:?}",
                intent.0
            );
        }
        if app.world().get::<Autopilot>(ship).is_none() {
            break;
        }
    }
    assert!(
        velocity_of(&app, ship).length() < 0.8,
        "still settles to within the deadband on the main drive (v = {})",
        velocity_of(&app, ship).length()
    );
}
