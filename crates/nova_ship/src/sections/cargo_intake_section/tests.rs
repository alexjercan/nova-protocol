//! The intake's contract: what it takes, what it refuses, and what it drops.
//!
//! Real avian, because a take is avian's own sensor contact. Every fixture is
//! one hull, at rest at the origin until a canister hits it, with an intake
//! mounted 2 units down its -Z, so the door face is the hull-local plane
//! z = -2.5 and the trigger fills the capture gap in front of it.

use std::{collections::BTreeMap, f32::consts::FRAC_1_SQRT_2};

use nova_gameplay::test_support::{settle, unfinished_integrity_physics_app};

use super::*;

/// The door face plane of the fixture intake, hull-local, in world units.
const FACE_Z: f32 = -2.5;
const CAPTURE_GAP: Meters = Meters(1.0);
const APERTURE_WIDTH: Meters = Meters(22.2);

fn intake_config() -> CargoIntakeSectionConfig {
    CargoIntakeSectionConfig {
        render_mesh: AssetRef::from("intake.glb#Scene0"),
        render_mesh_transform: None,
        canister_mesh: AssetRef::from("canister.glb#Scene0"),
        door_sound: AssetRef::from("door.wav"),
        eject_sound: AssetRef::from("eject.wav"),
        take_sound: AssetRef::from("take.wav"),
        detection_range: Meters(40.0),
        capture_gap: CAPTURE_GAP,
        aperture_width: APERTURE_WIDTH,
        aperture_height: Meters(15.3),
        eject_speed: MetersPerSecond(3.0),
    }
}

/// How many [`CargoCanisterTaken`] events fired.
#[derive(Resource, Default)]
struct Takes(u32);

/// A headless app with one hull carrying `stock` hull plates in a 400 kg
/// hold and one intake at each hull-local x in `intakes_x`. Returns the app,
/// the ship and the intakes.
fn intakes_app(stock: u32, intakes_x: &[f32]) -> (App, Entity, Vec<Entity>) {
    let mut app = unfinished_integrity_physics_app();
    app.add_plugins((
        SectionAnimationPlugin,
        NovaRoundPlugin,
        CargoIntakeSectionPlugin { render: false },
    ));
    app.init_resource::<Takes>();
    // Not pulled in by CargoIntakeSectionPlugin itself: only NovaGameplayPlugin
    // owns it in a real app, which this minimal rig never adds.
    app.init_resource::<CargoCanisterIdAllocator>();
    app.add_observer(|_: On<CargoCanisterTaken>, mut takes: ResMut<Takes>| {
        takes.0 += 1;
    });
    app.finish();
    let ship = app
        .world_mut()
        .spawn((
            Name::new("hull"),
            SpaceshipRootMarker,
            RigidBody::Dynamic,
            Transform::default(),
            ShipInventory::new(400_000, [(ItemType::HullPlate, stock)]),
        ))
        .id();
    app.world_mut().spawn((
        ChildOf(ship),
        Transform::default(),
        Collider::cuboid(1.0, 1.0, 1.0),
        ColliderDensity(1.0),
    ));
    let collider = SectionCollider::Cuboid {
        size: Vec3::new(3.0, 2.0, 1.0),
    };
    let intakes = intakes_x
        .iter()
        .map(|&x| {
            app.world_mut()
                .spawn((
                    ChildOf(ship),
                    Name::new("intake"),
                    Transform::from_xyz(x, 0.0, -2.0),
                    collider,
                    collider.to_collider(),
                    ColliderDensity(1.0),
                    SectionAnimations::new(vec![SectionAnimation {
                        cue: SectionAnimationCue::IntakeDoor,
                        node_prefix: "intake_slat_".to_string(),
                        motion: SectionAnimationMotion::Fold {
                            degrees: 80.0,
                            slat_width: 0.2292,
                            slat_thickness: 0.02,
                        },
                        open_seconds: 1.2,
                        close_seconds: 1.2,
                    }]),
                    cargo_intake_section(intake_config()),
                ))
                .id()
        })
        .collect();
    settle(&mut app);
    (app, ship, intakes)
}

/// [`intakes_app`] with one intake on the hull's centre line.
fn intake_app(stock: u32) -> (App, Entity, Entity) {
    let (app, ship, intakes) = intakes_app(stock, &[0.0]);
    (app, ship, intakes[0])
}

/// A drifting canister of `count` hull plates at `at` with `rotation`,
/// moving at `velocity`.
fn drifting_canister(
    app: &mut App,
    count: u32,
    at: Vec3,
    rotation: Quat,
    velocity: Vec3,
) -> Entity {
    app.world_mut()
        .spawn(cargo_canister(
            CargoCanister::new(ItemType::HullPlate, count),
            Transform::from_translation(at).with_rotation(rotation),
            velocity,
            AssetRef::from("canister.glb#Scene0"),
        ))
        .id()
}

/// Update until `canister` is gone, at most `frames` times. Returns whether
/// it went.
fn run_until_gone(app: &mut App, canister: Entity, frames: u32) -> bool {
    for _ in 0..frames {
        if app.world().get_entity(canister).is_err() {
            return true;
        }
        app.update();
    }
    app.world().get_entity(canister).is_err()
}

fn plates(app: &App, ship: Entity) -> u32 {
    app.world()
        .get::<ShipInventory>(ship)
        .unwrap()
        .count(ItemType::HullPlate)
}

fn takes(app: &App) -> u32 {
    app.world().resource::<Takes>().0
}

fn door(app: &App, intake: Entity) -> f32 {
    app.world()
        .get::<SectionAnimations>(intake)
        .unwrap()
        .cue_progress(SectionAnimationCue::IntakeDoor)
        .unwrap()
}

fn canisters(app: &mut App) -> Vec<(Entity, CargoCanister, Vec3)> {
    let world = app.world_mut();
    let mut q = world.query::<(Entity, &CargoCanister, &Position)>();
    q.iter(world)
        .map(|(entity, canister, position)| (entity, canister.clone(), position.0))
        .collect()
}

/// A canister centre `gap` in front of the face for an unrotated canister.
fn in_front(gap: f32) -> f32 {
    FACE_Z - CARGO_CANISTER_SIZE.z * 0.5 - gap
}

#[test]
fn a_canister_in_the_trigger_is_taken_whatever_its_velocity() {
    let capture_gap = CAPTURE_GAP.to_engine();
    let cases = [
        (
            "drifting away inside the trigger",
            Vec3::new(0.0, 0.0, in_front(capture_gap * 0.5)),
            Vec3::new(0.0, 0.0, -0.05),
        ),
        (
            "at rest inside the trigger",
            Vec3::new(0.0, 0.0, in_front(capture_gap * 0.5)),
            Vec3::ZERO,
        ),
        (
            "closing at 0.5 m/s",
            Vec3::new(0.0, 0.0, in_front(0.3)),
            Vec3::new(0.0, 0.0, 0.05),
        ),
        (
            "closing at 20 m/s",
            Vec3::new(0.0, 0.0, in_front(1.5)),
            Vec3::new(0.0, 0.0, 2.0),
        ),
        (
            "closing at 200 m/s",
            Vec3::new(0.0, 0.0, in_front(1.5)),
            Vec3::new(0.0, 0.0, 20.0),
        ),
        (
            "crossing the face at 200 m/s",
            Vec3::new(-3.0, 0.0, in_front(capture_gap * 0.5)),
            Vec3::new(20.0, 0.0, 0.0),
        ),
    ];
    let missed: Vec<_> = cases
        .into_iter()
        .filter(|&(_, at, velocity)| {
            let (mut app, ship, _) = intake_app(12);
            let canister = drifting_canister(&mut app, 4, at, Quat::IDENTITY, velocity);
            !(run_until_gone(&mut app, canister, 600)
                && plates(&app, ship) == 16
                && takes(&app) == 1)
        })
        .map(|(case, ..)| case)
        .collect();
    assert!(missed.is_empty(), "not taken whole once: {missed:?}");
}

#[test]
fn a_rotated_canister_whose_corner_reaches_the_trigger_is_taken() {
    let (mut app, ship, _) = intake_app(12);
    // Yawed 45 degrees, the canister's leading corner (nearest the face) is
    // (x - z) / sqrt 2 of its half size behind its centre across the face.
    // That corner enters the trigger just inside the aperture edge; the rest
    // of the footprint hangs over the frame.
    let half = CARGO_CANISTER_SIZE * 0.5;
    let leading = (half.x - half.z) * FRAC_1_SQRT_2;
    let footprint = (half.x + half.z) * FRAC_1_SQRT_2;
    let centre_x = APERTURE_WIDTH.to_engine() * 0.5 - 0.05 + leading;
    assert!(centre_x + footprint > APERTURE_WIDTH.to_engine() * 0.5 + 0.5);
    let canister = drifting_canister(
        &mut app,
        4,
        Vec3::new(centre_x, 0.3, FACE_Z - footprint - 0.5),
        Quat::from_rotation_y(core::f32::consts::FRAC_PI_4),
        Vec3::new(0.0, 0.0, 0.3),
    );

    assert!(
        run_until_gone(&mut app, canister, 400),
        "the canister was never taken"
    );
    assert_eq!(plates(&app, ship), 16);
}

#[test]
fn the_door_does_not_gate_a_take() {
    let (mut app, ship, intake) = intake_app(12);
    let canister = drifting_canister(
        &mut app,
        4,
        Vec3::new(0.0, 0.0, in_front(CAPTURE_GAP.to_engine() * 0.5)),
        Quat::IDENTITY,
        Vec3::ZERO,
    );

    assert!(run_until_gone(&mut app, canister, 400));
    assert_eq!(plates(&app, ship), 16);
    let progress = door(&app, intake);
    assert!(
        progress < 1.0,
        "taken only once the door was open: {progress}"
    );
    // With nothing in front of it the door shuts again.
    for _ in 0..120 {
        app.update();
    }
    assert_eq!(door(&app, intake), 0.0);
}

#[test]
fn a_canister_in_two_triggers_is_taken_once() {
    // Side by side, the two apertures leave a 0.78 unit gap on the hull's
    // centre line, which a lengthwise canister there spans into both.
    let (mut app, ship, _) = intakes_app(12, &[-1.5, 1.5]);
    let canister = drifting_canister(
        &mut app,
        4,
        Vec3::new(0.0, 0.0, in_front(CAPTURE_GAP.to_engine() * 0.5)),
        Quat::IDENTITY,
        Vec3::ZERO,
    );

    assert!(run_until_gone(&mut app, canister, 400));
    for _ in 0..4 {
        app.update();
    }
    assert_eq!(plates(&app, ship), 16);
    assert_eq!(takes(&app), 1);
}

#[test]
fn a_canister_striking_the_frame_or_hull_outside_the_trigger_is_not_taken() {
    let (mut app, ship, _) = intake_app(12);
    // Square on, its near face spans x 1.28 to 2.22: clear of the 1.11 unit
    // half aperture, and over the frame up to the collider's 1.5 edge.
    let frame = drifting_canister(
        &mut app,
        4,
        Vec3::new(1.75, 0.0, in_front(0.5)),
        Quat::IDENTITY,
        Vec3::new(0.0, 0.0, 0.3),
    );
    // Into the hull's back, far from the door.
    let hull = drifting_canister(
        &mut app,
        4,
        Vec3::new(0.0, 0.0, 1.0),
        Quat::IDENTITY,
        Vec3::new(0.0, 0.0, -0.3),
    );

    for _ in 0..240 {
        app.update();
    }
    for (canister, closing) in [(frame, Vec3::Z), (hull, Vec3::NEG_Z)] {
        let velocity = app.world().get::<LinearVelocity>(canister).unwrap().0;
        assert!(
            velocity.dot(closing) < 0.15,
            "{canister:?} never struck the ship: {velocity}"
        );
    }
    assert_eq!(plates(&app, ship), 12);
    assert_eq!(takes(&app), 0);
}

#[test]
fn a_round_crosses_the_trigger_and_hits_the_intake() {
    let (mut app, _, intake) = intake_app(12);
    app.world_mut()
        .entity_mut(intake)
        .insert(Health::new(100.0));
    settle(&mut app);

    let round = app
        .world_mut()
        .spawn((
            Name::new("bullet"),
            TurretBulletProjectileMarker,
            Transform::from_xyz(0.0, 0.0, -8.0),
            RoundVelocity(Vec3::Z * 100.0),
            ProjectileDamage::new(10.0, DamageType::Kinetic),
        ))
        .id();
    for _ in 0..20 {
        app.update();
    }

    assert!(app.world().get_entity(round).is_err(), "the round flew on");
    let health = app.world().get::<Health>(intake).unwrap().current;
    assert!(health < 100.0, "the intake took no hit: {health}");
}

#[test]
fn published_pickup_readiness_matches_the_take() {
    let (mut app, ship, intake) = intake_app(12);
    let canister = drifting_canister(
        &mut app,
        4,
        Vec3::new(0.0, 0.0, in_front(CAPTURE_GAP.to_engine() * 0.5)),
        Quat::IDENTITY,
        Vec3::ZERO,
    );
    app.update();
    let pair = app
        .world()
        .resource::<CargoPickupReadiness>()
        .pairs
        .iter()
        .find(|pair| pair.intake == intake && pair.canister == canister)
        .copied()
        .unwrap();
    assert_eq!(pair.ship, ship);
    // The HUD recomputes the face from the intake's rendered pose; on the
    // settled fixture that is the -Z face the trigger sits on.
    let intake_pose = app.world().get::<GlobalTransform>(intake).unwrap();
    let (face, normal) = cargo_intake_face(
        intake_pose.translation(),
        intake_pose.rotation(),
        *app.world().get::<SectionCollider>(intake).unwrap(),
    );
    assert!((face - Vec3::new(0.0, 0.0, FACE_Z)).length() < 0.01);
    assert!((normal - Vec3::NEG_Z).length() < 1e-5);

    // The candidate is ready only on the pass that takes it.
    let mut frames = 0;
    while app.world().get_entity(canister).is_ok() {
        assert!(app
            .world()
            .resource::<CargoPickupReadiness>()
            .pairs
            .iter()
            .all(|pair| pair.canister != canister || !pair.ready));
        app.update();
        frames += 1;
        assert!(frames < 200, "the canister was never taken");
    }
    assert!(app
        .world()
        .resource::<CargoPickupReadiness>()
        .pairs
        .iter()
        .any(|pair| pair.canister == canister && pair.ready));
    assert_eq!(plates(&app, ship), 16);

    // A distant candidate still gets a pair, but it is not ready.
    let far = drifting_canister(
        &mut app,
        1,
        Vec3::new(0.0, 0.0, in_front(8.0)),
        Quat::IDENTITY,
        Vec3::ZERO,
    );
    app.update();
    assert!(app
        .world()
        .resource::<CargoPickupReadiness>()
        .pairs
        .iter()
        .any(|pair| pair.canister == far && !pair.ready));
}

#[test]
fn zero_health_destroys_canister_and_loses_its_stock() {
    let (mut app, ship, _) = intake_app(12);
    let canister = drifting_canister(
        &mut app,
        4,
        Vec3::new(0.0, 0.0, in_front(4.0)),
        Quat::IDENTITY,
        Vec3::ZERO,
    );
    assert_eq!(app.world().get::<Health>(canister).unwrap().current, 20.0);
    app.world_mut().trigger(HealthApplyDamage {
        entity: canister,
        source: None,
        amount: 20.0,
    });
    app.update();
    assert!(app.world().get_entity(canister).is_err());
    assert_eq!(plates(&app, ship), 12);
}

#[test]
fn a_canister_larger_than_the_free_room_is_refused_whole() {
    let (mut app, ship, intake) = intake_app(38);
    let canister = drifting_canister(
        &mut app,
        4,
        Vec3::new(0.0, 0.0, in_front(CAPTURE_GAP.to_engine() * 0.5)),
        Quat::IDENTITY,
        Vec3::ZERO,
    );

    for _ in 0..120 {
        app.update();
    }
    assert_eq!(door(&app, intake), 0.0, "the door stays shut for it");
    assert!(app
        .world()
        .resource::<CargoPickupReadiness>()
        .pairs
        .iter()
        .any(|pair| pair.canister == canister && !pair.ready));
    assert_eq!(plates(&app, ship), 38, "no part of it is taken");
    assert_eq!(
        app.world()
            .get::<CargoCanister>(canister)
            .map(CargoCanister::total_mass_g),
        Some(40_000)
    );

    // Room is the only thing that refused it.
    app.world_mut()
        .get_mut::<ShipInventory>(ship)
        .unwrap()
        .remove(ItemType::HullPlate, 2);
    for _ in 0..4 {
        app.update();
    }
    assert_eq!(plates(&app, ship), 40);
    assert!(app.world().get_entity(canister).is_err());
}

#[test]
fn the_door_opens_for_a_canister_that_fits_beside_one_that_does_not() {
    let (mut app, ship, intake) = intake_app(38);
    let oversized = drifting_canister(
        &mut app,
        4,
        Vec3::new(-1.0, 0.0, -4.5),
        Quat::IDENTITY,
        Vec3::ZERO,
    );
    let fitting = drifting_canister(
        &mut app,
        1,
        Vec3::new(1.0, 0.0, -4.5),
        Quat::IDENTITY,
        Vec3::ZERO,
    );

    for _ in 0..120 {
        app.update();
    }
    assert_eq!(
        door(&app, intake),
        1.0,
        "the door opened for the fitting one"
    );
    assert_eq!(plates(&app, ship), 38, "neither reached the trigger");
    assert!(app.world().get_entity(oversized).is_ok());
    assert!(app.world().get_entity(fitting).is_ok());
}

#[test]
fn an_ejected_canister_leaves_without_being_taken_back() {
    let (mut app, ship, intake) = intake_app(8);
    app.world_mut()
        .entity_mut(intake)
        .insert(CargoIntakeEjectionQueue(VecDeque::from([
            CargoCanister::new(ItemType::HullPlate, 4),
        ])));

    // The ejection waits for the door, then leaves across the face.
    let mut frames = 0;
    let (canister, birth) = loop {
        app.update();
        frames += 1;
        assert!(frames < 200, "the ejection never left");
        if let Some((entity, held, position)) = canisters(&mut app).first() {
            assert_eq!(door(&app, intake), 1.0, "it left through an open door");
            assert_eq!(
                held.stacks().collect::<Vec<_>>(),
                [(ItemType::HullPlate, 4)]
            );
            break (*entity, *position);
        }
    };
    assert!(app
        .world()
        .get::<CargoIntakeEjectionQueue>(intake)
        .is_none());
    let born_at = in_front(CAPTURE_GAP.to_engine() + CARGO_CANISTER_CLEARANCE);
    assert!(
        (birth.z - born_at).abs() < 0.01 && birth.x.abs() < 1e-3,
        "born just past the trigger: {birth}"
    );

    for _ in 0..180 {
        app.update();
    }
    let position = app.world().get::<Position>(canister).unwrap().0;
    assert!(position.z < birth.z - 0.5, "it never left: {position}");
    assert_eq!(plates(&app, ship), 8);
    assert_eq!(takes(&app), 0);
}

#[test]
fn speculative_ejection_contact_is_refused_until_actual_reentry() {
    let (mut app, ship, intake) = intake_app(8);
    app.world_mut().get_mut::<LinearVelocity>(ship).unwrap().0 = Vec3::NEG_Z * 3.0;
    app.world_mut()
        .entity_mut(intake)
        .insert(CargoIntakeEjectionQueue(VecDeque::from([
            CargoCanister::new(ItemType::HullPlate, 4),
        ])));

    let mut frames = 0;
    let canister = loop {
        app.update();
        frames += 1;
        assert!(frames < 200, "the ejection never left");
        if let Some((entity, ..)) = canisters(&mut app).first() {
            break *entity;
        }
    };
    let trigger = app.world().get::<CargoIntakeTrigger>(intake).unwrap().0;
    let (_, speculative) = app
        .world()
        .resource::<ContactGraph>()
        .get(trigger, canister)
        .expect("ejected canister has a speculative contact");
    assert!(speculative.is_touching());
    let deepest = speculative
        .manifolds
        .iter()
        .flat_map(|manifold| &manifold.points)
        .map(|point| point.penetration)
        .fold(f32::NEG_INFINITY, f32::max);
    assert!(
        deepest.is_finite() && deepest < 0.0,
        "separated contact: {deepest}"
    );
    app.update();
    assert!(app.world().get_entity(canister).is_ok());
    assert_eq!(plates(&app, ship), 8);
    assert_eq!(takes(&app), 0);
    assert!(app
        .world()
        .resource::<CargoPickupReadiness>()
        .pairs
        .iter()
        .any(|pair| pair.intake == intake && pair.canister == canister && !pair.ready));

    // Move the same ship back toward its dropped canister. No origin ban may
    // prevent a real return through this intake.
    app.world_mut()
        .entity_mut(ship)
        .insert(ConstantLinearAcceleration(Vec3::NEG_Z));
    let mut overlapped = false;
    for _ in 0..180 {
        if let Some((_, contact)) = app
            .world()
            .resource::<ContactGraph>()
            .get(trigger, canister)
        {
            overlapped |= contact
                .manifolds
                .iter()
                .any(|manifold| manifold.points.iter().any(|point| point.penetration >= 0.0));
        }
        app.update();
        if app.world().get_entity(canister).is_err() {
            break;
        }
        assert_eq!(plates(&app, ship), 8);
        assert_eq!(takes(&app), 0);
    }
    assert!(overlapped, "the canister never intersected the trigger");
    assert!(
        app.world().get_entity(canister).is_err(),
        "actual reentry was refused"
    );
    assert!(app
        .world()
        .resource::<CargoPickupReadiness>()
        .pairs
        .iter()
        .any(|pair| pair.intake == intake && pair.canister == canister && pair.ready));
    assert_eq!(plates(&app, ship), 12);
    assert_eq!(takes(&app), 1);
    for _ in 0..4 {
        app.update();
    }
    assert_eq!(plates(&app, ship), 12);
    assert_eq!(takes(&app), 1);
}

/// The item counts held by live canisters and by `intake`'s queue.
fn queued_and_drifting(app: &mut App, intake: Entity) -> BTreeMap<ItemType, u32> {
    let mut held = BTreeMap::new();
    let queued = app
        .world()
        .get::<CargoIntakeEjectionQueue>(intake)
        .map_or(Vec::new(), |queue| queue.0.iter().cloned().collect());
    let drifting = canisters(app).into_iter().map(|(_, canister, _)| canister);
    for canister in queued.into_iter().chain(drifting) {
        for (item, count) in canister.stacks() {
            *held.entry(item).or_default() += count;
        }
    }
    held
}

#[test]
fn queued_canisters_leave_in_order_through_an_open_door_once_each_birth_point_clears() {
    let (mut app, ship, intake) = intake_app(8);
    let queued = [
        CargoCanister::new(ItemType::Torpedo, 1),
        CargoCanister::new(ItemType::RailSlug, 10),
        CargoCanister::new(ItemType::PdcRound, 1_000),
    ];

    // The interface pauses virtual time: a queue confirmed then waits.
    app.world_mut().resource_mut::<Time<Virtual>>().pause();
    app.world_mut()
        .entity_mut(intake)
        .insert(CargoIntakeEjectionQueue(VecDeque::from(queued.clone())));
    let expected = queued_and_drifting(&mut app, intake);
    for _ in 0..120 {
        app.update();
    }
    assert!(canisters(&mut app).is_empty(), "nothing left while paused");
    assert_eq!(door(&app, intake), 0.0);
    app.world_mut().resource_mut::<Time<Virtual>>().unpause();

    // Each canister leaves front first while the door stays open, once the
    // last has drifted clear of the birth point.
    let mut left: Vec<(Entity, CargoCanister)> = Vec::new();
    let mut minted = Vec::new();
    let mut frames = 0;
    while left.len() < queued.len() {
        app.update();
        frames += 1;
        assert!(frames < 2_000, "the queue stalled after {}", left.len());
        let born: Vec<_> = canisters(&mut app)
            .into_iter()
            .filter(|(entity, ..)| left.iter().all(|(seen, _)| seen != entity))
            .collect();
        assert!(born.len() <= 1, "two canisters left in one frame");
        if let Some((entity, canister, _)) = born.into_iter().next() {
            let id = app
                .world()
                .get::<CargoCanisterRuntimeId>(entity)
                .expect("each jettisoned canister has a runtime id")
                .0;
            assert!(!minted.contains(&id), "jettison reused canister id {id}");
            minted.push(id);
            assert_eq!(door(&app, intake), 1.0, "it left through an open door");
            left.push((entity, canister));
            let waiting = app
                .world()
                .get::<CargoIntakeEjectionQueue>(intake)
                .map_or(0, |queue| queue.0.len());
            assert_eq!(waiting, queued.len() - left.len());
        }
        assert_eq!(queued_and_drifting(&mut app, intake), expected);
    }
    assert_eq!(
        left.into_iter()
            .map(|(_, canister)| canister)
            .collect::<Vec<_>>(),
        queued
    );
    assert_eq!(plates(&app, ship), 8, "the hold never changed");
}

#[test]
fn a_despawned_intake_loses_its_waiting_canisters() {
    let (mut app, ship, intake) = intake_app(8);
    app.world_mut()
        .entity_mut(intake)
        .insert(CargoIntakeEjectionQueue(VecDeque::from([
            CargoCanister::new(ItemType::HullPlate, 4),
            CargoCanister::new(ItemType::HullPlate, 3),
        ])));
    let mut frames = 0;
    while canisters(&mut app).is_empty() {
        app.update();
        frames += 1;
        assert!(frames < 200, "the first canister never left");
    }

    app.world_mut().entity_mut(intake).despawn();
    for _ in 0..600 {
        app.update();
    }
    let left: Vec<_> = canisters(&mut app)
        .into_iter()
        .map(|(_, canister, _)| canister)
        .collect();
    assert_eq!(left, [CargoCanister::new(ItemType::HullPlate, 4)]);
    assert!(app
        .world_mut()
        .query::<&CargoIntakeEjectionQueue>()
        .iter(app.world())
        .next()
        .is_none());
    assert_eq!(plates(&app, ship), 8, "the lost canister does not return");
    assert!(
        app.world_mut()
            .query::<&CargoIntakeTriggerOf>()
            .iter(app.world())
            .next()
            .is_none(),
        "the trigger outlived its intake"
    );
}
