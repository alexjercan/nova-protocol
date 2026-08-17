//! Stacked flight computers, measured rather than felt: onset, peak rate,
//! overshoot, settling and residual spin for a commanded 90 degree turn,
//! across controller counts and hull sizes.
//!
//! The rig drives the SHIPPED helm - `ship_turn_rate` then `slew_rotation`,
//! the two lines the player's mouse path and the AI brain both run - with the
//! mouse replaced by a parked target attitude, so what it measures is the
//! production pipeline and not a second one built for the test.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_gameplay::{prelude::*, test_support::settle};

use super::support::*;
use crate::{
    flight::{autopilot::autopilot_system, ship_turn_rate, slew_rotation, NovaFlightSystems},
    prelude::*,
    sections::controller_section::{
        update_controller_section_rotation_input, update_controller_stack_tuning,
    },
};

/// Where the pilot is pointing. Parked for the whole run: the maneuver is a
/// single step command, which is what makes onset and overshoot legible.
#[derive(Resource, Default)]
struct Helm(Quat);

fn slew_helm(
    time: Res<Time>,
    settings: Res<FlightSettings>,
    helm: Res<Helm>,
    mut q_command: Query<
        (&mut ControllerSectionRotationInput, &ChildOf),
        With<ControllerSectionMarker>,
    >,
    q_computer: Query<
        (&PDController, &ChildOf),
        (
            With<ControllerSectionMarker>,
            Without<SectionInactiveMarker>,
        ),
    >,
    q_ship: Query<Entity, With<SpaceshipRootMarker>>,
) {
    for ship in &q_ship {
        let Some(turn_rate) = ship_turn_rate(
            q_computer
                .iter()
                .filter(|(_, &ChildOf(parent))| parent == ship)
                .map(|(pd, _)| pd.max_angular_acceleration),
            &settings,
        ) else {
            continue;
        };
        let max_step = turn_rate * time.delta_secs();
        for (mut command, _) in q_command
            .iter_mut()
            .filter(|(_, &ChildOf(parent))| parent == ship)
        {
            **command = slew_rotation(**command, helm.0, max_step);
        }
    }
}

fn helm_app() -> App {
    let mut app = flight_app();
    app.init_resource::<Helm>();
    app.add_systems(
        FixedUpdate,
        (update_controller_stack_tuning, slew_helm)
            .chain()
            .in_set(NovaFlightSystems)
            // Pinned inside the flight chain: the stack split feeds the helm's
            // turn-rate budget, the helm feeds the command copy, and the
            // autopilot writes the same command (idle here, but ambiguous
            // ordering against a writer is a trap either way).
            .after(autopilot_system)
            .before(update_controller_section_rotation_input),
    );
    app
}

/// A hull of `hull_sections` unit cuboids strung along z carrying
/// `controllers` flight computers at the origin. The computers are massless so
/// the table isolates the stack itself; a real one is a section like any
/// other, and its mass is one more reason the tenth is not worth mounting.
fn spawn_stacked_ship(app: &mut App, hull_sections: usize, controllers: usize) -> Entity {
    let ship = app
        .world_mut()
        .spawn((
            RigidBody::Dynamic,
            Transform::default(),
            SpaceshipRootMarker,
            FlightIntent::default(),
        ))
        .id();
    let first = -((hull_sections as f32) - 1.0) * 0.5;
    for index in 0..hull_sections {
        app.world_mut().spawn((
            ChildOf(ship),
            Name::new("hull"),
            Transform::from_xyz(0.0, 0.0, first + index as f32),
            Collider::cuboid(1.0, 1.0, 1.0),
            ColliderDensity(1.0),
        ));
    }
    for _ in 0..controllers {
        app.world_mut().spawn((
            ChildOf(ship),
            Name::new("controller"),
            ControllerSectionMarker,
            ControllerSectionRotationInput::default(),
            ControllerSectionTuning {
                frequency: 4.0,
                damping_ratio: 4.0,
                // The shipped budget (`basic_controller_section`).
                max_angular_acceleration: 0.5,
            },
            PDController {
                frequency: 4.0,
                damping_ratio: 4.0,
                max_angular_acceleration: 0.5,
            },
            PDControllerTarget(ship),
            Transform::default(),
        ));
    }
    ship
}

/// One commanded turn, in the numbers that decide whether stacking works.
#[derive(Debug, Clone, Copy)]
struct Maneuver {
    /// Seconds from the command to the hull having turned one degree - the
    /// anti-sluggish number. A heavy hull may turn slowly; it must not sit
    /// there first.
    onset: f32,
    /// Highest angular rate reached, deg/s - the diminishing-returns number.
    peak_rate: f32,
    /// Seconds until the hull first gets within five degrees of the command:
    /// how long the turn takes, as opposed to how fast it peaks.
    traverse: f32,
    /// How far past the commanded attitude the hull sailed, degrees.
    overshoot: f32,
    /// Seconds until the hull is inside a degree of the command and under
    /// 0.05 rad/s for good - "visibly stopped where it was pointed", not an
    /// instrument-grade tolerance.
    settle: f32,
    /// How often the attitude error changed sign AFTER the hull first
    /// reached the command - a ringing count. One clean overshoot that decays
    /// inside the settled band scores zero; a hull that swings back through
    /// and out again scores one per swing.
    reversals: usize,
    /// Angular rate left at the end of the run, deg/s. A stack that
    /// destabilized the PD would sit on a limit cycle here instead of at rest.
    residual: f32,
}

/// Signed shortest step from `previous` to `yaw`, so the yaw series can be
/// accumulated across the +/-pi wrap.
fn wrapped_delta(previous: f32, yaw: f32) -> f32 {
    let delta = (yaw - previous).rem_euclid(core::f32::consts::TAU);
    if delta > core::f32::consts::PI {
        delta - core::f32::consts::TAU
    } else {
        delta
    }
}

/// Command a yaw of `command` radians - about the hull's LARGEST principal
/// axis, the one `hull_turn_rate` budgets against - and watch it land.
///
/// Turn size decides which half of the loop is being measured, so the table
/// runs both. A SHORT turn never reaches the commanded slew rate: the hull
/// spends the whole maneuver on the PD's arrest ramp, where rate tracks the
/// remaining error. A near-180 FLIP is long enough to settle into tracking
/// the command, which is where the turn-rate budget - and therefore the
/// authority curve - shows up.
fn measure(hull_sections: usize, controllers: usize, command: f32, seconds: f32) -> Maneuver {
    let mut app = helm_app();
    let ship = spawn_stacked_ship(&mut app, hull_sections, controllers);
    settle(&mut app);

    app.world_mut().resource_mut::<Helm>().0 = Quat::from_rotation_y(command);

    let steps = (seconds * 60.0) as usize;
    let dt = 1.0 / 60.0;
    let settled_angle = 1.0f32.to_radians();
    let settled_rate = 0.05;
    let (mut onset, mut peak_rate, mut peak_turn, mut last_outside) = (f32::NAN, 0.0f32, 0.0f32, 0);
    let (mut reversals, mut reached, mut sign) = (0usize, false, 1.0f32);
    let (mut turned, mut previous, mut residual) = (0.0f32, 0.0f32, 0.0);
    let mut traverse = f32::NAN;
    for step in 0..steps {
        app.update();
        let yaw = app
            .world()
            .get::<Rotation>(ship)
            .unwrap()
            .to_euler(EulerRot::YXZ)
            .0;
        // Unwrapped, so an overshoot past a near-180 command reads as more
        // travel rather than as a jump to the far side of the circle.
        turned += wrapped_delta(previous, yaw);
        previous = yaw;
        let rate = angular_speed_of(&app, ship);
        residual = rate;
        peak_rate = peak_rate.max(rate);
        peak_turn = peak_turn.max(turned);
        if onset.is_nan() && turned.abs() >= 1.0f32.to_radians() {
            onset = step as f32 * dt;
        }
        let error = command - turned;
        if traverse.is_nan() && error.abs() <= 5.0f32.to_radians() {
            traverse = step as f32 * dt;
        }
        if error.abs() > settled_angle || rate > settled_rate {
            last_outside = step;
        }
        // The approach itself is one long "error is positive", and arriving
        // is one sign change nobody calls a wobble - so the count starts on
        // the far side of the command.
        if !reached && error <= 0.0 {
            reached = true;
            sign = -1.0;
        }
        if reached && error.abs() > settled_angle && error.signum() != sign {
            sign = error.signum();
            reversals += 1;
        }
    }

    Maneuver {
        onset,
        peak_rate: peak_rate.to_degrees(),
        traverse,
        overshoot: (peak_turn - command).max(0.0).to_degrees(),
        settle: (last_outside + 1) as f32 * dt,
        reversals,
        residual: residual.to_degrees(),
    }
}

/// Hull sizes the table walks, with the run length each turn needs. A
/// 3-section fighter, a 9-section cruiser, and a 15-section barge whose
/// largest principal inertia (~282) is two orders past the fighter's.
const HULLS: [(usize, f32, f32); 3] = [(3, 6.0, 8.0), (9, 12.0, 18.0), (15, 18.0, 30.0)];
const STACKS: [usize; 4] = [1, 2, 4, 10];
/// The two turns: a snap 90 and a 170 degree flip (short of 180 so the
/// commanded direction stays unambiguous).
const TURNS: [f32; 2] = [90.0, 170.0];

/// The acceptance table. Prints with `--nocapture`; the assertions below it
/// are the contract it exists to hold.
#[test]
fn stacking_turns_a_hull_better_with_diminishing_returns() {
    println!(
        "{:>5} {:>5} {:>5} {:>8} {:>10} {:>8} {:>9} {:>8} {:>6} {:>9}",
        "turn",
        "hull",
        "ctrl",
        "onset s",
        "peak d/s",
        "turn s",
        "over deg",
        "settle s",
        "rings",
        "residual"
    );
    for turn in TURNS {
        for (hull, snap_seconds, flip_seconds) in HULLS {
            let seconds = if turn > 90.0 {
                flip_seconds
            } else {
                snap_seconds
            };
            let mut baseline: Option<Maneuver> = None;
            for controllers in STACKS {
                let run = measure(hull, controllers, turn.to_radians(), seconds);
                println!(
                    "{turn:>5.0} {hull:>5} {controllers:>5} {:>8.2} {:>10.1} {:>8.2} {:>9.2} \
                     {:>8.2} {:>6} {:>9.3}",
                    run.onset,
                    run.peak_rate,
                    run.traverse,
                    run.overshoot,
                    run.settle,
                    run.reversals,
                    run.residual
                );

                // The hull answers the helm at once whatever it weighs. The
                // bound is the 15-section barge on one computer - the heaviest,
                // least authoritative case in the table - and it is still
                // moving within half a second, on pure angular acceleration
                // with no lag term anywhere in the loop.
                assert!(
                    run.onset < 0.55,
                    "hull {hull} x{controllers}: onset {} s is sluggish",
                    run.onset
                );
                // No stack may leave the hull ringing or buzzing: this is the
                // discrete-damping guard. A naive stack multiplies the D gain
                // by the section count, and past `kd * dt = 2` the PD
                // limit-cycles instead of parking.
                assert!(
                    run.residual < 0.5,
                    "hull {hull} x{controllers}: {} deg/s left over - the stack \
                     is sitting on a limit cycle",
                    run.residual
                );
                assert!(
                    run.settle < seconds - 0.5,
                    "turn {turn} hull {hull} x{controllers}: never settled within \
                     {seconds} s"
                );

                match baseline {
                    None => baseline = Some(run),
                    Some(one) => {
                        // Diminishing returns, hard: the whole stack is worth at
                        // most sqrt(2) of one computer's turn rate, because
                        // acceleration authority is capped at 2x and rate grows
                        // with its square root.
                        assert!(
                            run.peak_rate <= one.peak_rate * 1.45,
                            "hull {hull} x{controllers}: {} deg/s is past the \
                             sqrt(2) ceiling on {} deg/s",
                            run.peak_rate,
                            one.peak_rate
                        );
                        // The precision half is bought with rate on a SHORT
                        // turn, where the hull rides the arrest ramp the whole
                        // way and a stack starts braking earlier. Bounded, so a
                        // retune that made stacking a rate loss would fail here.
                        assert!(
                            run.peak_rate >= one.peak_rate * 0.8,
                            "hull {hull} x{controllers}: {} deg/s gives up too \
                             much of {} deg/s",
                            run.peak_rate,
                            one.peak_rate
                        );
                        // Precision is not something a stack may spend: it must
                        // never land worse than one computer lands.
                        assert!(
                            run.overshoot <= one.overshoot + 0.2,
                            "hull {hull} x{controllers}: overshoot grew from {} \
                             to {} deg",
                            one.overshoot,
                            run.overshoot
                        );
                        assert!(
                            run.onset <= one.onset + 1e-3,
                            "hull {hull} x{controllers}: onset grew from {} to \
                             {} s",
                            one.onset,
                            run.onset
                        );
                    }
                }
            }
        }
    }
}

/// The second computer must be a real upgrade and the tenth must not, on the
/// hull and turn where the turn-rate budget decides the outcome: a 9-section
/// cruiser flipping far enough to settle into tracking its commanded slew
/// instead of riding the arrest ramp the whole way.
#[test]
fn the_second_computer_is_worth_it_and_the_tenth_is_not() {
    let (hull, seconds, flip) = (9, 18.0, 170.0f32.to_radians());
    let one = measure(hull, 1, flip, seconds);
    let two = measure(hull, 2, flip, seconds);
    let four = measure(hull, 4, flip, seconds);
    let ten = measure(hull, 10, flip, seconds);

    assert!(
        two.peak_rate > one.peak_rate * 1.05,
        "a second computer must be felt: {} -> {} deg/s",
        one.peak_rate,
        two.peak_rate
    );
    assert!(
        ten.peak_rate < four.peak_rate * 1.08,
        "the tenth computer must be nearly pointless: {} -> {} deg/s",
        four.peak_rate,
        ten.peak_rate
    );
    // Turn rate grows with the square root of acceleration authority, so a 2x
    // authority cap gives a sqrt(2) rate cap.
    assert!(
        ten.peak_rate < one.peak_rate * 1.45,
        "the whole stack must stay under the sqrt(2) ceiling: {} -> {} deg/s",
        one.peak_rate,
        ten.peak_rate
    );
}

/// The precision half of the curve: a stack brakes earlier and removes
/// overshoot without materially delaying the turn.
#[test]
fn stacking_reduces_overshoot_without_a_material_delay() {
    let (hull, seconds, snap) = (15, 18.0, 90.0f32.to_radians());
    let one = measure(hull, 1, snap, seconds);
    let four = measure(hull, 4, snap, seconds);
    let ten = measure(hull, 10, snap, seconds);

    assert!(
        one.overshoot > 2.0,
        "one computer must overshoot for this to mean anything, got {} deg",
        one.overshoot
    );
    assert!(
        four.overshoot < one.overshoot * 0.5,
        "four computers must halve the overshoot: {} -> {} deg",
        one.overshoot,
        four.overshoot
    );
    assert_eq!(
        ten.reversals, 0,
        "a stacked hull must approach its command without ringing"
    );
    assert!(
        ten.settle <= one.settle,
        "the precision gain must not make settling slower: {} -> {} s",
        one.settle,
        ten.settle
    );
    assert!(
        ten.traverse <= one.traverse * 1.05,
        "the precision gain must not materially delay the turn: {} -> {} s",
        one.traverse,
        ten.traverse
    );
}

/// Redundancy in flight: a two-computer hull that loses one keeps flying on
/// the survivor, degraded to exactly single-computer handling rather than
/// stranded. The flight layer already drops the autopilot on the LAST
/// computer (`a_dead_flight_computer_disengages_the_autopilot`); this is the
/// other half of that rule.
#[test]
fn losing_one_of_two_computers_degrades_handling_instead_of_stranding_the_hull() {
    let mut app = helm_app();
    let ship = spawn_stacked_ship(&mut app, 9, 2);
    settle(&mut app);

    let controllers: Vec<Entity> = app
        .world_mut()
        .query_filtered::<Entity, With<ControllerSectionMarker>>()
        .iter(app.world())
        .collect();
    let budget = |app: &App| -> f32 {
        controllers
            .iter()
            .filter(|entity| app.world().get::<SectionInactiveMarker>(**entity).is_none())
            .filter_map(|entity| app.world().get::<PDController>(*entity))
            .map(|pd| pd.max_angular_acceleration)
            .sum()
    };
    let stacked = budget(&app);

    app.world_mut()
        .entity_mut(controllers[0])
        .insert(SectionInactiveMarker);
    app.world_mut().resource_mut::<Helm>().0 = Quat::from_rotation_y(core::f32::consts::FRAC_PI_2);
    run(&mut app, 600);

    assert!(
        (stacked - 0.75).abs() < 1e-3,
        "two computers carry 1.5 shares, got {stacked}"
    );
    assert!(
        (budget(&app) - 0.5).abs() < 1e-3,
        "the survivor must re-derive to exactly one computer, got {}",
        budget(&app)
    );
    // Degraded, not stranded: the hull still answers the command it was given
    // after the loss, and still parks on it.
    let yaw = app
        .world()
        .get::<Rotation>(ship)
        .unwrap()
        .to_euler(EulerRot::YXZ)
        .0;
    assert!(
        (yaw - core::f32::consts::FRAC_PI_2).abs() < 2.0f32.to_radians(),
        "the surviving computer must still fly the turn, stopped at {} deg",
        yaw.to_degrees()
    );
    assert!(
        angular_speed_of(&app, ship) < 0.02,
        "and still park the hull"
    );
}
