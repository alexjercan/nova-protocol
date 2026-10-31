//! The neutralized brake: a neutralized non-player hull spends its own finite
//! RCS toward world rest, and a hull that cannot coasts with the reason.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_gameplay::{prelude::*, test_support::settle};

use super::support::*;
use crate::prelude::*;

fn brake_of(app: &App, ship: Entity) -> Option<NeutralizedBrakeType> {
    app.world().get::<NeutralizedBrakeType>(ship).copied()
}

fn budget_of(app: &App, ship: Entity) -> RcsBudget {
    *app.world().get::<RcsBudget>(ship).unwrap()
}

/// Two neutralized hulls brake to world rest on their own RCS and pay
/// exactly the delta-v they shed: one still flying a maneuver on live
/// engines (the weapons-only neutralization), which the brake takes off, and
/// one neutralized by losing its last thruster, which needs no engine.
#[test]
fn a_neutralized_hull_brakes_to_world_rest_on_its_own_rcs_with_or_without_engines() {
    let mut app = brake_flight_app();
    let (flying, _, _) = spawn_ship(&mut app);
    let (stranded, stranded_thruster, _) = spawn_ship(&mut app);
    app.world_mut()
        .entity_mut(stranded)
        .insert(Transform::from_xyz(0.0, 0.0, 50.0));
    settle(&mut app);

    let drift = Vec3::new(2.0, -1.0, 1.5);
    app.world_mut().entity_mut(flying).insert((
        LinearVelocity(drift),
        Rotation(Quat::from_rotation_y(1.0)),
        Autopilot::engage(AutopilotAction::GotoPos {
            position: Vec3::new(0.0, 0.0, -500.0),
        }),
    ));
    app.world_mut()
        .entity_mut(stranded)
        .insert(LinearVelocity(drift));
    app.update();
    let shed = [flying, stranded].map(|ship| velocity_of(&app, ship).length());
    app.world_mut().entity_mut(flying).insert(NeutralizedMarker);
    // The rig's integrity layer neutralizes the unarmed hull for this loss.
    app.world_mut()
        .entity_mut(stranded_thruster)
        .insert(SectionInactiveMarker);

    run(&mut app, 120);

    assert!(
        app.world().get::<NeutralizedMarker>(stranded).is_some(),
        "losing the last thruster neutralized the unarmed hull"
    );
    assert!(
        app.world().get::<Autopilot>(flying).is_none(),
        "the brake took the helm from the maneuver"
    );
    for (ship, shed) in [flying, stranded].into_iter().zip(shed) {
        assert_eq!(brake_of(&app, ship), Some(NeutralizedBrakeType::Rested));
        let speed = velocity_of(&app, ship).length();
        assert!(
            speed < 1e-3,
            "the hull rests in the world frame, got {speed} u/s"
        );
        let spent = budget_of(&app, ship).spent;
        assert!(
            (spent - shed).abs() < 1e-3,
            "the hull's own magazine paid the {shed} u/s it shed, spent {spent}"
        );
    }
}

/// A hull that cannot brake coasts at its speed, spends nothing and names
/// why; the player's own neutralized ship is never braked at all.
#[test]
fn an_incapable_or_empty_neutralized_hull_coasts_and_names_why() {
    let mut app = brake_flight_app();
    let spawn_at = |app: &mut App, position: Vec3| {
        let (ship, _, controller) = spawn_ship(app);
        app.world_mut()
            .entity_mut(ship)
            .insert(Transform::from_translation(position));
        (ship, controller)
    };
    let (computerless, controller) = spawn_at(&mut app, Vec3::new(0.0, 0.0, 5000.0));
    let (disabled, _) = spawn_at(&mut app, Vec3::new(0.0, 0.0, 5020.0));
    let (empty, _) = spawn_at(&mut app, Vec3::new(0.0, 0.0, 5040.0));
    let (docked, _) = spawn_at(&mut app, Vec3::new(0.0, 0.0, 5060.0));
    let (player, _) = spawn_at(&mut app, Vec3::new(0.0, 0.0, 5080.0));
    // At r = 140 the menu-planetoid well pulls ~2.2 u/s^2, past what RCS holds.
    spawn_strong_well(&mut app);
    let (deep, _) = spawn_at(&mut app, Vec3::new(140.0, 0.0, 0.0));
    settle(&mut app);

    app.world_mut().entity_mut(controller).despawn();
    disable_rcs(&mut app, disabled);
    let capacity = budget_of(&app, empty).capacity;
    app.world_mut().entity_mut(empty).insert(RcsBudget {
        spent: capacity,
        ..default()
    });
    app.world_mut().entity_mut(docked).insert(DockedShip {
        connection: Entity::PLACEHOLDER,
        helm: Quat::IDENTITY,
        drives: false,
    });
    app.world_mut()
        .entity_mut(player)
        .insert(PlayerSpaceshipMarker);
    let drift = Vec3::new(2.0, 0.0, 0.0);
    let ships = [computerless, disabled, empty, docked, player, deep];
    let spent_before = ships.map(|ship| budget_of(&app, ship).spent);
    for ship in ships {
        app.world_mut()
            .entity_mut(ship)
            .insert((LinearVelocity(drift), NeutralizedMarker));
    }

    run(&mut app, 60);

    for (ship, expected) in [
        (computerless, NeutralizedBrakeType::NoFlightComputer),
        (disabled, NeutralizedBrakeType::RcsDisabled),
        (empty, NeutralizedBrakeType::RcsEmpty),
        (docked, NeutralizedBrakeType::Docked),
        (deep, NeutralizedBrakeType::Gravity),
    ] {
        assert_eq!(brake_of(&app, ship), Some(expected));
    }
    assert_eq!(
        brake_of(&app, player),
        None,
        "the player keeps their own STOP and is never braked"
    );
    for (ship, spent) in ships.into_iter().zip(spent_before) {
        let moved = (velocity_of(&app, ship) - drift).length();
        assert!(moved < 1e-4, "the hull coasts, its velocity moved {moved}");
        assert_eq!(
            budget_of(&app, ship).spent,
            spent,
            "and spends no RCS on a brake it cannot fly"
        );
    }
}

/// A brake that empties the magazine short of rest ends for good: the
/// magazine refills once the hull is idle, and the hull keeps coasting.
#[test]
fn an_exhausted_brake_does_not_restart_when_its_magazine_refills() {
    let mut app = brake_flight_app();
    let (ship, _, _) = spawn_ship(&mut app);
    settle(&mut app);
    let capacity = budget_of(&app, ship).capacity;
    let left = 0.5;
    let drift = Vec3::new(3.0, 0.0, 0.0);
    app.world_mut().entity_mut(ship).insert((
        RcsBudget {
            spent: capacity - left,
            ..default()
        },
        LinearVelocity(drift),
        NeutralizedMarker,
    ));

    run(&mut app, 60);
    assert_eq!(brake_of(&app, ship), Some(NeutralizedBrakeType::RcsEmpty));
    let coast = velocity_of(&app, ship);
    assert!(
        (coast.length() - (drift.length() - left)).abs() < 1e-3,
        "the last {left} u/s of the magazine went into the brake, coasting at {coast}"
    );

    // Past the recovery delay and a full refill.
    run(&mut app, 600);
    assert_eq!(
        budget_of(&app, ship).spent,
        0.0,
        "the idle magazine refilled"
    );
    assert_eq!(brake_of(&app, ship), Some(NeutralizedBrakeType::RcsEmpty));
    assert_eq!(
        velocity_of(&app, ship),
        coast,
        "and the refill never restarted the brake"
    );
}
