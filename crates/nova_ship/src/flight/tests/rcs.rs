//! RCS fine-adjustment: the torque-free COM push both the pilot and the
//! autopilot's terminal settle drive, paid from the ship's delta-v magazine.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_events::prelude::{MetersPerSecond, MetersPerSecondSquared};
use nova_gameplay::test_support::settle;

use super::support::*;
use crate::{flight::accumulate_rcs_axis, prelude::*};
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

#[test]
fn rcs_defaults_to_a_three_hundred_meter_per_second_magazine_and_five_g() {
    let settings = FlightSettings::default();
    assert_eq!(
        MetersPerSecond::from_engine(settings.rcs_budget),
        MetersPerSecond(300.0)
    );
    assert_eq!(settings.rcs_recovery_delay, 2.0);
    assert_eq!(
        MetersPerSecondSquared::from_engine(settings.rcs_recovery_rate),
        MetersPerSecondSquared(100.0)
    );
    assert_eq!(
        MetersPerSecond::from_engine(settings.rcs_handoff_speed),
        MetersPerSecond(100.0)
    );
    assert_eq!(
        MetersPerSecondSquared::from_engine(settings.rcs_accel),
        MetersPerSecondSquared(5.0 * 9.81)
    );
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

/// Full diagonal input shares one acceleration budget with straight input.
/// Without the vector clamp, three saturated axes accelerate `sqrt(3)` times
/// harder than one axis.
#[test]
fn rcs_clamps_total_acceleration_magnitude_in_every_direction() {
    fn speed_after_burn(intent: Vec3) -> f32 {
        let mut app = flight_app();
        app.world_mut().resource_mut::<FlightSettings>().rcs_accel = 3.0;
        let (ship, _) = spawn_rcs_ship(&mut app);
        set_rcs(&mut app, ship, intent);
        run(&mut app, 10);
        velocity_of(&app, ship).length()
    }

    let straight = speed_after_burn(Vec3::X);
    let diagonal = speed_after_burn(Vec3::ONE);
    assert!(
        straight > 0.1,
        "the test burn must produce measurable speed"
    );
    assert!(
        (diagonal - straight).abs() < 1e-3,
        "diagonal and straight burns must share one acceleration magnitude: {diagonal} vs {straight}"
    );
}

/// RCS is a ROOT capability: a ship without it does not move, even with an
/// intent written on it, and the refused command costs nothing.
#[test]
fn rcs_does_nothing_without_the_capability() {
    let mut app = flight_app();
    let (ship, _controller) = spawn_rcs_ship(&mut app);
    disable_capabilities(&mut app, ship, |capabilities| {
        capabilities.rcs_enabled = false;
    });
    set_rcs(&mut app, ship, Vec3::X);
    for _ in 0..300 {
        app.update();
    }
    assert!(
        velocity_of(&app, ship).length() < 1e-3,
        "no RCS capability, no fine-adjust"
    );
    assert_eq!(
        budget_of(&app, ship).spent,
        0.0,
        "a refused command spends nothing"
    );
}

/// The push is in the ship's LOCAL frame: with the hull yawed 90 degrees, a
/// local +X command drives the ship along the rotated world axis, not world
/// +X, with no off-axis drift and no spin (the `degenerate-inertia-frames`
/// lesson - exercise a non-identity frame).
#[test]
fn rcs_pushes_along_the_ship_local_axis_in_a_rotated_frame() {
    let mut app = flight_app();
    let (ship, _thruster, controller) = spawn_ship(&mut app);
    app.world_mut()
        .entity_mut(ship)
        .insert(RcsIntent::default());
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
    run(&mut app, 60);
    let v = velocity_of(&app, ship);
    let along = v.dot(world_axis);
    let off = (v - world_axis * along).length();
    assert!(
        along > 4.0,
        "a second of push builds speed along the rotated local +X (along={along})"
    );
    assert!(off < 0.05, "no world off-axis drift (off={off})");
    assert!(
        angular_speed_of(&app, ship) < 5e-2,
        "still no meaningful spin from the COM push"
    );
}

/// A STOP below the 100 m/s RCS hand-off speed hands the whole brake to the
/// torque-free RCS primitive: the hull keeps its bearing, `RcsIntent` goes non-zero, the
/// main thruster stays cold, and the ship still reaches rest. Delete the RCS
/// branch and the main drive fires; allow normal alignment and the hull yaws.
#[test]
fn stop_below_the_rcs_handoff_speed_brakes_without_turning() {
    let mut app = flight_app();
    let (ship, thruster, _controller) = spawn_ship(&mut app);
    settle(&mut app);
    let initial_rotation = *app.world().get::<Rotation>(ship).unwrap();
    // 9 u/s is 90 m/s: below the default 100 m/s hand-off. The velocity is lateral
    // to the main drive, so any main-drive braking plan would have to yaw.
    app.world_mut()
        .entity_mut(ship)
        .insert(LinearVelocity(Vec3::new(9.0, 0.0, 0.0)));
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
    let final_rotation = *app.world().get::<Rotation>(ship).unwrap();
    assert!(
        initial_rotation.angle_between(final_rotation) < 1e-3,
        "a STOP below the hand-off must keep the hull bearing"
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
/// keeps pushing and the ship drifts off on the magazine. Runs PAST the
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
    // ship here.
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

/// Without the RCS capability the autopilot must NOT write `RcsIntent`; the
/// same STOP settles on the main drive instead (the mainline-campaign path
/// while RCS is disabled pending rework).
#[test]
fn stop_terminal_without_the_rcs_capability_uses_the_main_drive() {
    let mut app = flight_app();
    let (ship, _thruster, _controller) = spawn_ship(&mut app);
    disable_capabilities(&mut app, ship, |capabilities| {
        capabilities.rcs_enabled = false;
    });
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

fn budget_of(app: &App, ship: Entity) -> RcsBudget {
    *app.world().get::<RcsBudget>(ship).unwrap()
}

/// RCS acts at any speed: a hull already moving 300 m/s gains the full 5 g
/// second from forward RCS, still without spin or off-axis drift.
#[test]
fn rcs_accelerates_a_hull_already_moving_300_meters_per_second() {
    let mut app = flight_app();
    let (ship, _controller) = spawn_rcs_ship(&mut app);
    let start = MetersPerSecond(300.0).to_engine();
    app.world_mut()
        .entity_mut(ship)
        .insert(LinearVelocity(Vec3::X * start));
    set_rcs(&mut app, ship, Vec3::X);
    run(&mut app, 60);

    let v = velocity_of(&app, ship);
    let gain = MetersPerSecond::from_engine(v.x - start).0;
    assert!(
        gain > 45.0,
        "a second of 5 g RCS at 300 m/s gains ~49 m/s, got {gain}"
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

/// The delayed magazine: a held push drains exactly the delta-v it delivers
/// and never more than the magazine holds, a command held on an empty magazine
/// keeps it empty, and release refills it only after the recovery delay.
#[test]
fn rcs_budget_drains_under_a_held_command_and_recovers_after_release() {
    let mut app = flight_app();
    let settings = FlightSettings::default();
    let capacity = settings.rcs_budget;
    let (ship, _controller) = spawn_rcs_ship(&mut app);
    set_rcs(&mut app, ship, Vec3::X);

    // Eight seconds of held push: longer than the ~6 s a full magazine lasts.
    for _ in 0..480 {
        app.update();
        let budget = budget_of(&app, ship);
        assert!(
            budget.spent <= capacity + 1e-4,
            "the magazine never overdraws ({} of {capacity})",
            budget.spent
        );
        assert!(
            (budget.spent - velocity_of(&app, ship).x).abs() < 1e-3,
            "it pays exactly the delta-v it delivered ({} vs {})",
            budget.spent,
            velocity_of(&app, ship).x
        );
    }
    let drained = budget_of(&app, ship);
    assert!(
        drained.is_empty(&settings),
        "a held push drains the magazine"
    );
    assert_eq!(drained.applied, 0.0, "an empty magazine pushes nothing");
    let top = velocity_of(&app, ship).x;
    assert!(
        (top - capacity).abs() < 1e-3,
        "one magazine delivers {capacity} u/s, got {top}"
    );

    // Still holding: the empty magazine stays empty and the hull coasts.
    run(&mut app, 180);
    assert!(
        budget_of(&app, ship).is_empty(&settings),
        "a held command keeps the magazine from recovering"
    );
    assert_eq!(velocity_of(&app, ship).x, top, "and delivers nothing");

    // Released: nothing inside the delay, then a steady refill to full.
    set_rcs(&mut app, ship, Vec3::ZERO);
    run(&mut app, 100);
    assert!(
        budget_of(&app, ship).is_empty(&settings),
        "no recovery inside the {} s delay",
        settings.rcs_recovery_delay
    );
    run(&mut app, 80);
    let refilling = budget_of(&app, ship).fraction(&settings);
    assert!(
        refilling > 0.0 && refilling < 1.0,
        "past the delay it refills gradually, got {refilling}"
    );
    run(&mut app, 240);
    assert_eq!(budget_of(&app, ship).spent, 0.0, "and it refills to full");
}

/// A STOP on an empty magazine does not stall waiting for RCS: the hand-off
/// refuses it, the main drive brakes, and the maneuver still settles.
#[test]
fn autopilot_stop_on_an_empty_rcs_budget_settles_on_the_main_drive() {
    let mut app = flight_app();
    let (ship, thruster, _controller) = spawn_ship(&mut app);
    settle(&mut app);
    let capacity = app.world().resource::<FlightSettings>().rcs_budget;
    app.world_mut().entity_mut(ship).insert((
        LinearVelocity(Vec3::new(9.0, 0.0, 0.0)),
        RcsBudget {
            spent: capacity,
            ..default()
        },
        Autopilot::engage(AutopilotAction::Stop),
    ));

    // Inside the recovery delay the magazine is certainly still empty.
    let mut max_thruster = 0.0f32;
    for _ in 0..100 {
        app.update();
        let intent = app
            .world()
            .get::<RcsIntent>(ship)
            .map_or(0.0, |i| i.0.length());
        assert!(
            intent < 1e-3,
            "a starved STOP writes no RCS command, got {intent}"
        );
        assert_eq!(budget_of(&app, ship).applied, 0.0, "and pushes nothing");
        max_thruster =
            max_thruster.max(**app.world().get::<ThrusterSectionInput>(thruster).unwrap());
    }
    assert!(
        max_thruster > 0.5,
        "the main drive takes the brake (max input {max_thruster})"
    );

    let mut disengaged = false;
    for _ in 0..1500 {
        app.update();
        if app.world().get::<Autopilot>(ship).is_none() {
            disengaged = true;
            break;
        }
    }
    assert!(disengaged, "the starved STOP still completes");
    assert!(
        velocity_of(&app, ship).length() < 0.8,
        "and settles within the deadband (v = {})",
        velocity_of(&app, ship).length()
    );
}

/// A docked pair moves as one body on the driver's RCS, and only the driver
/// pays: the partner's magazine is untouched and it makes no hiss.
#[test]
fn a_docked_pair_charges_only_the_driver_rcs_budget() {
    let mut app = flight_app();
    let (driver, ..) = spawn_ship(&mut app);
    let (partner, ..) = spawn_ship(&mut app);
    // Clear of the driver, so no contact pushes either hull.
    app.world_mut()
        .get_mut::<Transform>(partner)
        .unwrap()
        .translation = Vec3::X * 100.0;
    settle(&mut app);
    let connection = app
        .world_mut()
        .spawn(DockingConnection {
            first_ship: driver,
            first_section: Entity::PLACEHOLDER,
            second_ship: partner,
            second_section: Entity::PLACEHOLDER,
            helm: DockedHelmType::Neutral,
            measurement_fault: false,
        })
        .id();
    for (ship, drives) in [(driver, true), (partner, false)] {
        app.world_mut().entity_mut(ship).insert(DockedShip {
            connection,
            helm: Quat::IDENTITY,
            drives,
        });
    }
    let world = app.world();
    let assembly = DockedAssembly {
        mass: world.get::<ComputedMass>(driver).unwrap().value()
            + world.get::<ComputedMass>(partner).unwrap().value(),
        center_of_mass: Vec3::X * 50.0,
        linear_velocity: Vec3::ZERO,
        inertia: *world.get::<ComputedAngularInertia>(driver).unwrap(),
        reach: 60.0,
    };
    app.world_mut()
        .entity_mut(driver)
        .insert((assembly, RcsIntent(Vec3::Y)));

    run(&mut app, 60);
    let pushed = velocity_of(&app, driver);
    assert!(pushed.y > 4.0, "the pair is pushed ({pushed:?})");
    assert!(
        (velocity_of(&app, partner) - pushed).length() < 1e-4,
        "both roots take the same delta-v"
    );
    let paid = budget_of(&app, driver);
    assert!(
        (paid.spent - pushed.y).abs() < 1e-3,
        "the driver pays the pair's delta-v once ({} vs {})",
        paid.spent,
        pushed.y
    );
    assert!(paid.applied > 0.0, "the driver hisses");
    let partner_budget = budget_of(&app, partner);
    assert_eq!(partner_budget.spent, 0.0, "the partner pays nothing");
    assert_eq!(partner_budget.applied, 0.0, "and makes no hiss");
}
