//! A saving open world for tests outside this crate, behind the dev-only
//! `test-support` feature.
//!
//! The fixture arms a world over a small catalog and puts one player ship in
//! it. [`WorldSaveTestPlugin`] runs the production save systems on it, so a
//! [`WorldSaveSession`](crate::WorldSaveSession) snapshots, encodes and
//! writes exactly as in the game. Nothing streams: the window stays empty,
//! and the player is the whole world.

use avian3d::prelude::Rotation;
use bevy::prelude::*;
use nova_assets::prelude::{ContentCatalogDigest, LoadedSectionPack, LoadedSectionPacks};
use nova_events::prelude::EntityId;
use nova_gameplay::prelude::{
    CargoCanisterIdAllocator, ClockFreeze, DamageMarks, PlayerSpaceshipMarker, PointRotationOutput,
    ShipInventory, ShipInventoryStock,
};
use nova_scenario::prelude::{
    spaceship_scenario_object, CurrentScenario, GameStationDesigns, GeneratedMarks, NovaEventWorld,
    ScenarioConfig, SectionSource, ShipDesign, SpaceshipConfig, SpaceshipSectionConfig,
    SpentHandlers, StationDesign, StationServiceType,
};
use nova_ship::prelude::{
    ChaseCameraPlugin, ChaseZoom, ShipStyleConfig, SpaceshipCameraController,
    SpaceshipCameraInputMarker, SpaceshipCameraNormalInputMarker,
};
use nova_world::prelude::{CurrentSector, FrozenSectors, SectorCoord, WorldConfig, WorldObserver};

use crate::{
    role_style_id, save, GameStyles, NovaLayeredWorld, ShipRoleType, BLOCK_WAYSTATION_STATION_ID,
    OPEN_WORLD_ACTIVE_RADIUS, OPEN_WORLD_SECTOR_EDGE,
};

/// The scenario id the fixture's save names.
pub const FIXTURE_SCENARIO_ID: &str = "session";

/// The one station design the generator places, built from the fixture
/// packs' `dock`, `controller` and `cube` prototypes: a docking collar, a
/// flight computer and a plate, no drive.
pub fn fixture_stations() -> GameStationDesigns {
    let section = |id: &str, position: Vec3, prototype: &str| SpaceshipSectionConfig {
        id: id.into(),
        position,
        rotation: Quat::IDENTITY,
        source: SectionSource::prototype(prototype),
    };
    GameStationDesigns(vec![StationDesign {
        id: BLOCK_WAYSTATION_STATION_ID.into(),
        name: "Fixture Waystation".to_string(),
        about: "A collar, a bridge and a plate.".to_string(),
        hull: ShipDesign {
            sections: vec![
                section("collar", Vec3::NEG_X, "dock"),
                section("bridge", Vec3::Y, "controller"),
                section("plate", Vec3::ZERO, "cube"),
            ],
            ..default()
        },
        services: vec![StationServiceType::Trade],
        storage_cap_g: 0,
        stock: ShipInventoryStock::default(),
        credits: 100,
    }])
}

/// The production save systems and the transient restore before them, in
/// their production order, with no streaming before them, and the chase sync
/// that solves the player camera a save keeps.
pub struct WorldSaveTestPlugin;

impl Plugin for WorldSaveTestPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                save::restore_resumed_transients.run_if(resource_exists::<save::ResumedTransients>),
                save::save_systems(),
            )
                .chain(),
        );
        if !app.is_plugin_added::<ChaseCameraPlugin>() {
            app.add_plugins(ChaseCameraPlugin);
        }
    }
}

/// Arm `world` as an open world with one player ship and no live sector:
/// the fixture catalog, an armed config, the ledger, the canister counter
/// and the player camera. Returns the player ship, which has 300 credits.
///
/// Add [`WorldSaveTestPlugin`] first: the camera's chase state comes from an
/// observer that its chase plugin adds, and without it no save can complete.
pub fn arm_save_fixture(world: &mut World) -> Entity {
    bevy::tasks::IoTaskPool::get_or_init(bevy::tasks::TaskPool::default);
    let loaded = LoadedSectionPacks {
        packs: crate::ship_layout::fixture::packs()
            .into_iter()
            .map(|pack| LoadedSectionPack {
                id: pack.id,
                dependencies: pack.dependencies,
                sections: pack.sections,
            })
            .collect(),
        digest: ContentCatalogDigest(1),
    };
    let styles = GameStyles(
        ShipRoleType::ALL
            .into_iter()
            .map(|role| ShipStyleConfig {
                id: role_style_id(role).to_string(),
                ..default()
            })
            .collect(),
    );
    let items = nova_gameplay::test_support::test_items();
    let stations = fixture_stations();
    let generator = NovaLayeredWorld::from_loaded(&loaded, &styles, &items, &stations)
        .expect("the fixture packs build a snapshot, every role has a style and the station pins");
    world.insert_resource(loaded);
    world.insert_resource(items);
    world.insert_resource(WorldConfig {
        seed: 42,
        sector_edge: OPEN_WORLD_SECTOR_EDGE,
        active_radius: OPEN_WORLD_ACTIVE_RADIUS,
        generator,
    });
    world.init_resource::<FrozenSectors>();
    world.init_resource::<CargoCanisterIdAllocator>();
    world.init_resource::<Time<Virtual>>();
    world.init_resource::<Time<Real>>();
    world.init_resource::<ClockFreeze>();
    // The chase sync reads the generic clock, which `TimePlugin` would add.
    world.init_resource::<Time>();
    // The save names its scenario and takes the scenario state. A rig that
    // loaded a scenario of its own first keeps it; otherwise a bare session
    // scenario with nothing done yet.
    if world
        .get_resource::<CurrentScenario>()
        .is_none_or(|current| current.0.is_none())
    {
        world.insert_resource(CurrentScenario(Some(ScenarioConfig::new(
            FIXTURE_SCENARIO_ID,
            "Session",
            default(),
        ))));
    }
    world.init_resource::<NovaEventWorld>();
    world.init_resource::<SpentHandlers>();
    world.init_resource::<GeneratedMarks>();
    world.insert_resource(CurrentSector(SectorCoord::ORIGIN));
    let player = world
        .spawn((
            spaceship_scenario_object(SpaceshipConfig {
                credits: 300,
                ..default()
            }),
            ShipInventory::default(),
            DamageMarks::default(),
            Transform::from_xyz(5.0, 0.0, -2.0),
            GlobalTransform::from_xyz(5.0, 0.0, -2.0),
            Rotation::default(),
            PlayerSpaceshipMarker,
            WorldObserver,
            EntityId("player".to_string()),
        ))
        .id();
    // The player camera, as the controller builds it, with no input. The
    // chase sync of the first frame solves its pose, so the first save
    // waits one frame for it, as in the game.
    world.init_resource::<ChaseZoom>();
    world.spawn(SpaceshipCameraController).with_child((
        SpaceshipCameraInputMarker,
        SpaceshipCameraNormalInputMarker,
        PointRotationOutput::default(),
    ));
    player
}
