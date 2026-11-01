//! A station's escort for hire keeps its place beside the station across a
//! freeze and a thaw, even after another escort of the same station was hired.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_events::prelude::{EntityId, MetersPerSecond3};
use nova_gameplay::{
    prelude::*,
    test_support::{settle, test_items, unfinished_integrity_physics_app},
    transform::prelude::SmoothLookRotationPlugin,
};
use nova_ship::{flight::prelude::*, prelude::*};
use serde::Deserialize;

use crate::prelude::*;

/// The updates a park or a thaw may take before the test calls it lost.
const FRAME_CAP: usize = 64;

#[derive(Deserialize)]
enum Entry {
    Section(SectionConfig),
}

fn section(id: &str, position: Vec3, prototype: &str) -> SpaceshipSectionConfig {
    SpaceshipSectionConfig {
        id: id.to_string(),
        position,
        rotation: Quat::IDENTITY,
        source: SectionSource::prototype(prototype),
    }
}

/// A plate, a flight computer beside it and a docking collar, no turret.
fn station_design() -> StationDesign {
    StationDesign {
        hull: ShipDesign {
            sections: vec![
                section("core", Vec3::ZERO, REINFORCED_HULL_SECTION_ID),
                section("bridge", Vec3::X, BASIC_CONTROLLER_SECTION_ID),
                section("collar", Vec3::NEG_Z, DOCKING_PORT_SECTION_ID),
            ],
            integrity: ShipIntegrityConfig {
                collapse_threshold: Some(0.5),
            },
            ..default()
        },
        services: vec![StationServiceType::Trade],
        builds: Vec::new(),
        storage_cap_g: 0,
        stock: ShipInventoryStock::default(),
        credits: 100,
    }
}

/// A plate and a flight computer: a small hull to hire.
fn escort_design() -> ShipDesign {
    ShipDesign {
        sections: vec![
            section("plate", Vec3::ZERO, REINFORCED_HULL_SECTION_ID),
            section("helm", Vec3::X, BASIC_CONTROLLER_SECTION_ID),
        ],
        ..default()
    }
}

fn hire(name: &str, slot: u32) -> PendingParkedHull {
    PendingParkedHull::Offer(Box::new(StationHullOffer {
        name: name.to_string(),
        terms: HullOfferType::Hire { slot },
        design: escort_design(),
    }))
}

fn app() -> App {
    let mut app = unfinished_integrity_physics_app();
    app.add_plugins((
        SmoothLookRotationPlugin,
        PDControllerPlugin,
        SpaceshipSectionPlugin { render: false },
        SpaceshipPlugin,
        NovaFlightPlugin,
        ParkedHullsPlugin,
    ));
    let file = format!(
        "{}/../../assets/base/sections/base.content.ron",
        env!("CARGO_MANIFEST_DIR")
    );
    let sections: Vec<Entry> = ron::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    app.insert_resource(GameSections(
        sections
            .into_iter()
            .map(|Entry::Section(section)| section)
            .collect(),
    ));
    app.init_resource::<GameShipDesigns>();
    app.init_resource::<CargoCanisterIdAllocator>();
    app.insert_resource(test_items());
    app.finish();
    app
}

/// A body's pose as avian holds it, or its `Transform` before avian has seen
/// it.
fn pose(world: &World, entity: Entity) -> (Vec3, Quat) {
    let body = world.entity(entity);
    match (body.get::<Position>(), body.get::<Rotation>()) {
        (Some(position), Some(rotation)) => (position.0, rotation.0),
        _ => {
            let transform = body.get::<Transform>().expect("a body has a pose");
            (transform.translation, transform.rotation)
        }
    }
}

/// Where `hull` lies in the frame of `station`, and how fast it moves and
/// turns in it: position and rotation, velocity net of the station's own
/// velocity and the spin carrying the hull about the station, and spin net of
/// the station's spin.
fn relative(world: &World, station: Entity, hull: Entity) -> (Vec3, Quat, Vec3, Vec3) {
    let (station_at, station_turn) = pose(world, station);
    let (hull_at, hull_turn) = pose(world, hull);
    let velocity = |entity: Entity| {
        world
            .get::<LinearVelocity>(entity)
            .copied()
            .unwrap_or_default()
            .0
    };
    let spin = |entity: Entity| {
        world
            .get::<AngularVelocity>(entity)
            .copied()
            .unwrap_or_default()
            .0
    };
    let arm = hull_at - station_at;
    let into_station = station_turn.inverse();
    (
        into_station * arm,
        into_station * hull_turn,
        into_station * (velocity(hull) - (velocity(station) + spin(station).cross(arm))),
        into_station * (spin(hull) - spin(station)),
    )
}

/// The parked hull of `station` named `name`.
fn parked(world: &World, station: Entity, name: &str) -> Entity {
    let hulls = world
        .get::<ParkedHulls>(station)
        .unwrap_or_else(|| panic!("station {station:?} holds no parked hulls"));
    hulls
        .iter()
        .find(|&hull| {
            world
                .get::<Name>(hull)
                .is_some_and(|hull_name| hull_name.as_str() == name)
        })
        .unwrap_or_else(|| panic!("station {station:?} holds no hull named '{name}'"))
}

/// Park two escorts for hire beside a station at `station_pose` moving at
/// `velocity` and `spin`, hire the first, then freeze the station, write its
/// record to RON and read it back, and thaw it. Escort B, the one left, is
/// given a spin of its own at the freeze. Returns where Escort B lies and
/// moves in the station's frame ([`relative`]) before the freeze and after
/// the thaw.
fn hire_one_then_thaw(
    app: &mut App,
    station_pose: Transform,
    velocity: LinearVelocity,
    spin: AngularVelocity,
) -> [(Vec3, Quat, Vec3, Vec3); 2] {
    let station = {
        let mut commands = app.world_mut().commands();
        let mut entity = commands.spawn((station_pose, EntityId::new("outpost")));
        station_scenario_object(
            &mut entity,
            &station_design(),
            &StationConfig {
                design: StationDesignSource::Inline(station_design()),
                allegiance: Allegiance::Enemy,
                derelict: false,
                initial_velocity: MetersPerSecond3::ZERO,
            },
        );
        entity.id()
    };
    app.world_mut().flush();
    // A station is authored at rest; it gets its motion once its post is
    // derived, so the escorts park beside a moving station. Its attitude
    // hold damps the spin to about 1e-4 rad/s by the freeze.
    settle(app);
    app.world_mut().entity_mut(station).insert((
        velocity,
        spin,
        PendingParkedHulls(vec![hire("Escort A", 0), hire("Escort B", 1)]),
    ));

    let mut frames = 0;
    while app.world().entity(station).contains::<PendingParkedHulls>()
        || app
            .world()
            .get::<ParkedHulls>(station)
            .is_none_or(|hulls| hulls.len() < 2)
    {
        assert!(frames < FRAME_CAP, "the station never parked its escorts");
        app.update();
        frames += 1;
    }
    settle(app);

    // A hire freezes and despawns the offer hull (`make_hired_escort`).
    let escort_a = parked(app.world(), station, "Escort A");
    app.world_mut().entity_mut(escort_a).despawn();
    settle(app);
    // Set at the freeze: a spin set before the settle frames damps to about
    // 3e-4 rad/s by the freeze.
    let escort_b = parked(app.world(), station, "Escort B");
    app.world_mut()
        .entity_mut(escort_b)
        .insert(AngularVelocity(Vec3::new(0.02, -0.01, 0.03)));

    let before = relative(app.world(), station, escort_b);
    let (station_at, station_rotation) = pose(app.world(), station);
    let station_velocity = *app.world().get::<LinearVelocity>(station).unwrap();
    let station_spin = *app.world().get::<AngularVelocity>(station).unwrap();
    let frozen = freeze_station(app.world(), station).expect("a settled station freezes");
    let frozen: FrozenStation = ron::from_str(&ron::to_string(&frozen).unwrap()).unwrap();
    let [record] = frozen.parked() else {
        panic!(
            "the station must freeze one parked hull: {:?}",
            frozen.parked()
        );
    };
    assert_eq!(record.name(), "Escort B");
    assert_eq!(record.terms(), ParkedHullType::Hire { slot: 1 });

    app.world_mut().entity_mut(station).despawn();
    let thawed = {
        let mut commands = app.world_mut().commands();
        let mut entity = commands.spawn((
            Transform::from_translation(station_at).with_rotation(station_rotation),
            EntityId::new("outpost"),
        ));
        thaw_station(&mut entity, frozen);
        entity.insert((station_velocity, station_spin));
        entity.id()
    };
    app.world_mut().flush();

    let mut frames = 0;
    loop {
        app.update();
        frames += 1;
        if !app.world().entity(thawed).contains::<PendingParkedHulls>() {
            break;
        }
        assert!(
            frames < FRAME_CAP,
            "the thawed station never parked its escort"
        );
    }

    let escort_b = parked(app.world(), thawed, "Escort B");
    assert_eq!(
        app.world().get::<ParkedHullType>(escort_b),
        Some(&ParkedHullType::Hire { slot: 1 })
    );
    [before, relative(app.world(), thawed, escort_b)]
}

/// Escort B lies, moves and turns the same in its station's frame `before`
/// the freeze and `after` the thaw.
fn assert_kept_its_place([before, after]: [(Vec3, Quat, Vec3, Vec3); 2]) {
    let (before_at, before_turn, before_velocity, before_spin) = before;
    let (after_at, after_turn, after_velocity, after_spin) = after;
    assert!(
        (after_at - before_at).length() < 1e-2,
        "Escort B moved in the station's frame: position {before_at:?} before the freeze, \
         {after_at:?} after the thaw"
    );
    assert!(
        after_turn.angle_between(before_turn) < 1e-3,
        "Escort B turned in the station's frame: rotation {before_turn:?} before the freeze, \
         {after_turn:?} after the thaw"
    );
    assert!(
        (after_velocity - before_velocity).length() < 1e-2,
        "Escort B changed speed in the station's frame: velocity {before_velocity:?} before the \
         freeze, {after_velocity:?} after the thaw"
    );
    assert!(
        (after_spin - before_spin).length() < 1e-4,
        "Escort B changed spin in the station's frame: spin {before_spin:?} before the freeze, \
         {after_spin:?} after the thaw"
    );
}

/// Save/load and freeze/thaw leave a station's escort for hire where it was,
/// relative to the station, after another escort of the station was hired.
#[test]
fn an_escort_for_hire_thawed_in_flat_space_stands_where_it_froze_beside_its_station() {
    let mut app = app();
    let station = Transform::from_xyz(40.0, -12.0, 25.0).with_rotation(Quat::from_euler(
        EulerRot::YXZ,
        0.7,
        0.3,
        -0.4,
    ));
    assert_kept_its_place(hire_one_then_thaw(
        &mut app,
        station,
        LinearVelocity(Vec3::new(0.3, -0.2, 0.1)),
        AngularVelocity(Vec3::new(0.0, 0.01, 0.0)),
    ));
}

/// In a gravity well, save/load and freeze/thaw leave a station's escort for
/// hire where it was on the station's orbit, relative to the station, after
/// the escort on the other side was hired: it does not take the hired one's
/// side by list order.
#[test]
fn an_escort_for_hire_thawed_in_a_well_coasts_where_it_froze_beside_its_station() {
    let mut app = app();
    let centre = Vec3::new(-360.0, -12.0, 25.0);
    let mu = 400.0;
    app.world_mut().spawn((
        Transform::from_translation(centre),
        GravityWell {
            mu,
            body_radius: 50.0,
            soi_radius: 2000.0,
        },
    ));
    let station = Transform::from_xyz(40.0, -12.0, 25.0).with_rotation(Quat::from_euler(
        EulerRot::YXZ,
        0.7,
        0.3,
        -0.4,
    ));
    let orbit = station_orbit_velocity(mu, centre, station.translation)
        .expect("the station sits off the well's centre");
    assert_kept_its_place(hire_one_then_thaw(
        &mut app,
        station,
        LinearVelocity(orbit.to_engine()),
        AngularVelocity(Vec3::new(0.0, 0.01, 0.0)),
    ));
}
