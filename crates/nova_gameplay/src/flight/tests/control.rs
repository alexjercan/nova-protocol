//! The flight computer: it damps the hull while it lives, the autopilot
//! dies with it, and the command it writes must reach the PD this tick.

use avian3d::prelude::*;
use bevy::prelude::*;

use super::support::*;
use crate::{
    flight::NovaFlightPlugin,
    integrity::test_support::{settle, unfinished_integrity_physics_app},
    prelude::*,
    sections::thruster_section::thruster_impulse_system,
};

/// Control case for the disabled-controller regression below: a LIVE
/// controller damps an imposed spin, so the "disabled" test cannot pass
/// vacuously. Spins about Y (a transverse axis); the ship is a symmetric
/// top about its long z-axis, so this spin is torque-free-constant with no
/// tumbling - any decay is the PD doing its job.
#[test]
fn a_live_controller_damps_an_imposed_spin() {
    let mut app = flight_app();
    let (ship, _, _) = spawn_ship(&mut app);
    settle(&mut app);

    let spin = Vec3::new(0.0, 2.0, 0.0);
    app.world_mut()
        .entity_mut(ship)
        .insert(AngularVelocity(spin));
    run(&mut app, 120);

    let rate = app.world().get::<AngularVelocity>(ship).unwrap().length();
    assert!(
        rate < spin.length() * 0.5,
        "a live controller should damp the imposed spin: {} -> {rate} rad/s",
        spin.length()
    );
}

/// Regression: a controller disabled in place (`SectionInactiveMarker`, as the
/// integrity pipeline marks a zero-health non-leaf section) must stop torquing
/// the hull. Before the fix, `sync_controller_section_forces` still applied the
/// PD output toward the frozen command, so a dead computer kept stabilizing the
/// ship. Now the imposed spin is left untouched - a spun ship keeps spinning.
#[test]
fn a_disabled_controller_leaves_the_spin_untouched() {
    let mut app = flight_app();
    let (ship, _, controller) = spawn_ship(&mut app);
    settle(&mut app);

    let spin = Vec3::new(0.0, 2.0, 0.0);
    app.world_mut()
        .entity_mut(ship)
        .insert(AngularVelocity(spin));
    app.world_mut()
        .entity_mut(controller)
        .insert(SectionInactiveMarker);
    run(&mut app, 120);

    // No live computer, no other torque source: the spin is conserved.
    let rate = app.world().get::<AngularVelocity>(ship).unwrap().0;
    assert!(
        (rate - spin).length() < 0.05,
        "a disabled controller must not damp the spin: {spin:?} -> {rate:?}"
    );
}

/// The playtest bug: an editor-built ship binds keys straight
/// to its thrusters (`SpaceshipThrusterInputBinding`), and the autopilot
/// used to exclude bound thrusters from its authority - so it rotated but
/// could never burn. The computer must command every live engine.
#[test]
fn autopilot_commands_editor_bound_thrusters() {
    let mut app = flight_app();
    let (ship, thruster, _) = spawn_ship(&mut app);
    withhold_rcs(&mut app, ship);
    app.world_mut()
        .entity_mut(thruster)
        .insert(SpaceshipThrusterInputBinding(vec![]));
    settle(&mut app);
    app.world_mut()
        .entity_mut(ship)
        .insert(LinearVelocity(Vec3::new(6.0, 0.0, 0.0)));
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::Stop));

    run(&mut app, 900);

    let speed = velocity_of(&app, ship).length();
    assert!(
        speed < 0.5,
        "STOP must burn bound thrusters too, got {speed}"
    );
    // And the engines are cooled on release - a residual input on a
    // bound thruster would ghost-burn forever (nothing else writes it).
    let residual = app
        .world()
        .get::<ThrusterSectionInput>(thruster)
        .map(|i| i.0)
        .unwrap_or(f32::NAN);
    assert_eq!(residual, 0.0, "disengage must cool the engines");
}

#[test]
fn a_dead_flight_computer_disengages_the_autopilot() {
    let mut app = flight_app();
    let (ship, _, controller) = spawn_ship(&mut app);
    settle(&mut app);
    app.world_mut()
        .entity_mut(ship)
        .insert(LinearVelocity(Vec3::new(6.0, 0.0, 0.0)));
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::Stop));
    run(&mut app, 5);
    assert!(app.world().get::<Autopilot>(ship).is_some());

    // The controller section is knocked out: no computer, no autopilot.
    app.world_mut()
        .entity_mut(controller)
        .insert(SectionInactiveMarker);
    run(&mut app, 2);

    assert!(
        app.world().get::<Autopilot>(ship).is_none(),
        "losing the controller section must drop the autopilot"
    );
    // And the ship coasts from here - nothing else brakes it.
    let v0 = velocity_of(&app, ship);
    run(&mut app, 60);
    assert!((velocity_of(&app, ship) - v0).length() < 0.2);
}

/// The rotation command must reach the PD on the tick it was written
/// it was written. The copy from ControllerSectionRotationInput
/// into the bcs PDControllerInput used to run in the Update schedule
/// while both its producer (autopilot, FixedUpdate) and consumer (PD,
/// PDControllerSystems::Sync in FixedUpdate) tick on the fixed clock -
/// so the PD chased a command 1-2 ticks stale, varying with the
/// 64 Hz-vs-render beat, and fought up to 0.22 rad of phantom command
/// error during fast slews (~20% wasted torque). This rig runs the REAL
/// plugins (NovaFlightPlugin + ControllerSectionPlugin), so it pins the
/// SHIPPED wiring, not a hand-wired copy of it: a probe inside
/// FixedUpdate, after PDControllerSystems::Sync, asserts the PD
/// consumed exactly the command the autopilot wrote this tick, on
/// every tick of a leg with an active slew. A/B: the Update-schedule
/// copy fails at 0.22 rad.
#[test]
fn autopilot_command_reaches_the_pd_on_the_same_tick() {
    #[derive(Resource, Default)]
    struct StaleTrace {
        max_angle: f32,
        max_cmd_step: f32,
        samples: usize,
    }

    fn stale_probe(
        mut trace: ResMut<StaleTrace>,
        mut prev: Local<Option<Quat>>,
        q_controller: Query<
            (&PDControllerInput, &ControllerSectionRotationInput),
            With<ControllerSectionMarker>,
        >,
    ) {
        for (pd_input, command) in &q_controller {
            trace.max_angle = trace.max_angle.max(pd_input.angle_between(**command));
            trace.samples += 1;
            if let Some(prev) = *prev {
                trace.max_cmd_step = trace.max_cmd_step.max(command.angle_between(prev));
            }
            *prev = Some(**command);
        }
    }

    let mut app = unfinished_integrity_physics_app();
    app.add_plugins(PDControllerPlugin);
    app.add_plugins(NovaFlightPlugin);
    app.add_plugins(crate::prelude::ControllerSectionPlugin { render: false });
    app.init_resource::<StaleTrace>();
    // The thruster plugin carries render-material deps, so the impulse
    // system is registered directly, as the flight harness does.
    app.add_systems(
        FixedUpdate,
        thruster_impulse_system.in_set(SpaceshipSectionSystems),
    );
    app.add_systems(FixedUpdate, stale_probe.after(PDControllerSystems::Sync));
    app.finish();

    let (ship, _controller) = diag_ship(&mut app);
    // 30 deg off the nose: the align phase slews the command every
    // tick, which is exactly when staleness shows.
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::GotoPos {
            position: Vec3::new(300.0, 0.0, -600.0),
        }));
    for _ in 0..120 {
        app.update();
    }

    let trace = app.world().resource::<StaleTrace>();
    // Delivery guards: the probe must have sampled real ticks and the
    // command must actually have been slewing - a parked command is
    // stale-proof by construction and would prove nothing.
    assert!(trace.samples > 100, "probe sampled {} ticks", trace.samples);
    assert!(
        trace.max_cmd_step > 5e-3,
        "the command must actually slew during the align phase \
         (max step {})",
        trace.max_cmd_step
    );
    // Bound sits above f32 Quat::angle_between noise (acos of a dot
    // near 1.0 floors around 1e-3 for identical rotations) and an
    // order of magnitude below the smallest stale-wiring reading
    // (0.048 in this rig; 0.22 during a full flip).
    assert!(
        trace.max_angle < 5e-3,
        "the PD must consume the command written THIS tick; max phantom \
         error {} rad",
        trace.max_angle
    );
}
