//! The intake's contract: what it takes, what it refuses, and what it drops.
//!
//! Real avian, because the speed rule reads rigid-body velocities and the
//! no-impact rule is judged by avian's own contacts. Every fixture is one
//! hull, at rest at the origin until a canister hits it, with an intake
//! mounted 2 units down its -Z, so the door face is the hull-local plane
//! z = -2.5 and a canister in front of it has z below that.

use std::{
    collections::BTreeMap,
    f32::consts::{FRAC_1_SQRT_2, FRAC_PI_4},
};

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
        maximum_capture_speed: MetersPerSecond(5.0),
        eject_speed: MetersPerSecond(3.0),
    }
}

/// One fixed tick of a watched canister, read just before the intake pass.
#[derive(Clone, Copy, Debug)]
struct Tick {
    /// The distance from the canister's nearest corner to the face plane at
    /// the hull's live pose, in world units, or `None` once the canister is
    /// gone.
    gap: Option<f32>,
    /// Whether any of the canister's contact pairs is touching.
    touching: bool,
    /// Whether it touches the watched intake collider.
    mouth_touch: bool,
    velocity: Vec3,
    door: f32,
}

/// The canister whose ticks [`record_watched_canister`] logs.
#[derive(Resource)]
struct Watch {
    canister: Entity,
    intake: Entity,
    ticks: Vec<Tick>,
}

fn record_watched_canister(
    watch: Option<ResMut<Watch>>,
    collisions: Collisions,
    q_canisters: Query<(&Position, &Rotation, &LinearVelocity)>,
    q_intakes: Query<&SectionAnimations>,
    q_hull: Single<(&Position, &Rotation), With<SpaceshipRootMarker>>,
) {
    let Some(mut watch) = watch else {
        return;
    };
    let (hull_position, hull_rotation) = *q_hull;
    let door = q_intakes
        .get(watch.intake)
        .unwrap()
        .cue_progress(SectionAnimationCue::IntakeDoor)
        .unwrap();
    let tick = match q_canisters.get(watch.canister) {
        Ok((position, rotation, velocity)) => Tick {
            gap: Some(
                (0..8)
                    .map(|corner| {
                        let sign = |bit: u32| if corner & bit == 0 { -0.5 } else { 0.5 };
                        let local = CARGO_CANISTER_SIZE * Vec3::new(sign(1), sign(2), sign(4));
                        let corner = position.0 + rotation.0 * local;
                        FACE_Z - (hull_rotation.0.inverse() * (corner - hull_position.0)).z
                    })
                    .fold(f32::INFINITY, f32::min),
            ),
            touching: collisions
                .collisions_with(watch.canister)
                .any(|pair| pair.is_touching()),
            mouth_touch: collisions.collisions_with(watch.canister).any(|pair| {
                pair.is_touching()
                    && (pair.collider1 == watch.intake || pair.collider2 == watch.intake)
            }),
            velocity: velocity.0,
            door,
        },
        Err(_) => Tick {
            gap: None,
            touching: false,
            mouth_touch: false,
            velocity: Vec3::ZERO,
            door,
        },
    };
    watch.ticks.push(tick);
}

/// A headless app with one hull carrying `stock` hull plates in a 400 kg
/// hold and one intake. Returns the app, the ship and the intake.
fn intake_app(stock: u32) -> (App, Entity, Entity) {
    let mut app = unfinished_integrity_physics_app();
    app.add_plugins((
        SectionAnimationPlugin,
        CargoIntakeSectionPlugin { render: false },
    ));
    app.add_systems(
        FixedUpdate,
        record_watched_canister.before(CargoIntakeSystems),
    );
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
    let intake = app
        .world_mut()
        .spawn((
            ChildOf(ship),
            Name::new("intake"),
            Transform::from_xyz(0.0, 0.0, -2.0),
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
        .id();
    settle(&mut app);
    (app, ship, intake)
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

/// Log `canister`'s ticks from now on.
fn watch(app: &mut App, canister: Entity, intake: Entity) {
    app.world_mut().insert_resource(Watch {
        canister,
        intake,
        ticks: Vec::new(),
    });
}

fn ticks(app: &App) -> &[Tick] {
    &app.world().resource::<Watch>().ticks
}

/// Asserts the watched canister, now gone, kept `velocity` and touched
/// nothing on every tick up to its take, and was taken on the first tick its
/// gap closed to the capture gap. The take runs after the tick's record, so
/// the last record is the tick it was taken on.
fn assert_taken_before_any_contact(ticks: &[Tick], velocity: Vec3) {
    let capture_gap = CAPTURE_GAP.to_engine();
    let first = ticks
        .iter()
        .position(|tick| tick.gap.is_some_and(|gap| gap <= capture_gap))
        .expect("the canister never closed to the capture gap");
    for (index, tick) in ticks[..=first].iter().enumerate() {
        assert!(!tick.touching, "touching at tick {index}: {tick:?}");
        assert!(
            tick.gap.is_some_and(|gap| gap > 0.0),
            "gone or overlapping at tick {index}: {tick:?}"
        );
        assert!(
            (tick.velocity - velocity).length() < 1e-5,
            "pushed at tick {index}: {tick:?}"
        );
    }
    assert!(
        ticks[first].gap.unwrap() > capture_gap - 0.02,
        "the first capture tick {:?} is not the first inside the gap",
        ticks[first]
    );
    assert_eq!(
        first,
        ticks.len() - 1,
        "not taken on the first tick inside the capture gap"
    );
}

fn plates(app: &App, ship: Entity) -> u32 {
    app.world()
        .get::<ShipInventory>(ship)
        .unwrap()
        .count(ItemType::HullPlate)
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
fn a_slow_canister_is_taken_whole_only_through_a_fully_open_door() {
    let (mut app, ship, intake) = intake_app(12);
    let canister = drifting_canister(
        &mut app,
        4,
        Vec3::new(0.0, 0.0, in_front(0.06)),
        Quat::IDENTITY,
        Vec3::ZERO,
    );

    // A take in one frame's fixed pass reads the door the frame before left,
    // and the door only opens, so no frame that ends short of open can have
    // taken the canister.
    let mut frames = 0;
    loop {
        app.update();
        frames += 1;
        assert!(frames < 200, "the door never opened");
        if door(&app, intake) >= 1.0 {
            break;
        }
        assert_eq!(
            plates(&app, ship),
            12,
            "taken through a door at {}",
            door(&app, intake)
        );
        assert!(app.world().get_entity(canister).is_ok());
    }
    for _ in 0..4 {
        app.update();
    }

    assert_eq!(plates(&app, ship), 16, "the whole canister is taken");
    assert!(
        app.world().get_entity(canister).is_err(),
        "the canister is gone"
    );
    // With nothing in front of it the door shuts again.
    for _ in 0..90 {
        app.update();
    }
    assert_eq!(door(&app, intake), 0.0);
}

#[test]
fn a_slow_canister_is_taken_before_it_touches_the_intake() {
    let (mut app, ship, intake) = intake_app(12);
    // 4 m/s head-on from 15 m out: the door is open long before it arrives.
    let velocity = Vec3::new(0.0, 0.0, 0.4);
    let canister = drifting_canister(
        &mut app,
        4,
        Vec3::new(0.0, 0.0, in_front(1.5)),
        Quat::IDENTITY,
        velocity,
    );
    watch(&mut app, canister, intake);

    let mut frames = 0;
    while app.world().get_entity(canister).is_ok() {
        app.update();
        frames += 1;
        assert!(frames < 400, "the canister was never taken");
    }

    assert_taken_before_any_contact(ticks(&app), velocity);
    assert_eq!(plates(&app, ship), 16);
}

#[test]
fn a_rotated_canister_whose_footprint_fits_the_aperture_is_taken_before_any_contact() {
    let (mut app, ship, intake) = intake_app(12);
    // Yawed 45 degrees, the canister's half-width across the face and its
    // half-depth along the normal are both (x + z) / sqrt 2 of its half size.
    let half = CARGO_CANISTER_SIZE * 0.5;
    let footprint = (half.x + half.z) * FRAC_1_SQRT_2;
    let bound = APERTURE_WIDTH.to_engine() * 0.5 - CARGO_APERTURE_MARGIN.to_engine() - footprint;
    let velocity = Vec3::new(0.0, 0.0, 0.4);
    let canister = drifting_canister(
        &mut app,
        4,
        Vec3::new(bound - 0.01, 0.3, FACE_Z - footprint - 1.5),
        Quat::from_rotation_y(FRAC_PI_4),
        velocity,
    );
    watch(&mut app, canister, intake);

    let mut frames = 0;
    while app.world().get_entity(canister).is_ok() {
        app.update();
        frames += 1;
        assert!(frames < 400, "the canister was never taken");
    }

    assert_taken_before_any_contact(ticks(&app), velocity);
    assert_eq!(plates(&app, ship), 16);
}

#[test]
fn a_canister_whose_footprint_crosses_the_aperture_edge_is_not_taken() {
    let (mut app, ship, intake) = intake_app(12);
    let half = CARGO_CANISTER_SIZE * 0.5;
    let footprint = (half.x + half.z) * FRAC_1_SQRT_2;
    let bound = APERTURE_WIDTH.to_engine() * 0.5 - CARGO_APERTURE_MARGIN.to_engine() - footprint;
    let canister = drifting_canister(
        &mut app,
        4,
        Vec3::new(bound + 0.01, 0.3, FACE_Z - footprint - 1.5),
        Quat::from_rotation_y(FRAC_PI_4),
        Vec3::new(0.0, 0.0, 0.4),
    );
    watch(&mut app, canister, intake);

    // Until it meets the face, through the whole capture gap.
    let mut frames = 0;
    while !ticks(&app).last().is_some_and(|tick| tick.touching) {
        app.update();
        frames += 1;
        assert!(frames < 400, "the canister never reached the face");
    }

    let capture_gap = CAPTURE_GAP.to_engine();
    let ticks = ticks(&app);
    assert!(
        ticks.iter().all(|tick| tick.gap.is_some()),
        "the canister was taken"
    );
    let open_in_gap = ticks
        .iter()
        .filter(|tick| tick.door >= 1.0 && tick.gap.is_some_and(|gap| gap <= capture_gap))
        .count();
    assert!(
        open_in_gap > 10,
        "it crossed the gap at an open door for {open_in_gap} ticks"
    );
    assert_eq!(plates(&app, ship), 12);
    assert!(app
        .world()
        .resource::<CargoPickupReadiness>()
        .pairs
        .iter()
        .any(|pair| pair.canister == canister && !pair.ready));
    for _ in 0..120 {
        app.update();
    }
    assert!(
        app.world().get_entity(canister).is_ok(),
        "frame contact was taken"
    );
    assert_eq!(plates(&app, ship), 12);
}

#[test]
fn a_fast_canister_is_taken_when_it_hits_the_open_intake_mouth() {
    let (mut app, ship, intake) = intake_app(12);
    // 8 m/s head-on: the door is open before it reaches the capture gap.
    let canister = drifting_canister(
        &mut app,
        4,
        Vec3::new(0.0, 0.0, in_front(1.5)),
        Quat::IDENTITY,
        Vec3::new(0.0, 0.0, 0.8),
    );
    watch(&mut app, canister, intake);

    let mut frames = 0;
    let mut fast_in_gap = 0;
    while app.world().get_entity(canister).is_ok() {
        app.update();
        frames += 1;
        assert!(frames < 400, "the canister was never taken at the mouth");
        if ticks(&app).last().is_some_and(|tick| {
            tick.door >= 1.0
                && tick.gap.is_some_and(|gap| gap <= CAPTURE_GAP.to_engine())
                && !tick.touching
        }) {
            fast_in_gap += 1;
            assert!(app
                .world()
                .resource::<CargoPickupReadiness>()
                .pairs
                .iter()
                .any(|pair| pair.canister == canister && !pair.ready));
        }
    }
    assert!(fast_in_gap > 0, "speed was never isolated in the open gap");
    let ticks = ticks(&app);
    assert!(
        ticks.last().is_some_and(|tick| tick.mouth_touch),
        "not an intake collider hit: {ticks:?}"
    );
    assert!(
        ticks.last().unwrap().door >= 1.0
            && ticks
                .last()
                .unwrap()
                .gap
                .is_some_and(|gap| gap <= CAPTURE_GAP.to_engine()),
        "taken outside the open capture gap: {:?}",
        ticks.last()
    );
    assert_eq!(plates(&app, ship), 16);
    assert!(app
        .world()
        .resource::<CargoPickupReadiness>()
        .pairs
        .iter()
        .any(|pair| pair.canister == canister && pair.ready));
}

#[test]
fn a_canister_at_the_open_mouth_is_taken_without_separating_after_impact() {
    let (mut app, ship, intake) = intake_app(12);
    let canister = drifting_canister(
        &mut app,
        4,
        Vec3::new(0.0, 0.0, in_front(1.5)),
        Quat::IDENTITY,
        Vec3::new(0.0, 0.0, 0.8),
    );
    watch(&mut app, canister, intake);
    let mut frames = 0;
    while !ticks(&app).last().is_some_and(|tick| tick.mouth_touch) {
        app.update();
        frames += 1;
        assert!(frames < 400, "the canister never met the door");
    }
    assert!(app.world().get_entity(canister).is_err());
    assert_eq!(plates(&app, ship), 16);
    assert!(app
        .world()
        .resource::<CargoPickupReadiness>()
        .pairs
        .iter()
        .any(|pair| pair.canister == canister && pair.ready));
}

#[test]
fn published_pickup_readiness_matches_the_take_and_refuses_unmet_gates() {
    let (mut app, ship, intake) = intake_app(12);
    let canister = drifting_canister(
        &mut app,
        4,
        Vec3::new(0.0, 0.0, in_front(0.06)),
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
        .unwrap();
    assert_eq!(pair.ship, ship);
    // The HUD recomputes the face from the intake's rendered pose; on the
    // settled fixture that is the -Z face the take measures against.
    let intake_pose = app.world().get::<GlobalTransform>(intake).unwrap();
    let (face, normal) = cargo_intake_face(
        intake_pose.translation(),
        intake_pose.rotation(),
        *app.world().get::<SectionCollider>(intake).unwrap(),
    );
    assert!((face - Vec3::new(0.0, 0.0, FACE_Z)).length() < 0.01);
    assert!((normal - Vec3::NEG_Z).length() < 1e-5);
    assert!(!pair.ready, "a closed door cannot take cargo");
    assert!(app.world().get_entity(canister).is_ok());

    // The same stationary candidate becomes ready only on the pass that
    // takes it. No prediction of when the door will finish counts as ready.
    let mut frames = 0;
    while app.world().get_entity(canister).is_ok() {
        app.update();
        frames += 1;
        assert!(frames < 200, "the door never allowed the take");
        if app.world().get_entity(canister).is_ok() {
            assert!(app
                .world()
                .resource::<CargoPickupReadiness>()
                .pairs
                .iter()
                .all(|pair| pair.canister != canister || !pair.ready));
        }
    }
    assert!(app
        .world()
        .resource::<CargoPickupReadiness>()
        .pairs
        .iter()
        .any(|pair| pair.canister == canister && pair.ready));
    assert_eq!(plates(&app, ship), 16);

    // A locked distant candidate still gets a pair, but it is not ready.
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
        Vec3::new(0.0, 0.0, in_front(0.06)),
        Quat::IDENTITY,
        Vec3::ZERO,
    );

    for _ in 0..120 {
        app.update();
    }
    assert_eq!(door(&app, intake), 1.0, "the door opened for it");
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
fn an_ejected_canister_is_taken_back_as_soon_as_it_closes_on_the_open_door() {
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
        "born just past the capture gap: {birth}"
    );

    // Turned straight back at 3 m/s, still in detection and at the open door
    // that dropped it.
    let velocity = Vec3::new(0.0, 0.0, 0.3);
    app.world_mut()
        .get_mut::<LinearVelocity>(canister)
        .unwrap()
        .0 = velocity;
    watch(&mut app, canister, intake);
    let mut frames = 0;
    while app.world().get_entity(canister).is_ok() {
        app.update();
        frames += 1;
        assert!(frames < 200, "the returning canister was never taken");
    }

    assert_taken_before_any_contact(ticks(&app), velocity);
    assert!(
        ticks(&app).iter().all(|tick| tick.door >= 1.0),
        "the door moved between the drop and the take"
    );
    assert_eq!(plates(&app, ship), 12);
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
}
