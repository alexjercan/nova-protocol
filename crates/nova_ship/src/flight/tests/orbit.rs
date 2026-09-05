//! The ORBIT verb: plan a ring, insert onto it, and station-keep - which
//! never self-completes.

use bevy::prelude::*;
use nova_gameplay::{prelude::*, test_support::settle};

use super::support::*;
use crate::{flight::RcsReference, prelude::*};

/// `Hold` is not a cosmetic label: `nova_scenario`'s `track_orbit_transitions`
/// reads it as the definition of a stable orbit, and every scenario orbit event
/// (stable, lap, and so the whole First Shift orbit beat) is gated on it. A
/// flight retune that leaves the ring flown but never flips the phase would
/// stall a chapter with nothing on screen to say why. This pins the phase in
/// the geometry the campaign uses: a planetoid-strength well, where the trim
/// runs on the main drive rather than RCS.
#[test]
fn a_strong_well_orbit_reaches_the_hold_phase_the_scenario_layer_reads() {
    let mut app = orbit_app();
    let well = spawn_strong_well(&mut app);
    let (ship, _, _) = spawn_ship(&mut app);
    app.world_mut()
        .entity_mut(ship)
        .insert(Transform::from_xyz(140.0, 0.0, 0.0));
    settle(&mut app);
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::Orbit {
            well,
            plan: None,
        }));

    let mut held = false;
    for _ in 0..8000 {
        app.update();
        if app.world().get::<Autopilot>(ship).map(|ap| ap.phase) == Some(AutopilotPhase::Hold) {
            held = true;
            break;
        }
    }
    assert!(
        held,
        "station-keeping in a strong well must reach Hold - scenario orbit progress is gated on it"
    );
}

/// Regression: the two menu ambience ships crashed the asteroid and could not
/// hold orbit. In a STRONG well - local gravity accel above the RCS accel - the
/// error-relative ORBIT trim must NOT take over station-keeping: a 1.5 u/s^2
/// RCS cannot hold against a >1.5 u/s^2 inward pull, so handing it the orbit
/// and zeroing the main drive spirals the ship in. The `use_rcs_orbit` gate now
/// requires RCS to have clear authority over local gravity; here it does not,
/// so the ship keeps the ring on the full-authority main drive and RCS stays
/// idle. WITHOUT the gate the radius collapses and this fails.
#[test]
fn strong_gravity_orbit_holds_the_ring_on_the_main_drive_not_rcs() {
    let mut app = orbit_app();
    let well = spawn_strong_well(&mut app);
    let (ship, _, _) = spawn_ship(&mut app);
    app.world_mut()
        .entity_mut(ship)
        .insert(Transform::from_xyz(140.0, 0.0, 0.0));
    settle(&mut app);
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::Orbit {
            well,
            plan: None,
        }));

    // Let the insertion settle, then watch a long hold.
    run(&mut app, 3000);
    let plan_radius = match app.world().get::<Autopilot>(ship) {
        Some(Autopilot {
            action: AutopilotAction::Orbit {
                plan: Some(plan), ..
            },
            ..
        }) => plan.radius,
        other => panic!("ORBIT should stay engaged with a plan, got {other:?}"),
    };

    let mut r_min = f32::MAX;
    let mut saw_rcs = false;
    for _ in 0..5000 {
        app.update();
        r_min = r_min.min(position_of(&app, ship).length());
        if app
            .world()
            .get::<RcsIntent>(ship)
            .is_some_and(|i| i.0.length() > 1e-3)
        {
            saw_rcs = true;
        }
    }

    assert!(
        r_min > 0.6 * plan_radius,
        "the ship must hold the ring, not spiral into the rock (r_min {r_min}, plan {plan_radius})"
    );
    assert!(
        !saw_rcs,
        "in a strong well the orbit stays on the main drive - RCS lacks the authority, so it must not engage"
    );
}

/// ORBIT trims via the error-relative RCS, but ONLY while the residual `|v -
/// v_orbit|` is below the cap. From near-rest the desired is the full orbital
/// velocity (~4.9 u/s at r=50, above this hull's 2 u/s cap), so the main drive
/// spins the orbit up and RCS stays idle; once the ship is near orbital
/// velocity the residual drops sub-cap and RCS takes over the trim. The
/// invariant that pins error-relative (not absolute) behavior: whenever RCS is
/// trimming, its `RcsReference` is the fast orbital velocity (well above the
/// cap) and the VECTOR `|v - reference|` is within the one budget - impossible
/// under the old absolute cap, which would have gated to zero.
///
/// The cap is pinned per hull rather than left to
/// [`FlightSettings::rcs_speed_cap`]: the premise is a cap BELOW the ring's
/// orbital speed, and the shipped default (100 m/s) sits well above it.
#[test]
fn orbit_engages_rcs_only_to_trim_a_sub_cap_residual() {
    let cap = 2.0;
    let mut app = orbit_app();
    let well = spawn_orbit_well(&mut app);
    let (ship, _, _) = spawn_ship(&mut app);
    app.world_mut()
        .entity_mut(ship)
        .insert((Transform::from_xyz(50.0, 0.0, 0.0), RcsSpeedCap(cap)));
    settle(&mut app);
    // From rest the residual is the full orbital speed, above the cap, so
    // the first ticks must NOT engage RCS - the main drive spins up.
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::Orbit {
            well,
            plan: None,
        }));
    for _ in 0..5 {
        app.update();
        let intent = app
            .world()
            .get::<RcsIntent>(ship)
            .map(|i| i.0.length())
            .unwrap_or(0.0);
        assert!(
            intent < 1e-3,
            "RCS must not trim while spinning up from rest (residual > cap), got {intent}"
        );
    }

    let mut saw_trim = false;
    for _ in 0..1500 {
        app.update();
        let intent = app
            .world()
            .get::<RcsIntent>(ship)
            .map(|i| i.0)
            .unwrap_or(Vec3::ZERO);
        if intent.length() > 1e-3 {
            saw_trim = true;
            let reference = app
                .world()
                .get::<RcsReference>(ship)
                .map(|r| r.0)
                .unwrap_or(Vec3::ZERO);
            let v = velocity_of(&app, ship);
            assert!(
                reference.length() > cap,
                "the trim reference is the fast orbital velocity, above the cap (got {})",
                reference.length()
            );
            assert!(
                (v - reference).length() <= cap + 0.5,
                "RCS only trims a sub-cap residual (|v - ref| = {}, cap {cap})",
                (v - reference).length()
            );
        }
    }
    assert!(
        saw_trim,
        "ORBIT should engage the error-relative RCS once at orbital speed"
    );
    assert!(
        app.world().get::<Autopilot>(ship).is_some(),
        "orbit never self-completes"
    );
}

#[test]
fn orbit_engages_from_near_rest_and_holds_the_ring_for_a_lap() {
    let mut app = orbit_app();
    let well = spawn_orbit_well(&mut app);
    let (ship, _, _) = spawn_ship(&mut app);
    // Park near-rest at r = 50, inside the stable band: the whole
    // insertion - plan, align, burn to tangential v_circ, hold - is the
    // computer's job.
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

    // Insertion window, then a full ~64s lap under observation.
    run(&mut app, 900);
    let plan_radius = match app.world().get::<Autopilot>(ship) {
        Some(Autopilot {
            action: AutopilotAction::Orbit {
                plan: Some(plan), ..
            },
            ..
        }) => plan.radius,
        other => panic!("ORBIT should stay engaged with a plan, got {other:?}"),
    };
    assert!(
        (plan_radius - 50.0).abs() < 1.0,
        "r = 50 is inside the band, the plan should keep it, got {plan_radius}"
    );

    let (mut r_min, mut r_max) = (f32::MAX, f32::MIN);
    let mut held = false;
    for _ in 0..4200 {
        app.update();
        let r = position_of(&app, ship).length();
        r_min = r_min.min(r);
        r_max = r_max.max(r);
        if app.world().get::<Autopilot>(ship).map(|ap| ap.phase) == Some(AutopilotPhase::Hold) {
            held = true;
        }
    }

    assert!(
        r_min > 0.8 * plan_radius && r_max < 1.25 * plan_radius,
        "orbit drifted out of the band: min {r_min}, max {r_max}, plan {plan_radius}"
    );
    assert!(held, "station-keeping should reach the Hold phase");
    let speed = velocity_of(&app, ship).length();
    let v_circ = nova_gameplay::gravity::circular_orbit_speed(1200.0, plan_radius);
    assert!(
        (speed - v_circ).abs() < 0.35 * v_circ,
        "orbital speed should sit near v_circ {v_circ}, got {speed}"
    );
    assert!(
        app.world().get::<Autopilot>(ship).is_some(),
        "an orbit is not a destination: ORBIT never self-completes"
    );
}

#[test]
fn orbit_disengages_when_the_well_dies() {
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
    run(&mut app, 60);
    assert!(app.world().get::<Autopilot>(ship).is_some());

    app.world_mut().entity_mut(well).despawn();
    run(&mut app, 2);

    assert!(
        app.world().get::<Autopilot>(ship).is_none(),
        "a dead well disengages ORBIT, like a vanished GOTO target"
    );
}

#[test]
fn orbit_inherits_the_capability_coupling() {
    // Dead engines: the computer cannot circularize, ORBIT disengages -
    // same rule as STOP/GOTO.
    let mut app = orbit_app();
    let well = spawn_orbit_well(&mut app);
    let (ship, thruster, _) = spawn_ship(&mut app);
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
    run(&mut app, 30);
    app.world_mut()
        .entity_mut(thruster)
        .insert(SectionInactiveMarker);
    run(&mut app, 2);
    assert!(
        app.world().get::<Autopilot>(ship).is_none(),
        "no live engines, no ORBIT"
    );

    // Dead flight computer: same, one level earlier.
    let mut app = orbit_app();
    let well = spawn_orbit_well(&mut app);
    let (ship, _, controller) = spawn_ship(&mut app);
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
    run(&mut app, 30);
    app.world_mut()
        .entity_mut(controller)
        .insert(SectionInactiveMarker);
    run(&mut app, 2);
    assert!(
        app.world().get::<Autopilot>(ship).is_none(),
        "no live flight computer, no ORBIT"
    );
}
