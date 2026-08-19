//! The attitude model, measured rather than felt: onset, peak rate, overshoot,
//! settling and residual spin for a commanded turn, across controller counts
//! and hull sizes.
//!
//! The table exists to show SIZE, which is the thing the flat acceleration
//! model had none of. A small hull is structure-bound and sharp and gains
//! nothing from more computers; a big enough hull is torque-bound and buys its
//! physics back one computer at a time until the structure catches it.
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
    sections::controller_section::update_controller_section_rotation_input,
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
        slew_helm
            .in_set(NovaFlightSystems)
            // Pinned inside the flight chain: `flight_app` runs the stack split
            // ahead of the whole chain, the helm reads the ceiling it wrote and
            // feeds the command copy, and the autopilot writes the same command
            // (idle here, but ambiguous ordering against a writer is a trap
            // either way).
            .after(autopilot_system)
            .before(update_controller_section_rotation_input),
    );
    app
}

/// A hull of `hull_sections` unit cuboids strung along z at `density`,
/// carrying `controllers` flight computers at the origin.
///
/// The computers are massless so the table isolates the stack itself; a real
/// one is a section like any other, and its mass at radius is one more reason
/// the tenth is not worth mounting. `density` is how the table gets a
/// TORQUE-bound hull without a hundred colliders: inertia scales with it and
/// the structural arm does not, which is the same crossover a longer hull
/// reaches by growing.
fn spawn_stacked_ship(
    app: &mut App,
    hull_sections: usize,
    density: f32,
    controllers: usize,
) -> Entity {
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
            // `SectionMarker` + `SectionCollider` are what the structural arm
            // is measured over, exactly as on a scenario-built hull.
            SectionMarker,
            SectionCollider::Cuboid { size: Vec3::ONE },
            Transform::from_xyz(0.0, 0.0, first + index as f32),
            Collider::cuboid(1.0, 1.0, 1.0),
            ColliderDensity(density),
        ));
    }
    for _ in 0..controllers {
        app.world_mut().spawn((
            ChildOf(ship),
            Name::new("controller"),
            ControllerSectionMarker,
            ControllerSectionRotationInput::default(),
            ControllerSectionTuning {
                steering_lag: 0.5,
                // The shipped torque (`basic_controller_section`).
                max_torque: 1501.0,
            },
            PDController {
                frequency: 4.0,
                damping_ratio: 4.0,
                // Derived by the stack pass on the first tick; a bundle cannot
                // know a ceiling that belongs to the hull.
                max_angular_acceleration: 0.0,
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
fn measure(hull: Hull, controllers: usize, command: f32, seconds: f32) -> Maneuver {
    let mut app = helm_app();
    let ship = spawn_stacked_ship(&mut app, hull.sections, hull.density, controllers);
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

/// One row of the hull table.
#[derive(Clone, Copy, Debug)]
struct Hull {
    /// Display name, so a failure says which craft broke.
    name: &'static str,
    /// Unit cuboids strung along z. Also fixes the structural arm, at
    /// `sections / 2` world units.
    sections: usize,
    /// Collider density, which buys inertia without buying arm.
    density: f32,
    /// Whether one computer is enough to reach this hull's structural
    /// ceiling. The whole point of the table: the answer differs by size, and
    /// it decides whether stacking is felt at all.
    structure_bound: bool,
    /// Run length for a snap 90 and for a near-180 flip, seconds.
    seconds: (f32, f32),
}

/// The hull sizes the table walks. A fighter and a cruiser that both run out
/// of METAL before they run out of computer, and a barge that runs out of
/// computer first - the case the flat acceleration model could not express.
const HULLS: [Hull; 3] = [
    Hull {
        name: "fighter",
        sections: 3,
        density: 1.0,
        structure_bound: true,
        seconds: (6.0, 8.0),
    },
    Hull {
        name: "cruiser",
        sections: 15,
        density: 1.0,
        structure_bound: true,
        seconds: (10.0, 16.0),
    },
    Hull {
        name: "barge",
        sections: 15,
        density: 20.0,
        structure_bound: false,
        seconds: (18.0, 30.0),
    },
];
const STACKS: [usize; 4] = [1, 2, 4, 10];
/// The two turns: a snap 90 and a 170 degree flip (short of 180 so the
/// commanded direction stays unambiguous).
const TURNS: [f32; 2] = [90.0, 170.0];

/// The acceptance table. Prints with `--nocapture`; the assertions below it
/// are the contract it exists to hold.
#[test]
fn size_decides_the_turn_and_stacking_only_helps_a_torque_bound_hull() {
    println!(
        "{:>5} {:>8} {:>5} {:>8} {:>10} {:>8} {:>9} {:>8} {:>6} {:>9}",
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
        for hull in HULLS {
            let seconds = if turn > 90.0 {
                hull.seconds.1
            } else {
                hull.seconds.0
            };
            let name = hull.name;
            let mut baseline: Option<Maneuver> = None;
            for controllers in STACKS {
                let run = measure(hull, controllers, turn.to_radians(), seconds);
                println!(
                    "{turn:>5.0} {name:>8} {controllers:>5} {:>8.2} {:>10.1} {:>8.2} {:>9.2} \
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
                // bound is the barge on one computer - the heaviest, least
                // authoritative case in the table - and it is still moving
                // within half a second, on pure angular acceleration with no
                // lag term anywhere in the loop.
                assert!(
                    run.onset < 0.55,
                    "hull {name} x{controllers}: onset {} s is sluggish",
                    run.onset
                );
                // No stack may leave the hull ringing or buzzing: this is the
                // discrete-damping guard. A naive stack multiplies the D gain
                // by the section count, and past `kd * dt = 2` the PD
                // limit-cycles instead of parking.
                assert!(
                    run.residual < 0.5,
                    "hull {name} x{controllers}: {} deg/s left over - the stack \
                     is sitting on a limit cycle",
                    run.residual
                );
                assert!(
                    run.settle < seconds - 0.5,
                    "turn {turn} hull {name} x{controllers}: never settled within \
                     {seconds} s"
                );

                match baseline {
                    None => baseline = Some(run),
                    Some(one) => {
                        if hull.structure_bound {
                            // The hull's metal, not its computers, sets its
                            // ceiling - so a stack buys NO rate at all. This is
                            // the assertion the old x2 authority curve fails.
                            assert!(
                                run.peak_rate <= one.peak_rate * 1.05,
                                "structure-bound {name} x{controllers}: {} deg/s \
                                 past one computer's {} deg/s - the metal, not \
                                 the computer, is the limit",
                                run.peak_rate,
                                one.peak_rate
                            );
                        }
                        // The precision half is bought with rate on a SHORT
                        // turn, where the hull rides the arrest ramp the whole
                        // way and a stack starts braking earlier. Bounded, so a
                        // retune that made stacking a rate loss would fail here.
                        assert!(
                            run.peak_rate >= one.peak_rate * 0.8,
                            "hull {name} x{controllers}: {} deg/s gives up too \
                             much of {} deg/s",
                            run.peak_rate,
                            one.peak_rate
                        );
                        // Precision is not something a stack may spend: it must
                        // never land worse than one computer lands.
                        assert!(
                            run.overshoot <= one.overshoot + 0.2,
                            "hull {name} x{controllers}: overshoot grew from {} \
                             to {} deg",
                            one.overshoot,
                            run.overshoot
                        );
                        // Onset may slip by a tick or two and no more. On a
                        // structure-bound hull the stack buys no authority to
                        // pay for its earlier braking, so the precision split
                        // is a small unpaid cost there; a hull that sat there
                        // waiting would blow this.
                        assert!(
                            run.onset <= one.onset + 2.5 / 60.0,
                            "hull {name} x{controllers}: onset grew from {} to \
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

/// The complaint that opened the attitude work: a minimal hull used to turn at
/// exactly the rate a barge turned at. It must not any more, and the reason has
/// to be the hull rather than the computer - all three carry the same one.
///
/// Measured on TRAVERSE, how long the turn takes, rather than on peak rate: a
/// torque-starved hull lags its own slewed command and then catches up faster
/// than the command ever moved, so its peak flatters it.
#[test]
fn a_small_hull_out_turns_a_big_one_on_the_same_computer() {
    let flip = 170.0f32.to_radians();
    let traverse = |hull: Hull| measure(hull, 1, flip, hull.seconds.1).traverse;
    let fighter = traverse(HULLS[0]);
    let cruiser = traverse(HULLS[1]);
    let barge = traverse(HULLS[2]);

    assert!(
        cruiser > fighter * 1.4,
        "a 15-section cruiser must be far slower round than a 3-section \
         fighter: {cruiser} vs {fighter} s"
    );
    assert!(
        barge > cruiser * 1.4,
        "and the barge slower again than the cruiser: {barge} vs {cruiser} s"
    );
}

/// Fitting computers to a hull that cannot turn: torque adds with no curve, so
/// the second one is worth a whole second one - and then the structure catches
/// the stack, and the wall it stops at is the one a LIGHT hull of the same
/// length is already up against.
#[test]
fn a_torque_bound_hull_buys_its_physics_back_and_no_more() {
    let barge = HULLS[2];
    let (seconds, flip) = (barge.seconds.1, 170.0f32.to_radians());
    let one = measure(barge, 1, flip, seconds);
    let two = measure(barge, 2, flip, seconds);
    let four = measure(barge, 4, flip, seconds);
    let ten = measure(barge, 10, flip, seconds);

    assert!(
        two.traverse < one.traverse * 0.85,
        "a second computer must be worth a whole second computer: {} -> {} s",
        one.traverse,
        two.traverse
    );
    assert!(
        ten.traverse > four.traverse * 0.98,
        "the structure must have caught the stack by four: {} -> {} s",
        four.traverse,
        ten.traverse
    );
    // The wall is the HULL's, not a curve somebody authored: the cruiser is the
    // same 15 sections at a twentieth of the mass, so it has the same arm and
    // the same structural ceiling, and the two land on the same number.
    let cruiser = measure(HULLS[1], 4, flip, HULLS[1].seconds.1);
    assert!(
        (four.peak_rate / cruiser.peak_rate - 1.0).abs() < 0.02,
        "a barge with enough computers must turn exactly like a light hull of \
         the same length: {} vs {} deg/s",
        four.peak_rate,
        cruiser.peak_rate
    );
}

/// The precision half of the stack, which SURVIVES the model change: a stack
/// brakes earlier and removes overshoot without materially delaying the turn.
/// The barge, where one computer overshoots enough for the difference to mean
/// something.
#[test]
fn stacking_reduces_overshoot_without_a_material_delay() {
    let hull = HULLS[2];
    let (seconds, snap) = (hull.seconds.0, 90.0f32.to_radians());
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
///
/// On the barge, because a torque-bound hull is the only one that FEELS the
/// loss - a structure-bound hull keeps its ceiling and notices nothing, which
/// is the model working and not a hole in the coverage.
#[test]
fn losing_one_of_two_computers_degrades_handling_instead_of_stranding_the_hull() {
    let barge = HULLS[2];
    let mut app = helm_app();
    let ship = spawn_stacked_ship(&mut app, barge.sections, barge.density, 2);
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

    let survivor = budget(&app);
    assert!(
        (stacked / survivor - 2.0).abs() < 5e-2,
        "two computers must carry twice the torque-bound ceiling: {stacked} -> \
         {survivor}"
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
