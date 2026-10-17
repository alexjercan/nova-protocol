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
    CargoCanisterIdAllocator, Chart, CivilizationStances, ClockFreeze, DamageMarks, GoalRecord,
    JobMixType, JobPostings, Jobs, PlayerSpaceshipMarker, PointRotationOutput, ShipInventory,
    ShipInventoryStock, ShipRoleType,
};
use nova_scenario::prelude::{
    spaceship_scenario_object, CurrentScenario, GameStartRoles, GeneratedMarks, NovaEventWorld,
    OpenWorldSession, RoleGoal, RoleGoalType, ScenarioConfig, SpaceshipConfig, SpentHandlers,
    StartKit, StartRole,
};
use nova_ship::prelude::{
    ChaseCameraPlugin, ChaseZoom, ShipStyleConfig, SpaceshipCameraController,
    SpaceshipCameraInputMarker, SpaceshipCameraNormalInputMarker,
};
use nova_world::prelude::{
    CurrentSector, FrozenSectors, SectorCoord, WorldClock, WorldConfig, WorldObserver,
};

use crate::{
    role_style_id, save, GameStyles, NovaLayeredWorld, StationArchetypeType,
    OPEN_WORLD_ACTIVE_RADIUS, OPEN_WORLD_SECTOR_EDGE,
};

/// The scenario id the fixture's save names.
pub const FIXTURE_SCENARIO_ID: &str = "session";

/// The start role id the fixture's saves name.
pub const FIXTURE_ROLE_ID: &str = "fixture_role";

/// The start role catalog a fixture save is checked against: one role,
/// [`FIXTURE_ROLE_ID`]. Nothing spawns it, so its hull names no design.
pub fn fixture_roles() -> GameStartRoles {
    GameStartRoles(vec![StartRole {
        id: FIXTURE_ROLE_ID.into(),
        name: "Fixture Role".to_string(),
        about: "A role the save fixtures name.".to_string(),
        hull: "fixture_hull".into(),
        kit: StartKit {
            credits: 300,
            stock: ShipInventoryStock::default(),
        },
        jobs: JobMixType::Trade,
        goal: RoleGoal {
            title: "A goal".to_string(),
            kind: RoleGoalType::OwnYard,
        },
        lore: String::new(),
    }])
}

/// The ship-layout fixture packs as the content merge would register them,
/// under digest 1: the catalog [`NovaLayeredWorld`] pins in a test.
pub fn fixture_loaded() -> LoadedSectionPacks {
    LoadedSectionPacks {
        packs: crate::ship_layout::fixture::packs()
            .into_iter()
            .map(|pack| LoadedSectionPack {
                id: pack.id,
                dependencies: pack.dependencies,
                sections: pack.sections,
            })
            .collect(),
        digest: ContentCatalogDigest(1),
    }
}

/// One style for each role's [`role_style_id`] and each station archetype's
/// [`StationArchetypeType::style_id`].
pub fn fixture_styles() -> GameStyles {
    let mut ids: Vec<&str> = ShipRoleType::ALL.into_iter().map(role_style_id).collect();
    ids.extend(
        StationArchetypeType::ALL
            .into_iter()
            .map(StationArchetypeType::style_id),
    );
    ids.sort_unstable();
    ids.dedup();
    GameStyles(
        ids.into_iter()
            .map(|id| ShipStyleConfig {
                id: id.to_string(),
                ..default()
            })
            .collect(),
    )
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
/// the fixture catalog and start roles, an armed config, the ledger, the
/// canister counter and the player camera. Returns the player ship, which has 300 credits.
///
/// Add [`WorldSaveTestPlugin`] first: the camera's chase state comes from an
/// observer that its chase plugin adds, and without it no save can complete.
pub fn arm_save_fixture(world: &mut World) -> Entity {
    bevy::tasks::IoTaskPool::get_or_init(bevy::tasks::TaskPool::default);
    let loaded = fixture_loaded();
    let items = nova_gameplay::test_support::test_items();
    let generator = NovaLayeredWorld::from_loaded(&loaded, &fixture_styles(), &items).expect(
        "the fixture packs build a snapshot and every role and station archetype has a style",
    );
    // The session New Game would have inserted: the save snapshots its
    // starting station.
    world.insert_resource(OpenWorldSession {
        seed: 42,
        role: FIXTURE_ROLE_ID.into(),
        start: crate::start::starting_station(42, &generator)
            .expect("seed 42 has a living ally station near the origin"),
    });
    world.insert_resource(loaded);
    world.insert_resource(items);
    world.insert_resource(fixture_roles());
    world.insert_resource(WorldConfig {
        seed: 42,
        sector_edge: OPEN_WORLD_SECTOR_EDGE,
        active_radius: OPEN_WORLD_ACTIVE_RADIUS,
        generator,
    });
    world.init_resource::<FrozenSectors>();
    world.init_resource::<WorldClock>();
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
    world.init_resource::<Chart>();
    world.init_resource::<Jobs>();
    world.init_resource::<JobPostings>();
    world.init_resource::<CivilizationStances>();
    world.init_resource::<GoalRecord>();
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
