//! The scripted alignment order: a hull turned onto an authored bearing by a
//! scenario, with no autopilot and no translation.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_gameplay::{prelude::*, test_support::settle};

use super::support::*;
use crate::prelude::*;

/// The bearing a test aims at: well off the ship's nose, so the hull has a
/// real swing to make rather than a nudge inside the tolerance.
const MARK: Vec3 = Vec3::new(400.0, 0.0, 0.0);

#[test]
fn a_scripted_alignment_turns_the_hull_onto_its_mark_and_reports_settled() {
    let mut app = flight_app();
    let (ship, _, _) = spawn_ship(&mut app);
    settle(&mut app);

    let start_error = forward_of(&app, ship).angle_between(MARK.normalize());
    assert!(
        start_error > 1.0,
        "the fixture must start well off the mark, was {start_error} rad"
    );

    app.world_mut().entity_mut(ship).insert(ScriptedAlign {
        look_at: MARK,
        tolerance: 0.02,
    });

    let mut settled_at = None;
    for tick in 0..2400 {
        app.update();
        if app.world().entity(ship).contains::<ScriptedAlignSettled>() {
            settled_at = Some(tick);
            break;
        }
    }
    assert!(
        settled_at.is_some(),
        "the alignment must reach its tolerance within the budget"
    );

    let error = forward_of(&app, ship).angle_between(MARK.normalize());
    assert!(
        error <= 0.02,
        "settled means the HULL is on the bearing, was {error} rad"
    );
    assert!(
        velocity_of(&app, ship).length() < 0.01,
        "an alignment never burns for translation"
    );
    assert!(
        app.world().get::<Autopilot>(ship).is_none(),
        "and it engages no autopilot"
    );
}

#[test]
fn a_settled_alignment_holds_its_facing_against_a_shove() {
    let mut app = flight_app();
    let (ship, _, _) = spawn_ship(&mut app);
    settle(&mut app);
    app.world_mut().entity_mut(ship).insert(ScriptedAlign {
        look_at: MARK,
        tolerance: 0.02,
    });
    for _ in 0..2400 {
        app.update();
        if app.world().entity(ship).contains::<ScriptedAlignSettled>() {
            break;
        }
    }
    assert!(app.world().entity(ship).contains::<ScriptedAlignSettled>());

    // A collision, or a wreck shouldering past: something puts the hull off
    // its bearing after the order already reported complete. Written straight
    // onto the pose, because the rig's PD has enough authority to swallow any
    // torque a test could apply inside one tick.
    let knocked = Quat::from_rotation_y(0.6) * app.world().get::<Rotation>(ship).unwrap().0;
    app.world_mut().entity_mut(ship).insert(Rotation(knocked));
    app.update();
    assert!(
        forward_of(&app, ship).angle_between(MARK.normalize()) > 0.02,
        "the shove must actually knock the hull off the bearing"
    );

    run(&mut app, 2400);
    let error = forward_of(&app, ship).angle_between(MARK.normalize());
    assert!(
        error <= 0.02,
        "the hold brings the hull back onto its mark, was {error} rad"
    );
    assert!(
        app.world().entity(ship).contains::<ScriptedAlignSettled>(),
        "and a completed order does not un-complete: the beat after it already ran"
    );
}

#[test]
fn a_hull_with_no_live_flight_computer_never_settles() {
    let mut app = flight_app();
    let (ship, _, controller) = spawn_ship(&mut app);
    settle(&mut app);
    app.world_mut()
        .entity_mut(controller)
        .insert(SectionInactiveMarker);
    app.world_mut().entity_mut(ship).insert(ScriptedAlign {
        look_at: MARK,
        tolerance: 0.02,
    });

    run(&mut app, 600);
    assert!(
        !app.world().entity(ship).contains::<ScriptedAlignSettled>(),
        "no flight computer, no turn - and an order that honestly never completes"
    );
}

/// A docked driver aims from its pair's centre of mass, the point the pair
/// turns about. The mark sits dead ahead of the root's own centre of mass and
/// well off the pair's, so the two answers differ. With the assembly lost the
/// order is refused: it neither swings nor settles on the root's own bearing.
/// This harness has no docked wrench, so the hull never turns; the command is
/// what is read.
#[test]
fn a_docked_driver_aims_its_scripted_bearing_from_the_pair_centre_of_mass() {
    let mut app = flight_app();
    let (ship, _, controller) = spawn_ship(&mut app);
    settle(&mut app);
    let world = app.world();
    let root_com = world.get::<Position>(ship).unwrap().0
        + world
            .get::<Rotation>(ship)
            .unwrap()
            .mul_vec3(world.get::<ComputedCenterOfMass>(ship).unwrap().0);
    let pair_com = root_com + Vec3::X * 5.0;
    let mark = root_com + Vec3::NEG_Z * 10.0;
    let assembly = DockedAssembly {
        mass: 2.0 * world.get::<ComputedMass>(ship).unwrap().value(),
        center_of_mass: pair_com,
        linear_velocity: Vec3::ZERO,
        inertia: *world.get::<ComputedAngularInertia>(ship).unwrap(),
        reach: 6.0,
    };
    app.world_mut().entity_mut(ship).insert((
        DockedShip {
            connection: Entity::PLACEHOLDER,
            helm: Quat::IDENTITY,
            drives: true,
        },
        assembly,
        ScriptedAlign {
            look_at: mark,
            tolerance: 0.02,
        },
    ));
    let command = |app: &App| {
        app.world()
            .get::<ControllerSectionRotationInput>(controller)
            .unwrap()
            .mul_vec3(Vec3::NEG_Z)
    };

    run(&mut app, 60);
    let pair_bearing = (mark - pair_com).normalize();
    assert!(
        command(&app).angle_between(pair_bearing) < 0.02,
        "the command aims from the pair: {} vs {pair_bearing}",
        command(&app)
    );
    assert!(
        !app.world().entity(ship).contains::<ScriptedAlignSettled>(),
        "the hull is not on the pair's bearing yet"
    );

    app.world_mut().entity_mut(ship).remove::<DockedAssembly>();
    let held = command(&app);
    run(&mut app, 60);
    assert_eq!(command(&app), held, "no swing without the assembly");
    assert!(
        !app.world().entity(ship).contains::<ScriptedAlignSettled>(),
        "and no completion on the root's own bearing"
    );
}
