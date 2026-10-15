//! A living station on real catalog sections loses its last turret and turns
//! into a dead station, and a frozen copy of it thaws dead.

use bevy::prelude::*;
use nova_events::prelude::{EntityId, MetersPerSecond3};
use nova_gameplay::{
    prelude::*,
    test_support::{settle, test_items, unfinished_integrity_physics_app},
};
use nova_ship::{flight::prelude::*, prelude::*};
use serde::Deserialize;

use crate::prelude::*;

#[derive(Deserialize)]
#[expect(
    clippy::large_enum_variant,
    reason = "each catalog entry is parsed once and moved out"
)]
enum Entry {
    Section(SectionConfig),
}

/// A plate, a flight computer beside it, a docking collar with its hatch
/// outboard and one PDC turret on top: armed, so its last turret decides.
fn design() -> StationDesign {
    let section =
        |id: &str, position: Vec3, rotation: Quat, prototype: &str| SpaceshipSectionConfig {
            id: id.to_string(),
            position,
            rotation,
            source: SectionSource::prototype(prototype),
        };
    StationDesign {
        hull: ShipDesign {
            sections: vec![
                section(
                    "core",
                    Vec3::ZERO,
                    Quat::IDENTITY,
                    REINFORCED_HULL_SECTION_ID,
                ),
                section(
                    "bridge",
                    Vec3::X,
                    Quat::IDENTITY,
                    BASIC_CONTROLLER_SECTION_ID,
                ),
                section(
                    "collar",
                    Vec3::NEG_Z,
                    Quat::IDENTITY,
                    DOCKING_PORT_SECTION_ID,
                ),
                section(
                    "gun",
                    Vec3::new(0.0, 0.75, 0.0),
                    Quat::IDENTITY,
                    PDC_KINETIC_TURRET_SECTION_ID,
                ),
            ],
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
    app.init_resource::<GameShipDesigns>();
    app.init_resource::<CargoCanisterIdAllocator>();
    app.insert_resource(test_items());
    app.finish();
    app
}

fn section(app: &App, station: Entity, id: &str) -> Entity {
    let children: Vec<Entity> = app.world().get::<Children>(station).unwrap().to_vec();
    children
        .into_iter()
        .find(|child| {
            app.world()
                .get::<EntityId>(*child)
                .is_some_and(|entity_id| entity_id.0 == id)
        })
        .unwrap_or_else(|| panic!("no live section '{id}'"))
}

/// Whether `station` is a dead station in every way a reader asks.
fn assert_dead(app: &App, station: Entity, context: &str) {
    let root = app.world().entity(station);
    assert!(root.contains::<DerelictShipMarker>(), "{context}: derelict");
    assert!(root.contains::<LootableShipMarker>(), "{context}: lootable");
    assert_eq!(
        root.get::<Allegiance>(),
        Some(&Allegiance::Neutral),
        "{context}: Neutral"
    );
    assert!(!root.contains::<AISpaceshipMarker>(), "{context}: no AI");
    assert!(
        !station_offers(
            root.get::<StationServices>(),
            true,
            StationServiceType::Trade
        ) && root.contains::<DerelictShipMarker>(),
        "{context}: services refused"
    );
}

#[test]
fn a_station_that_loses_its_last_turret_turns_into_a_dead_station() {
    let mut app = app();
    let station = {
        let mut commands = app.world_mut().commands();
        let mut entity = commands.spawn((Transform::default(), EntityId::new("outpost")));
        station_scenario_object(
            &mut entity,
            &design(),
            &StationConfig {
                design: StationDesignSource::Inline(design()),
                allegiance: Allegiance::Enemy,
                derelict: false,
                initial_velocity: MetersPerSecond3::ZERO,
            },
        );
        entity.id()
    };
    settle(&mut app);
    settle(&mut app);
    {
        let root = app.world().entity(station);
        assert!(
            root.contains::<AISpaceshipMarker>(),
            "a living station is AI-driven"
        );
        assert_eq!(root.get::<Allegiance>(), Some(&Allegiance::Enemy));
        assert!(!root.contains::<DerelictShipMarker>());
        assert!(station_offers(
            root.get::<StationServices>(),
            false,
            StationServiceType::Trade
        ));
        assert_eq!(
            root.get::<Children>().unwrap().len(),
            4,
            "every section stands on the root"
        );
    }

    let gun = section(&app, station, "gun");
    app.world_mut().trigger(HealthApplyDamage {
        entity: gun,
        source: None,
        amount: 1.0e6,
    });
    settle(&mut app);
    settle(&mut app);

    assert_dead(&app, station, "after the last turret died");
    let bridge = section(&app, station, "bridge");
    assert!(
        app.world()
            .entity(bridge)
            .contains::<SectionInactiveMarker>(),
        "the flight computer is switched off"
    );
    for live in ["core", "collar"] {
        let entity = section(&app, station, live);
        assert!(
            !app.world()
                .entity(entity)
                .contains::<SectionInactiveMarker>(),
            "'{live}' stays live: structure and port"
        );
    }
    let health = app.world().get::<Health>(station).unwrap();
    assert!(health.current < health.max, "the station took the damage");
    info!("station pool {} of {}", health.current, health.max);
    println!(
        "dead station: pool {:.0} of {:.0}, derelict, lootable, Neutral, no AI",
        health.current, health.max
    );

    let frozen = freeze_station(app.world(), station).expect("a settled station freezes");
    app.world_mut().entity_mut(station).despawn();
    let thawed = {
        let mut commands = app.world_mut().commands();
        let mut entity = commands.spawn((Transform::default(), EntityId::new("outpost")));
        thaw_station(&mut entity, frozen);
        entity.id()
    };
    settle(&mut app);
    assert_dead(&app, thawed, "after a freeze and thaw");
    let thawed_bridge = section(&app, thawed, "bridge");
    assert!(
        app.world()
            .entity(thawed_bridge)
            .contains::<SectionInactiveMarker>(),
        "the thawed flight computer stays off"
    );
}
