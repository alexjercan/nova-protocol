//! A living station is a docking hub: three hulls docked at three of its
//! collars are one assembly the station flies and holds level, and one hull
//! leaving leaves the other two docked.
//!
//! Real catalog sections, the real station spawn and the real flight
//! computer. Every assertion is on state or motion, never on time: a leg the
//! station does not finish inside the tick cap is a failure, not a timing.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_events::prelude::{EntityId, MetersPerSecond3};
use nova_gameplay::{
    prelude::*,
    test_support::{settle, test_items, unfinished_integrity_physics_app},
};
use nova_ship::{flight::prelude::*, prelude::*};
use serde::Deserialize;

use crate::prelude::*;

const STATION: &str = "test_hub";
const TENDER: &str = "test_tender";
/// The station core: a solid box of hull cells.
const CORE: IVec3 = IVec3::new(5, 3, 5);
/// Face-to-face gap at capture, cells: inside the one-cell capture distance.
const GAP: f32 = 0.5;
/// The fixed updates the station may take to level before the test calls it
/// lost.
const TICK_CAP: usize = 60 * 120;

#[derive(Deserialize)]
#[expect(
    clippy::large_enum_variant,
    reason = "each catalog entry is parsed once and moved out"
)]
enum Entry {
    Section(SectionConfig),
}

fn section(id: &str, position: Vec3, rotation: Quat, prototype: &str) -> SpaceshipSectionConfig {
    SpaceshipSectionConfig {
        id: id.to_string(),
        position,
        rotation,
        source: SectionSource::prototype(prototype),
    }
}

/// The four collars, one on each side face at mid height, each facing out.
fn collars() -> [(&'static str, Vec3, Quat); 4] {
    let mid = Vec3::new(2.0, 1.0, 2.0);
    [
        (
            "west",
            mid + Vec3::new(-3.0, 0.0, 0.0),
            Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
        ),
        (
            "east",
            mid + Vec3::new(3.0, 0.0, 0.0),
            Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
        ),
        ("north", mid + Vec3::new(0.0, 0.0, -3.0), Quat::IDENTITY),
        (
            "south",
            mid + Vec3::new(0.0, 0.0, 3.0),
            Quat::from_rotation_y(std::f32::consts::PI),
        ),
    ]
}

/// A hull box, a flight computer and a PDC turret on top, and a collar on
/// each side.
fn station_design() -> StationDesign {
    let mut sections = Vec::new();
    for x in 0..CORE.x {
        for y in 0..CORE.y {
            for z in 0..CORE.z {
                sections.push(section(
                    &format!("hull_{x}_{y}_{z}"),
                    Vec3::new(x as f32, y as f32, z as f32),
                    Quat::IDENTITY,
                    REINFORCED_HULL_SECTION_ID,
                ));
            }
        }
    }
    sections.push(section(
        "bridge",
        Vec3::new(1.0, 3.0, 2.0),
        Quat::IDENTITY,
        BASIC_CONTROLLER_SECTION_ID,
    ));
    sections.push(section(
        "gun",
        Vec3::new(3.0, 3.0, 2.0),
        Quat::IDENTITY,
        PDC_KINETIC_TURRET_SECTION_ID,
    ));
    for (id, position, rotation) in collars() {
        sections.push(section(id, position, rotation, DOCKING_PORT_SECTION_ID));
    }
    StationDesign {
        id: STATION.into(),
        name: "Test hub".to_string(),
        about: String::new(),
        hull: ShipDesign {
            sections,
            integrity: ShipIntegrityConfig {
                collapse_threshold: Some(0.5),
            },
            ..default()
        },
        services: vec![StationServiceType::Trade],
        storage_cap_g: 0,
        stock: ShipInventoryStock::default(),
        credits: 100,
    }
}

/// A collar at the origin facing -Z and three hull cells behind it.
fn tender_design() -> ShipDesignPrototype {
    let mut sections = vec![section(
        "collar",
        Vec3::ZERO,
        Quat::IDENTITY,
        DOCKING_PORT_SECTION_ID,
    )];
    for z in 1..4 {
        sections.push(section(
            &format!("hull_{z}"),
            Vec3::new(0.0, 0.0, z as f32),
            Quat::IDENTITY,
            REINFORCED_HULL_SECTION_ID,
        ));
    }
    ShipDesignPrototype {
        id: TENDER.into(),
        name: "Test tender".to_string(),
        design: ShipDesign {
            sections,
            ..default()
        },
    }
}

fn app() -> App {
    let mut app = unfinished_integrity_physics_app();
    app.add_plugins((
        PDControllerPlugin,
        SpaceshipSectionPlugin { render: false },
        SpaceshipPlugin,
        NovaFlightPlugin,
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
    app.insert_resource(GameShipDesigns(vec![tender_design()]));
    app.insert_resource(GameStationDesigns(vec![station_design()]));
    app.init_resource::<CargoCanisterIdAllocator>();
    app.insert_resource(test_items());
    app.finish();
    app
}

/// The station at the origin, living and Neutral.
fn spawn_station(app: &mut App) -> Entity {
    let mut commands = app.world_mut().commands();
    let mut entity = commands.spawn((Transform::default(), EntityId::new("hub")));
    station_scenario_object(
        &mut entity,
        &station_design(),
        &StationConfig {
            design: STATION.into(),
            allegiance: Allegiance::Neutral,
            derelict: false,
            initial_velocity: MetersPerSecond3::ZERO,
        },
    );
    entity.id()
}

/// A tender facing the collar at `position`/`rotation` on the station at the
/// origin, its collar face `GAP` cells off the station's.
fn spawn_tender(app: &mut App, position: Vec3, rotation: Quat) -> Entity {
    let axis = rotation * Vec3::NEG_Z;
    let face = position + axis * 0.5;
    let turn = Quat::from_rotation_arc(Vec3::NEG_Z, -axis);
    let at = face + axis * GAP - turn * Vec3::new(0.0, 0.0, -0.5);
    app.world_mut()
        .spawn((
            Transform::from_translation(at).with_rotation(turn),
            spaceship_scenario_object(SpaceshipConfig {
                design: ShipDesignSource::prototype(TENDER),
                controller: SpaceshipController::None,
                initial_velocity: MetersPerSecond3::ZERO,
                allegiance: None,
                capabilities: default(),
                inventory: ShipInventoryStock::default(),
                lootable: false,
                credits: 0,
            }),
        ))
        .id()
}

fn mass(app: &App, root: Entity) -> f32 {
    app.world().get::<ComputedMass>(root).unwrap().value()
}

fn assembly_mass(app: &App, root: Entity) -> f32 {
    app.world()
        .get::<DockedAssembly>(root)
        .expect("a docked root carries its assembly")
        .mass
}

/// Each tender's pose in the station's frame: what its joint holds fixed.
fn relative(app: &App, station: Entity, tender: Entity) -> (Vec3, Quat) {
    let pose = |root: Entity| {
        (
            app.world().get::<Position>(root).unwrap().0,
            app.world().get::<Rotation>(root).unwrap().0,
        )
    };
    let ((p0, r0), (p1, r1)) = (pose(station), pose(tender));
    (r0.inverse() * (p1 - p0), r0.inverse() * r1)
}

#[test]
fn a_station_hub_flies_three_docked_hulls_as_one_level_body_and_keeps_two_when_one_leaves() {
    let mut app = app();
    let station = spawn_station(&mut app);
    let tenders: Vec<Entity> = collars()[..3]
        .iter()
        .map(|&(_, position, rotation)| spawn_tender(&mut app, position, rotation))
        .collect();
    settle(&mut app);
    settle(&mut app);
    let parked = app.world().get::<Rotation>(station).unwrap().0;

    for &tender in &tenders {
        app.world_mut().trigger(DockingConnectionRequest {
            entity: tender,
            target: station,
        });
        app.update();
        assert!(
            app.world().get::<DockedShip>(tender).is_some(),
            "the station admits the tender at a free collar"
        );
    }
    settle(&mut app);

    let docks = app.world().get::<StationDocks>(station).unwrap();
    assert_eq!(docks.len(), 3, "one connection per docked hull");
    let total = mass(&app, station) + tenders.iter().map(|t| mass(&app, *t)).sum::<f32>();
    for root in std::iter::once(station).chain(tenders.iter().copied()) {
        assert!(
            (assembly_mass(&app, root) - total).abs() <= 1e-3 * total,
            "{root:?}: assembly mass {} is the sum {total}",
            assembly_mass(&app, root)
        );
    }
    assert!(
        app.world().get::<DockedStation>(station).unwrap().drives,
        "the living station flies its assembly"
    );
    for &tender in &tenders {
        assert!(
            !app.world().get::<DockedShip>(tender).unwrap().drives,
            "every docked hull is held"
        );
    }

    // A ram's kick, applied to the assembly as one rigid body, the size
    // `system_station_body` gives a lone station: turned half a radian about
    // a slanted axis through its centre of mass and left spinning at 0.3
    // rad/s. The computer must bring the whole assembly back onto the parked
    // attitude.
    let starts: Vec<(Vec3, Quat)> = tenders
        .iter()
        .map(|t| relative(&app, station, *t))
        .collect();
    let center = app
        .world()
        .get::<DockedAssembly>(station)
        .unwrap()
        .center_of_mass;
    let axis = Vec3::new(0.2, 0.4, -0.25).normalize();
    let turn = Quat::from_axis_angle(axis, 0.5);
    let spin = axis * 0.3;
    for root in std::iter::once(station).chain(tenders.iter().copied()) {
        let mut entity = app.world_mut().entity_mut(root);
        let position = entity.get::<Position>().unwrap().0;
        let rotation = entity.get::<Rotation>().unwrap().0;
        let local_center = entity.get::<ComputedCenterOfMass>().unwrap().0;
        let moved = center + turn * (position - center);
        entity.get_mut::<Position>().unwrap().0 = moved;
        entity.get_mut::<Rotation>().unwrap().0 = turn * rotation;
        entity.get_mut::<AngularVelocity>().unwrap().0 = spin;
        entity.get_mut::<LinearVelocity>().unwrap().0 =
            spin.cross(moved + turn * rotation * local_center - center);
    }
    let mut peak = 0.0f32;
    let mut level_at = None;
    for tick in 0..TICK_CAP {
        app.update();
        let error = app
            .world()
            .get::<Rotation>(station)
            .unwrap()
            .0
            .angle_between(parked);
        peak = peak.max(error);
        let rate = app
            .world()
            .get::<AngularVelocity>(station)
            .unwrap()
            .0
            .length();
        if peak > STATION_LEVEL_TOLERANCE_RAD && error < STATION_LEVEL_TOLERANCE_RAD && rate < 1e-3
        {
            level_at = Some(tick);
            break;
        }
    }
    assert!(
        level_at.is_some(),
        "the station never came back level: peak {peak} rad"
    );
    assert!(
        peak > 2.0 * STATION_LEVEL_TOLERANCE_RAD,
        "the kick turned the assembly well off level: peak {peak} rad"
    );
    // Stable, not passing through: it stays level with no input.
    for _ in 0..240 {
        app.update();
    }
    let error = app
        .world()
        .get::<Rotation>(station)
        .unwrap()
        .0
        .angle_between(parked);
    assert!(
        error < STATION_LEVEL_TOLERANCE_RAD,
        "the station holds level: {error} rad off"
    );
    for (tender, start) in tenders.iter().zip(&starts) {
        let (offset, turn) = relative(&app, station, *tender);
        let drift = (offset - start.0).length();
        let twist = turn.angle_between(start.1);
        assert!(
            drift < 0.01 && twist < 0.01,
            "{tender:?}: the joint held ({drift} u, {twist} rad)"
        );
    }
    println!(
        "station hub: peak {peak:.4} rad, level within {STATION_LEVEL_TOLERANCE_RAD} rad \
         after {} ticks, then {error:.5} rad",
        level_at.unwrap_or_default()
    );

    app.world_mut()
        .trigger(DockingReleaseRequest { entity: tenders[0] });
    settle(&mut app);
    assert!(
        app.world().get::<DockedShip>(tenders[0]).is_none(),
        "the tender that let go is free"
    );
    for &tender in &tenders[1..] {
        assert!(
            app.world().get::<DockedShip>(tender).is_some(),
            "the other two stay docked"
        );
    }
    assert_eq!(app.world().get::<StationDocks>(station).unwrap().len(), 2);
    let remaining = mass(&app, station) + tenders[1..].iter().map(|t| mass(&app, *t)).sum::<f32>();
    assert!(
        (assembly_mass(&app, station) - remaining).abs() <= 1e-3 * remaining,
        "the assembly is re-measured without the tender that left"
    );
    assert!(app.world().get::<DockedAssembly>(tenders[0]).is_none());
}
