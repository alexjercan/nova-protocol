//! The manual burn: thrust balancing on an off-center or damage-shifted
//! hull, the Newtonian response at speed, and the impulse-frame regressions.

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
    disable_rcs(&mut app, ship);
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

/// The manual drive is Newtonian: a held burn accelerates the hull along its
/// nose at ANY velocity, on any heading. The rig is the case the old manual
/// speed governor killed - nose on -Z, the whole velocity on +X, so every bit
/// of the burn is across the travel. Under that governor the along-nose gain
/// over ten seconds of held full burn was EXACTLY zero (the velocity vector
/// came back bit-identical), because the burn was purely tangential to the
/// budget sphere; that was the reported control failure.
///
/// The same run pins the two properties that make an uncapped drive flyable:
/// a centered drive adds no spin at speed, and releasing the burn PRESERVES
/// the velocity it ended with instead of bleeding it off.
#[test]
fn manual_burn_accelerates_along_the_nose_at_any_speed() {
    const FRAMES: usize = 600;
    const CROSSING: f32 = 20.0;

    let mut app = flight_app();
    let (ship, thruster, _) = spawn_ship(&mut app);
    settle(&mut app);
    app.world_mut().entity_mut(ship).insert((
        FlightIntent { burn: 1.0 },
        LinearVelocity(Vec3::X * CROSSING),
    ));

    // THIS rig's full-throttle authority: a ThrusterSectionMagnitude is an
    // impulse per fixed tick, so magnitude / mass is the delta-v one tick of
    // full burn adds. The growth bound below is derived from it instead of a
    // hardcoded slack constant.
    let per_tick = **app
        .world()
        .get::<ThrusterSectionMagnitude>(thruster)
        .unwrap()
        / app.world().get::<ComputedMass>(ship).unwrap().value();

    let along_nose = |app: &App| -velocity_of(app, ship).z;
    let mut max_spin = 0.0f32;
    let burn_frames = |app: &mut App, frames: usize, max_spin: &mut f32| {
        for _ in 0..frames {
            app.update();
            *max_spin = max_spin.max(angular_speed_of(app, ship));
        }
    };

    burn_frames(&mut app, FRAMES / 2, &mut max_spin);
    let mid = along_nose(&app);
    burn_frames(&mut app, FRAMES - FRAMES / 2, &mut max_spin);
    let end = along_nose(&app);

    // Half the ideal impulse sum, which leaves the spool-up ramp room. The
    // governor delivered 0.0 into this same rig.
    let floor = 0.5 * per_tick * FRAMES as f32;
    assert!(
        end > floor,
        "a held burn must accelerate the hull along its nose while it crosses \
         at {CROSSING} u/s: got {end} u/s, floor {floor} ({per_tick} u/s per \
         tick over {FRAMES} frames)"
    );
    // And the answer must not FADE as the speed grows: the fully spooled
    // second half adds at least what the ramping first half did.
    assert!(
        end - mid >= mid,
        "the drive must answer the same fast as slow: first half +{mid}, \
         second half +{}",
        end - mid
    );
    assert!(
        max_spin < 0.05,
        "a centered drive must add no spin at speed, max {max_spin} rad/s"
    );
    // Thrust is the only force in the rig, so the crossing component is
    // untouched: turning and burning never spends the speed already carried.
    let crossing = velocity_of(&app, ship).x;
    assert!(
        (crossing - CROSSING).abs() < 1e-3,
        "the crossing component must survive the burn, got {crossing}"
    );

    // Release: the spool-down tail runs out and the hull COASTS.
    app.world_mut().get_mut::<FlightIntent>(ship).unwrap().burn = 0.0;
    run(&mut app, 120);
    let coasting = velocity_of(&app, ship);
    assert!(
        -coasting.z >= end,
        "the spool-down tail only ever adds, got {} vs {end}",
        -coasting.z
    );
    run(&mut app, 600);
    let later = velocity_of(&app, ship);
    assert!(
        (later - coasting).length() < 1e-3,
        "releasing the burn must preserve the velocity, not decay it: \
         {coasting:?} -> {later:?}"
    );
}

/// A balanced twin drive with one surviving lateral: two forward (-Z) engines
/// at equal and opposite lever arms about the live COM, so the uniform seed is
/// already torque-free and the lateral is never lit, plus a lateral mounted
/// forward of the COM that can counter-torque a lone drive. Returns
/// (ship, port main, starboard main, lateral).
fn spawn_balanced_twin_drive(app: &mut App) -> (Entity, Entity, Entity, Entity) {
    let ship = app
        .world_mut()
        .spawn((
            RigidBody::Dynamic,
            Transform::default(),
            SpaceshipRootMarker,
            FlightIntent::default(),
        ))
        .id();
    let mut block = |name: &str, transform: Transform, thruster: bool| -> Entity {
        let mut entity = app.world_mut().spawn((
            ChildOf(ship),
            Name::new(name.to_owned()),
            transform,
            Collider::cuboid(1.0, 1.0, 1.0),
            ColliderDensity(1.0),
        ));
        if thruster {
            entity.insert((
                ThrusterSectionMarker,
                ThrusterSectionMagnitude(1.0),
                ThrusterSectionInput(0.0),
            ));
        }
        entity.id()
    };
    block("hull", Transform::from_xyz(0.0, 0.0, -1.0), false);
    let port = block("port main", Transform::from_xyz(-2.0, 0.0, 1.0), true);
    let starboard = block("starboard main", Transform::from_xyz(2.0, 0.0, 1.0), true);
    // Thrust toward -X (local -Z rotated +90 degrees about Y), forward of the
    // COM: the only engine on the hull that can null a lone main's yaw.
    let lateral = block(
        "lateral",
        Transform::from_xyz(0.0, 0.0, -3.0)
            .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
        true,
    );
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
    (ship, port, starboard, lateral)
}

fn thruster_input(app: &App, thruster: Entity) -> f32 {
    **app.world().get::<ThrusterSectionInput>(thruster).unwrap()
}

/// The yaw torque `engines` currently impose about the ship's live COM, in the
/// body frame - the quantity the allocation exists to null. Engines the caller
/// leaves out are not part of the live set and contribute nothing.
fn live_yaw_torque(app: &App, ship: Entity, engines: &[Entity]) -> f32 {
    let com = app
        .world()
        .get::<ComputedCenterOfMass>(ship)
        .map_or(Vec3::ZERO, |c| c.0);
    engines
        .iter()
        .map(|&engine| {
            let transform = app.world().get::<Transform>(engine).unwrap();
            let magnitude = **app.world().get::<ThrusterSectionMagnitude>(engine).unwrap();
            let input = thruster_input(app, engine);
            let thrust = transform.rotation.mul_vec3(Vec3::NEG_Z) * magnitude * input;
            (transform.translation - com).cross(thrust).y
        })
        .sum()
}

/// Losing a drive updates the LIVE allocation set on the production
/// eligibility seam (`SectionInactiveMarker`, what neutralize writes and what
/// the burn query filters on): the severed engine drops out of the allocation
/// entirely - its input is never written again - and the burn is re-split
/// across what is left, recruiting the lateral that the intact balanced set
/// never needed. Focused sibling of
/// `single_drive_on_a_shifted_hull_recruits_a_lateral_to_hold_heading`, which
/// covers the same allocation on a hull that never had the second drive.
#[test]
fn severing_a_drive_reallocates_the_burn_across_the_surviving_live_set() {
    const BURN: f32 = 0.4;
    let mut app = flight_app();
    let (ship, port, starboard, lateral) = spawn_balanced_twin_drive(&mut app);
    settle(&mut app);
    app.world_mut().get_mut::<FlightIntent>(ship).unwrap().burn = BURN;
    run(&mut app, 120);

    // The starting set: balanced, multi-group, and already torque-free, so the
    // seed stands and the lateral group stays dark.
    let (port_input, starboard_input) =
        (thruster_input(&app, port), thruster_input(&app, starboard));
    assert!(
        (port_input - starboard_input).abs() < 1e-3 && port_input > 0.3,
        "a balanced twin drive splits the burn evenly (port {port_input}, \
         starboard {starboard_input})"
    );
    assert!(
        thruster_input(&app, lateral) < 1e-3,
        "a torque-free firing set recruits nothing (lateral {})",
        thruster_input(&app, lateral)
    );

    // Sever the starboard drive the way damage does, and ask for a bigger
    // burn in the same breath: a drive still in the set would follow the new
    // demand, so the frozen input below is the set membership, not the ramp.
    app.world_mut()
        .entity_mut(starboard)
        .insert(SectionInactiveMarker);
    app.world_mut().get_mut::<FlightIntent>(ship).unwrap().burn = 1.0;
    let severed_at = thruster_input(&app, starboard);
    app.update();

    assert_eq!(
        thruster_input(&app, starboard),
        severed_at,
        "a severed drive is out of the allocation set, not throttled by it"
    );
    assert!(
        thruster_input(&app, port) > port_input,
        "the surviving drive carries the whole new demand (port {} was {port_input})",
        thruster_input(&app, port)
    );
    assert!(
        thruster_input(&app, lateral) > 0.0,
        "the surviving set recruits its counter-torque on the next flight tick"
    );

    // And the re-split converges on a balanced surviving set: the live engines
    // null their own yaw, while the severed drive's frozen input is exactly
    // the torque the ship would carry if it were still being allocated to.
    run(&mut app, 120);
    let live = live_yaw_torque(&app, ship, &[port, lateral]);
    let lone = live_yaw_torque(&app, ship, &[port]);
    assert!(
        live.abs() < 0.05 * lone.abs(),
        "the surviving live set must balance itself (residual {live}, lone \
         drive {lone})"
    );
    let drift = app
        .world()
        .get::<Rotation>(ship)
        .unwrap()
        .0
        .angle_between(Quat::IDENTITY);
    assert!(
        drift < 0.15,
        "and the hull holds its heading on the surviving set ({drift} rad)"
    );
}
