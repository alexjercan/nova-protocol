//! The manual burn: thrust balancing on an off-center or damage-shifted
//! hull, the soft speed cap, and the impulse-frame regressions.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_gameplay::{prelude::*, test_support::settle};

use super::support::*;
use crate::prelude::*;
/// A lone off-center engine at full burn still pulls, and a centered drive
/// stays held. This is the balancer's no-headroom floor: differential throttle
/// scales an engine's magnitude, not its line of action, so a single engine
/// cannot null its own torque - and a full-throttle demand pins it at 1.0 with
/// nothing to trim against. The PD holds only within the torque implied by its
/// acceleration authority and the live inertia; past it the ship pulls. When
/// there is more than one forward engine and throttle headroom, the balancer holds the
/// heading instead - see
/// `balanced_partial_burn_holds_an_off_center_twin_drive`.
#[test]
fn off_center_burn_pulls_but_a_centered_drive_is_held() {
    let drift_after_burn = |thruster_x: f32| -> f32 {
        let mut app = flight_app();
        let (ship, thruster, controller) = spawn_ship(&mut app);
        app.world_mut()
            .get_mut::<PDController>(controller)
            .unwrap()
            .max_angular_acceleration = 0.5; // shipped acceleration authority
        app.world_mut()
            .get_mut::<Transform>(thruster)
            .unwrap()
            .translation
            .x = thruster_x;
        settle(&mut app);
        app.world_mut().get_mut::<FlightIntent>(ship).unwrap().burn = 1.0;
        for _ in 0..120 {
            app.update();
        }
        app.world()
            .get::<Rotation>(ship)
            .unwrap()
            .0
            .angle_between(Quat::IDENTITY)
    };

    let held = drift_after_burn(0.0);
    assert!(
        held < 0.15,
        "a centered main drive must stay held by the PD ({held} rad drift)"
    );
    let pulled = drift_after_burn(2.0);
    assert!(
        pulled > 0.4,
        "an engine 2 units off the centerline must out-torque the computer \
         ({pulled} rad drift)"
    );
}

/// Thrust balancing: a drive that is off-center about the live COM pulls at
/// full throttle (no spare thrust to trim with, held only by the PD) but a
/// partial burn - which leaves the flight computer throttle headroom - is split
/// into a differential throttle that nulls the net torque, so the ship tracks
/// its heading like a centered drive. Two forward engines at unequal lever arms
/// make the ship genuinely off-center; only the throttle headroom differs
/// between the two cases.
#[test]
fn balanced_partial_burn_holds_an_off_center_twin_drive() {
    // Two forward (thrust -Z) engines at x = +4 and x = -1. The four unit
    // sections put the COM at x = (0 + 0 + 4 - 1)/4 = 0.75, so the lever
    // arms are 3.25 and 1.75 - a uniform throttle nets ~1.5 units of
    // torque, well past the PD's hold. With headroom the balancer runs the
    // near engine hotter so 3.25*near = 1.75*far and the net torque is 0.
    let drift_after_burn = |burn: f32| -> f32 {
        let mut app = flight_app();
        let ship = app
            .world_mut()
            .spawn((
                RigidBody::Dynamic,
                Transform::default(),
                SpaceshipRootMarker,
                FlightIntent::default(),
            ))
            .id();
        app.world_mut().spawn((
            ChildOf(ship),
            Name::new("hull"),
            Transform::from_xyz(0.0, 0.0, -1.0),
            Collider::cuboid(1.0, 1.0, 1.0),
            ColliderDensity(1.0),
        ));
        for x in [4.0f32, -1.0] {
            app.world_mut().spawn((
                ChildOf(ship),
                Name::new("thruster"),
                ThrusterSectionMarker,
                ThrusterSectionMagnitude(1.0),
                ThrusterSectionInput(0.0),
                Transform::from_xyz(x, 0.0, 1.0),
                Collider::cuboid(1.0, 1.0, 1.0),
                ColliderDensity(1.0),
            ));
        }
        app.world_mut().spawn((
            ChildOf(ship),
            Name::new("controller"),
            ControllerSectionMarker,
            ControllerSectionRotationInput::default(),
            PDController {
                frequency: 4.0,
                damping_ratio: 4.0,
                max_angular_acceleration: 0.5, // shipped acceleration authority
                sustained_angular_speed: f32::INFINITY,
            },
            PDControllerTarget(ship),
            Transform::from_xyz(0.0, 0.0, 0.0),
            Collider::cuboid(1.0, 1.0, 1.0),
            ColliderDensity(1.0),
        ));
        settle(&mut app);
        app.world_mut().get_mut::<FlightIntent>(ship).unwrap().burn = burn;
        for _ in 0..120 {
            app.update();
        }
        app.world()
            .get::<Rotation>(ship)
            .unwrap()
            .0
            .angle_between(Quat::IDENTITY)
    };

    // A 40% burn leaves ample headroom: the balancer nulls the torque and
    // the ship holds its heading within the centered-drive tolerance.
    let balanced = drift_after_burn(0.4);
    assert!(
        balanced < 0.15,
        "a partial burn must balance the off-center twin drive \
         ({balanced} rad drift)"
    );
    // A full-stick burn pins both engines at 1.0 - no headroom to trim -
    // so the same ship pulls, exactly the balancer's documented floor.
    let full = drift_after_burn(1.0);
    assert!(
        full > 0.4,
        "a full-throttle burn has no headroom to trim and still pulls \
         ({full} rad drift)"
    );
}

/// The off-axis counter-torque case: a single main drive on a damage-shifted
/// hull cannot balance itself by differential throttle (there is nothing in the
/// firing set to trim against), but the allocator recruits the surviving
/// lateral purely for its counter-torque and the ship holds its heading within
/// the centered-drive tolerance - even at full stick, because the recruit's
/// trim budget is its own throttle, not the main drive's headroom. Without the
/// lateral the same hull pulls, exactly the pre-allocation floor.
#[test]
fn single_drive_on_a_shifted_hull_recruits_a_lateral_to_hold_heading() {
    let burn_outcome = |with_lateral: bool| -> (f32, f32) {
        let mut app = flight_app();
        let (ship, lateral) = spawn_damage_shifted_single_drive(&mut app, with_lateral);
        settle(&mut app);
        app.world_mut().get_mut::<FlightIntent>(ship).unwrap().burn = 1.0;
        for _ in 0..120 {
            app.update();
        }
        let drift = app
            .world()
            .get::<Rotation>(ship)
            .unwrap()
            .0
            .angle_between(Quat::IDENTITY);
        let recruit = if with_lateral {
            **app.world().get::<ThrusterSectionInput>(lateral).unwrap()
        } else {
            0.0
        };
        (drift, recruit)
    };

    let (held, recruit) = burn_outcome(true);
    assert!(
        held < 0.15,
        "the recruited lateral must hold the heading ({held} rad drift)"
    );
    assert!(
        recruit > 0.2,
        "the lateral must actually be firing for counter-torque \
         (input {recruit})"
    );
    let (pulled, _) = burn_outcome(false);
    assert!(
        pulled > 0.4,
        "without a lateral to recruit the shifted hull must still pull \
         ({pulled} rad drift)"
    );
}

/// The same recruitment through the autopilot path: a STOP burn on the
/// damage-shifted single-drive hull lights the lateral (it is outside the
/// firing cone, recruited by the wrench allocation in the world frame),
/// and the maneuver still converges to rest - the recruit's sideways
/// force is the decided bounded drift, and chasing it down is exactly
/// what the autopilot's velocity-error rule does. Heading straightness
/// under a fixed burn is pinned by the manual-path test above; here the
/// hull deliberately turns to kill the drift, so rest is the invariant.
#[test]
fn autopilot_burn_recruits_a_lateral_on_a_shifted_hull() {
    let mut app = flight_app();
    let (ship, lateral) = spawn_damage_shifted_single_drive(&mut app, true);
    settle(&mut app);
    // Moving backward (+Z): STOP's velocity error points -Z, straight
    // along the main drive - no rotation needed, the burn starts at once.
    // Enough speed that the deceleration takes long enough for the
    // spooled inputs to be observable mid-burn.
    app.world_mut().get_mut::<LinearVelocity>(ship).unwrap().0 = Vec3::new(0.0, 0.0, 20.0);
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::Stop));
    // Sample DURING the burn - the autopilot stops the ship and winds the
    // engines down, so an after-the-fact reading would see only zeros.
    let mut recruit = 0.0f32;
    let mut frames = 0;
    while app.world().get::<Autopilot>(ship).is_some() && frames < 1500 {
        app.update();
        frames += 1;
        recruit = recruit.max(**app.world().get::<ThrusterSectionInput>(lateral).unwrap());
    }
    assert!(
        recruit > 0.2,
        "the autopilot must recruit the lateral for counter-torque \
         (peak input {recruit})"
    );
    assert!(
        app.world().get::<Autopilot>(ship).is_none(),
        "STOP must converge to rest despite the recruit's bounded drift \
         (speed {} after {frames} frames)",
        velocity_of(&app, ship).length()
    );
    // The recruit's sideways push leaves a LATERAL residual, and the
    // settle band's contract is that sub-band crumbs off the drive axis are
    // released, not hunted with attitude flips - so rest here means within
    // the settle band, not the old
    // 0.5. The shipped single-centered-drive ship keeps its exact rest:
    // an axial residual keeps the drive's aligned authority, so release
    // still waits for stop_speed_epsilon.
    let settle_band = app.world().resource::<FlightSettings>().settle_deadband;
    assert!(
        velocity_of(&app, ship).length() < settle_band + 0.05,
        "the ship must rest within the settle band ({:?})",
        velocity_of(&app, ship)
    );
}

/// The soft manual speed cap: a held full burn levels off just past the cap -
/// the overshoot is the spool-down tail, bounded by accel / spool_down_rate -
/// while the SAME ship uncapped blows straight past it. The uncapped leg is the
/// delivery guard proving the burn itself works AND the measured acceleration
/// the overshoot bound derives from (this rig is deliberately over-powered; the
/// physics-derived bound keeps the assertion honest instead of hardcoding a
/// slack constant).
#[test]
fn manual_burn_levels_off_at_the_speed_cap() {
    const CAP: f32 = 3.0;
    const FRAMES: usize = 1200;

    let run_ship = |cap: Option<f32>| -> (f32, f32) {
        let mut app = flight_app();
        let (ship, ..) = spawn_ship(&mut app);
        app.world_mut()
            .entity_mut(ship)
            .insert(FlightIntent { burn: 1.0 });
        if let Some(cap) = cap {
            app.world_mut().entity_mut(ship).insert(FlightSpeedCap(cap));
        }
        run(&mut app, FRAMES / 2);
        let mid = velocity_of(&app, ship).length();
        run(&mut app, FRAMES / 2);
        (mid, velocity_of(&app, ship).length())
    };

    let (uncapped_mid, uncapped) = run_ship(None);
    assert!(
        uncapped > CAP + 2.0,
        "delivery guard: the uncapped burn must sail past the cap, got {uncapped}"
    );
    // Measured acceleration of THIS rig, from the uncapped leg.
    let accel = uncapped_mid / (FRAMES as f32 / 2.0 / 60.0);

    let (capped_mid, capped) = run_ship(Some(CAP));
    // Overshoot bound: the spool-down tail keeps pushing for ~1/spool_down_rate
    // after the taper cuts the command, plus a couple of ticks of
    // taper-crossing.
    let settings = FlightSettings::default();
    let bound = CAP + accel * (1.0 / settings.spool_down_rate + 2.0 / 60.0) + 0.2;
    assert!(
        capped <= bound,
        "a capped ship levels off near the cap: got {capped}, bound {bound} \
         (cap {CAP}, measured accel {accel})"
    );
    assert!(
        capped >= CAP * 0.5,
        "the cap is a ceiling, not a parking brake: got {capped} vs cap {CAP}"
    );
    assert!(
        (capped - capped_mid).abs() < 0.05,
        "the capped ship has PLATEAUED, not still accelerating: {capped_mid} -> {capped}"
    );
}

/// Regression: the shipped 5-section player geometry (all sections on the z
/// axis, unit masses, single rear drive at z = +2, PD at the shipped 4/4/40)
/// holding the reverse direction from 300 u/s - the exact "wobbles when
/// decelerating" playtest scenario. The diagnostic trace measured the hull DEAD
/// STEADY here (max spin 0.0023 rad/s through flip + full 22 s burn), ruling
/// out a physical mechanism; this pins that so any future speed-coupled torque
/// regression (the stale-impulse-point family) fails loudly.
#[test]
fn hold_reverse_decel_from_300_keeps_the_hull_steady() {
    let mut app = flight_app();
    let ship = app
        .world_mut()
        .spawn((
            RigidBody::Dynamic,
            Transform::default(),
            TransformInterpolation,
            SpaceshipRootMarker,
            FlightIntent::default(),
        ))
        .id();
    let section = |app: &mut App, name: &str, z: f32| {
        app.world_mut()
            .spawn((
                ChildOf(ship),
                Name::new(name.to_string()),
                Transform::from_xyz(0.0, 0.0, z),
                Collider::cuboid(1.0, 1.0, 1.0),
                ColliderDensity(1.0),
            ))
            .id()
    };
    let controller = section(&mut app, "controller", 0.0);
    app.world_mut().entity_mut(controller).insert((
        ControllerSectionMarker,
        ControllerSectionRotationInput::default(),
        PDController {
            frequency: 4.0,
            damping_ratio: 4.0,
            max_angular_acceleration: 0.5,
            sustained_angular_speed: f32::INFINITY,
        },
        PDControllerTarget(ship),
    ));
    section(&mut app, "hull_front", 1.0);
    section(&mut app, "hull_back", -1.0);
    let thruster = section(&mut app, "thruster", 2.0);
    app.world_mut().entity_mut(thruster).insert((
        ThrusterSectionMarker,
        ThrusterSectionMagnitude(1.0),
        ThrusterSectionInput(0.0),
    ));
    section(&mut app, "turret_mass", -2.0);
    settle(&mut app);

    // Phase 1: cruising nose-first at 300 u/s, the player flips the
    // command to retrograde (mouse still afterwards: command constant).
    app.world_mut()
        .entity_mut(ship)
        .insert(LinearVelocity(Vec3::NEG_Z * 300.0));
    app.world_mut()
        .get_mut::<ControllerSectionRotationInput>(controller)
        .unwrap()
        .0 = Quat::from_rotation_y(std::f32::consts::PI);

    // A 0.5 rad/s2 controller needs about five ideal seconds for a 180;
    // the fixed setpoint also needs damping time before the burn starts.
    run(&mut app, 900);
    // Delivery guard: the flip must actually have happened, or the
    // steady-burn bound below is vacuous.
    assert!(
        forward_of(&app, ship).dot(Vec3::Z) > 0.999,
        "the command flip must complete before the burn phase"
    );

    // Phase 2: hold full reverse burn until (near) rest.
    app.world_mut().get_mut::<FlightIntent>(ship).unwrap().burn = 1.0;
    let mut max_spin_burn = 0.0f32;
    for _ in 0..3600 {
        app.update();
        let spin = app.world().get::<AngularVelocity>(ship).unwrap().length();
        max_spin_burn = max_spin_burn.max(spin);
        if velocity_of(&app, ship).length() < 1.0 {
            break;
        }
    }
    // Delivery guard: the burn must have delivered the deceleration.
    let speed = velocity_of(&app, ship).length();
    assert!(
        speed < 1.0,
        "the reverse burn must bring 300 u/s to rest, got {speed}"
    );
    assert!(
        max_spin_burn < 0.05,
        "the hull must stay steady while decelerating, max spin {max_spin_burn} rad/s"
    );
}

/// The impulse system must push from the raw physics pose, not the render pose.
/// In FixedUpdate, `GlobalTransform` is the PREVIOUS frame's propagation -
/// since the interpolation opt-in an eased pose one to two ticks behind raw
/// physics. A lateral engine whose thrust line passes exactly through the COM
/// adds zero true torque, but pushed from a point ~`v * dt` behind a fast hull
/// it torques the ship every tick: the high-speed twitch/flip of the playtest.
/// At 150 u/s the stale point trails ~2.3 u, which spun this rig past 1 rad/s
/// within a handful of frames before the fix.
#[test]
fn high_speed_lateral_burn_through_the_com_adds_no_spin() {
    let mut app = flight_app();
    let (ship, thruster) = spawn_uncontrolled_dumbbell_with_com_lateral(&mut app);
    settle(&mut app);

    app.world_mut()
        .entity_mut(ship)
        .insert(LinearVelocity(Vec3::Z * 150.0));
    app.world_mut()
        .get_mut::<ThrusterSectionInput>(thruster)
        .unwrap()
        .0 = 1.0;

    run(&mut app, 60);

    let spin = app.world().get::<AngularVelocity>(ship).unwrap().length();
    assert!(
        spin < 0.05,
        "a thrust line through the COM must not spin the hull, got {spin} rad/s"
    );
}

/// The playtest symptom: at high velocity the hull itself twitched - real
/// attitude jitter, not a camera artifact. The mechanism was the stale impulse
/// point, which only bites when the thrust has a component PERPENDICULAR to the
/// travel (a decel path with drift correction), so the faithful rig is a full
/// production stack burning across its own velocity: PD at the shipped 40
/// acceleration authority, TransformInterpolation on the hull, centered drive, high
/// cross velocity, zero rotation command. Against the pre-fix impulse code this
/// rig's PD is overwhelmed by ~2.3 u of application-point error per tick and
/// the max observed spin runs away past 1 rad/s; a steady hull must stay at
/// zero the whole run.
#[test]
fn cross_velocity_burn_keeps_the_hull_steady_at_high_speed() {
    let mut app = flight_app();
    let (ship, _, controller) = spawn_ship(&mut app);
    // Production-faithful scheduling: clock-bug rigs must mirror the
    // production interpolation opt-in.
    app.world_mut()
        .entity_mut(ship)
        .insert(TransformInterpolation);
    app.world_mut()
        .get_mut::<PDController>(controller)
        .unwrap()
        .max_angular_acceleration = 0.5; // shipped acceleration authority
    settle(&mut app);

    // Fast cross-travel (+X) under a full forward burn (-Z): thrust
    // perpendicular to velocity, the regime where a stale application
    // point torques the hull.
    app.world_mut()
        .entity_mut(ship)
        .insert(LinearVelocity(Vec3::X * 150.0));
    app.world_mut().get_mut::<FlightIntent>(ship).unwrap().burn = 1.0;

    let mut max_spin = 0.0f32;
    for _ in 0..180 {
        app.update();
        max_spin = max_spin.max(app.world().get::<AngularVelocity>(ship).unwrap().length());
    }
    // Delivery guard: a steady hull only proves the fix if the engine actually
    // fired - a silent burn seam would pass the spin bound vacuously. Three
    // seconds of full burn must have accelerated the ship along -Z.
    let burned = velocity_of(&app, ship).z;
    assert!(
        burned < -20.0,
        "the -Z main drive must have delivered thrust, got vz {burned}"
    );
    assert!(
        max_spin < 0.05,
        "zero rotation command + centered drive must hold the hull steady \
         at speed, max spin {max_spin} rad/s"
    );
}

#[test]
fn manual_burn_accelerates_and_is_ignored_while_engaged() {
    let mut app = flight_app();
    let (ship, _, _) = spawn_ship(&mut app);
    withhold_rcs(&mut app, ship);
    settle(&mut app);

    // Manual: analog burn accelerates along the nose.
    app.world_mut().get_mut::<FlightIntent>(ship).unwrap().burn = 1.0;
    run(&mut app, 120);
    let manual_speed = velocity_of(&app, ship).length();
    assert!(
        velocity_of(&app, ship).z < -1.0,
        "manual burn should accelerate"
    );

    // Engaged with the burn value still set (in the real game holding W
    // would disengage via the input observer; this pins that the manual
    // *system* never drives an engaged ship): the ship must stop
    // accelerating and start the maneuver instead of burning on.
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::Stop));
    run(&mut app, 120);
    let engaged_speed = velocity_of(&app, ship).length();
    // Allowance: the pre-engage burn spools down over ~0.4s and the hull
    // swings through partly-forward attitudes while the slewed command
    // ramps. Still burning at full manual throttle would have added
    // ~26 u/s over these ticks.
    assert!(
        engaged_speed < manual_speed + 3.0,
        "an engaged ship must not keep accelerating from stale manual burn \
         ({manual_speed} -> {engaged_speed})"
    );

    // Pilot lets go; STOP runs to completion and hands back a resting ship.
    app.world_mut().get_mut::<FlightIntent>(ship).unwrap().burn = 0.0;
    run(&mut app, 1200);
    let speed = velocity_of(&app, ship).length();
    assert!(speed < 0.5, "STOP should reach rest, got {speed}");
    assert!(app.world().get::<Autopilot>(ship).is_none());
}

/// The manual cap is one TOTAL-speed budget, not a budget per heading: a ship
/// already crossing at the cap gets nothing more from a full burn, and one
/// crossing at half the cap levels off at the cap rather than adding a second
/// cap's worth along the nose. Under the old along-burn gate the crossing
/// component was invisible to the taper, so turning and burning again reached
/// `sqrt(2) * cap`.
#[test]
fn manual_burn_spends_one_total_speed_budget_whatever_the_heading() {
    const CAP: f32 = 20.0;

    // The nose points -Z, so a +X velocity is pure crossing speed - exactly
    // what a pilot carries after building speed and then turning.
    let speed_after_burn = |crossing: f32| -> f32 {
        let mut app = flight_app();
        let (ship, ..) = spawn_ship(&mut app);
        settle(&mut app);
        app.world_mut().entity_mut(ship).insert((
            FlightSpeedCap(CAP),
            FlightIntent { burn: 1.0 },
            LinearVelocity(Vec3::X * crossing),
        ));
        run(&mut app, 900);
        velocity_of(&app, ship).length()
    };

    let at_the_cap = speed_after_burn(CAP);
    assert!(
        at_the_cap <= CAP + 0.05,
        "a full burn across a ship already at the cap buys nothing \
         (got {at_the_cap}, cap {CAP})"
    );
    let half = speed_after_burn(0.5 * CAP);
    assert!(
        half > 0.5 * CAP + 1.0,
        "delivery guard: the burn must actually fire below the budget \
         (got {half})"
    );
    // The spool-down tail keeps pushing after the gate closes; the old
    // per-heading gate reached sqrt(0.25 + 1) * CAP = 22.4 and kept going.
    assert!(
        half <= CAP + 1.0,
        "and it must level off at the ONE budget, not add a fresh cap along \
         the nose (got {half}, cap {CAP})"
    );
}

/// Recovery: a ship carried past the cap can always burn its way back inside
/// it. The budget never blocks a burn that slows the ship down, at the cap or
/// far above it.
#[test]
fn manual_burn_brakes_a_ship_from_above_the_cap_back_inside_it() {
    const CAP: f32 = 20.0;
    let mut app = flight_app();
    let (ship, ..) = spawn_ship(&mut app);
    settle(&mut app);
    // Travelling +Z with the nose on -Z: a held burn is pure retro.
    app.world_mut().entity_mut(ship).insert((
        FlightSpeedCap(CAP),
        FlightIntent { burn: 1.0 },
        LinearVelocity(Vec3::Z * 3.0 * CAP),
    ));
    let mut slowest = f32::MAX;
    let mut fastest = 0.0f32;
    for _ in 0..900 {
        app.update();
        let speed = velocity_of(&app, ship).length();
        slowest = slowest.min(speed);
        fastest = fastest.max(speed);
    }
    assert!(
        slowest < 1.0,
        "an overspeed ship must be able to brake all the way back to rest \
         (slowest {slowest}, cap {CAP})"
    );
    // Past rest the held burn is prograde again, and the budget catches it at
    // the cap - never at the speed it started overspeed with.
    assert!(
        fastest <= 3.0 * CAP + 0.05,
        "braking must not be answered with a bigger budget (fastest {fastest})"
    );
    let speed = velocity_of(&app, ship).length();
    assert!(
        speed <= CAP + 1.0,
        "and it settles on the one budget once it is back inside it \
         (got {speed}, cap {CAP})"
    );
}
