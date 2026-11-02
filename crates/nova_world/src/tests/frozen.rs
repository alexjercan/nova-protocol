//! What a cell keeps while it is off-window, and which cell keeps a body.
//!
//! Each proof drives the real stage systems - adoption, retirement, cleanup -
//! against a bare or headless app and stands in for the worker with the same
//! preparation a job runs, so a record goes through exactly the path a live
//! session takes it through.

use std::time::Duration;

use avian3d::prelude::*;
use bevy::{ecs::system::RunSystemOnce, prelude::*};
use nova_events::prelude::{EntityId, Meters, Meters3, ScenarioAddressableMarker};
use nova_gameplay::{
    prelude::{
        circular_orbit_speed, AssetRef, BuildRecipe, CargoCanister, CargoCanisterRuntimeId,
        DamageMark, DamageMarks, DerelictShipMarker, GravitySettings, GravityWell, Health,
        HealthApplyDamage, IntegrityDestroyMarker, ItemDesignId, ItemRoleType, NovaGravityPlugin,
        PlayerSpaceshipMarker, SectionMarker, ShipCredits, ShipInventory, ShipInventoryStock,
        SpaceshipRootMarker, TempEntity, TraderMarket,
    },
    test_support::{settle, test_items, unfinished_integrity_physics_app},
};
use nova_scenario::prelude::{
    freeze_ship, station_orbit_velocity, AsteroidCarvePlugin, AsteroidField, AsteroidMarker,
    AsteroidPlugin, CurrentScenario, MinedCanisterQueue, MinedOreDrop, MiningPlugin,
    ParkedHullsPlugin, SectionSource, ShipDesign, ShipDesignSource, SpaceshipController,
    SpaceshipDesign, SpaceshipPlugin, SpaceshipSectionConfig, StationBuilds, StationDesignSource,
    StationHullOffer, StationMarker, StationServices, StationStorage, WreckCargoPlugin,
};
use nova_ship::prelude::{
    cargo_canister, docking_section, thruster_section, AINonCombatant, BaseSectionConfig,
    BodyRadius, DockedHelmType, DockedShip, DockingConnection, DockingConnectionRequest,
    DockingHelmRequest, DockingSectionConfig, FlightIntent, GameSections, HullOfferType,
    NovaFlightPlugin, PDControllerPlugin, ParkedHull, SavedJointFrame, SectionAnimations,
    SectionConfig, SectionKind, ShipCapabilities, SpaceshipSectionPlugin, ThrusterSectionConfig,
    TurretSectionConfig, UndockSeparation,
};

use super::{
    answering, design_mut, empty, one_ship, rock_config, rocks, ship, station, test_sections,
    Answers, Rocks, TEST_HULL_SECTION_ID,
};
use crate::{
    adopt_moving_bodies, clear_sector_work, frozen::SettlingBodies, generation::prepare_cell,
    materialize_sector, retire_sectors, sector_id, streaming::ClearedConfig, CurrentSector,
    FrozenBody, FrozenBodyType, FrozenSector, FrozenSectors, GeneratedBodyId, ObserverBody,
    PendingSectorShip, PendingSectorStation, ReadySectors, SectorAsteroid, SectorCoord,
    SectorGenerationInput, SectorGenerator, SectorJobStats, SectorManifest, SectorRoot,
    SectorSnapshotError, ThawedAfter, WorldClock, WorldConfig, WorldObserver,
};

/// Arm `config` around `centre` with nothing live yet, so the arming frame's
/// cleanup has nothing to take and every stage after it accepts the config.
fn arm<G: SectorGenerator>(world: &mut World, config: WorldConfig<G>, centre: SectorCoord) {
    world.insert_resource(config);
    world.init_resource::<CurrentScenario>();
    world.init_resource::<ReadySectors>();
    world.init_resource::<SectorJobStats>();
    world.init_resource::<ClearedConfig>();
    world.init_resource::<FrozenSectors>();
    world.init_resource::<WorldClock>();
    world.init_resource::<SettlingBodies>();
    world.init_resource::<Time<Virtual>>();
    world
        .run_system_once(clear_sector_work::<G>)
        .expect("cleanup runs");
    world.insert_resource(CurrentSector(centre));
}

/// The observer stands in `centre`, and the frame's last two stages run.
fn walk_to<G: SectorGenerator>(world: &mut World, centre: SectorCoord) {
    world.insert_resource(CurrentSector(centre));
    world
        .run_system_once(adopt_moving_bodies::<G>)
        .expect("adoption runs");
    world
        .run_system_once(retire_sectors::<G>)
        .expect("retirement runs");
}

/// Prepare `coord` the way its job would from the cell's record, and spawn
/// it with the record, the observer at `observer`. Returns the root.
fn materialize<G: SectorGenerator>(
    world: &mut World,
    coord: SectorCoord,
    observer: ObserverBody,
) -> Entity {
    let config = world.resource::<WorldConfig<G>>().clone();
    let record = world.resource::<FrozenSectors>().shared(coord);
    let prepared = prepare_cell(config, coord, record).expect("the cell prepares");
    world.resource_mut::<FrozenSectors>().consume(coord);
    let sections = world
        .get_resource::<GameSections>()
        .cloned()
        .unwrap_or_default();
    let clock = *world.resource::<WorldClock>();
    let root = materialize_sector(&mut world.commands(), prepared, &sections, observer, &clock);
    world.flush();
    root
}

/// An observer nowhere near any test body.
fn far() -> ObserverBody {
    ObserverBody {
        position: Meters3::new(0.0, 1.0e6, 0.0),
        reach: Meters::ZERO,
    }
}

/// The ids of the frozen bodies a cell's record holds.
fn frozen_ids(world: &World, coord: SectorCoord) -> Vec<String> {
    world
        .resource::<FrozenSectors>()
        .get(coord)
        .map(|record| {
            record
                .bodies()
                .iter()
                .filter_map(|body| body.id().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// The persistent body called `id`.
fn body_with_id(world: &mut World, id: &GeneratedBodyId) -> Vec<Entity> {
    world
        .query::<(Entity, &GeneratedBodyId)>()
        .iter(world)
        .filter(|(_, each)| *each == id)
        .map(|(entity, _)| entity)
        .collect()
}

/// A bare docking hull: a root, a collider and one port section named
/// `fore`, which the hull is sized from.
fn docking_hull(app: &mut App, at: Vec3, rotation: Quat, parent: Option<Entity>) -> Entity {
    let mut ship = app.world_mut().spawn((
        Name::new("hull"),
        SpaceshipRootMarker,
        RigidBody::Dynamic,
        Transform::from_translation(at).with_rotation(rotation),
    ));
    if let Some(parent) = parent {
        ship.insert(ChildOf(parent));
    }
    let ship = ship.id();
    app.world_mut().spawn((
        ChildOf(ship),
        Transform::default(),
        Collider::cuboid(1.0, 1.0, 1.0),
        ColliderDensity(1.0),
    ));
    app.world_mut().spawn((
        ChildOf(ship),
        EntityId("fore".to_string()),
        SectionMarker,
        Transform::default(),
        docking_section(DockingSectionConfig::default()),
    ));
    ship
}

/// The `fore` docking port `docking_hull` builds `ship` with.
fn fore_section(world: &mut World, ship: Entity) -> Entity {
    world
        .query::<(Entity, &EntityId, &ChildOf)>()
        .iter(world)
        .find(|(_, id, child_of)| id.0 == "fore" && child_of.parent() == ship)
        .map(|(entity, ..)| entity)
        .expect("the hull has a fore section")
}

/// The prototype a thawed ship's own `fore` docking port resolves against.
const TEST_DOCKING_SECTION_ID: &str = "test_docking";

/// A single-section design naming `TEST_DOCKING_SECTION_ID` as `fore`: what a
/// thawed `docking_hull` ship resolves its frozen `fore` section against.
fn docking_hull_design() -> SpaceshipDesign {
    SpaceshipDesign(ShipDesignSource::Inline(ShipDesign {
        sections: vec![SpaceshipSectionConfig {
            id: "fore".into(),
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            source: SectionSource::prototype(TEST_DOCKING_SECTION_ID),
        }],
        ..default()
    }))
}

/// Load `TEST_DOCKING_SECTION_ID` into `app`'s section catalog, alongside the
/// hull prototype `ship_app` already seeds it with.
fn load_docking_section(app: &mut App) {
    app.world_mut()
        .resource_mut::<GameSections>()
        .0
        .push(SectionConfig {
            base: BaseSectionConfig {
                id: TEST_DOCKING_SECTION_ID.to_string(),
                advancement: 0.0,
                draw_weight: 1.0,
                ..default()
            },
            kind: SectionKind::Docking(DockingSectionConfig::default()),
        });
}

/// A generator that places nothing.
#[derive(Clone, Debug, PartialEq)]
struct Nothing;

impl SectorGenerator for Nothing {
    fn validate(&self, _geometry: crate::WorldGeometry) -> Result<(), crate::SectorFault> {
        Ok(())
    }

    fn generate(&self, input: SectorGenerationInput) -> Result<SectorManifest, crate::SectorFault> {
        Ok(empty(input.coord, Vec::new()))
    }
}

/// A ship docked to the player belongs to the cell it stands in, not the one
/// that generated it. The player takes the helm and burns the pair out of
/// the partner's home cell, and each frame the window follows the player.
/// When home leaves the window, the partner has already moved with the pair:
/// the dock holds, the player stays top-level, and home freezes empty.
///
/// Before the Adopt stage this partner was despawned with its home root and
/// the dock released.
#[test]
fn a_docked_partner_moves_to_the_cell_it_stands_in_when_its_home_cell_retires() {
    let mut app = ship_app();

    let home = SectorCoord::ORIGIN;
    // A short edge and a one-cell window, so a full burn carries the pair
    // two cells out of home in about two seconds.
    let edge = Meters(320.0);
    arm(
        app.world_mut(),
        WorldConfig {
            seed: 1,
            sector_edge: edge,
            active_radius: 1,
            generator: Nothing,
        },
        home,
    );
    let home_root = app
        .world_mut()
        .spawn((SectorRoot(home), Transform::default()))
        .id();
    let centre = home.centre(edge).to_engine();
    let player = docking_hull(&mut app, centre, Quat::IDENTITY, None);
    app.world_mut().entity_mut(player).insert((
        WorldObserver,
        PlayerSpaceshipMarker,
        FlightIntent::default(),
    ));
    app.world_mut().spawn((
        ChildOf(player),
        SectionMarker,
        Transform::default(),
        thruster_section(ThrusterSectionConfig::default()),
    ));
    let partner = docking_hull(
        &mut app,
        centre + Vec3::NEG_Z * 1.5,
        Quat::from_rotation_y(std::f32::consts::PI),
        Some(home_root),
    );
    settle(&mut app);
    app.world_mut().trigger(DockingConnectionRequest {
        entity: player,
        target: partner,
    });
    settle(&mut app);
    app.world_mut()
        .trigger(DockingHelmRequest { entity: player });
    settle(&mut app);
    let world = app.world_mut();
    assert_eq!(
        world
            .query::<&DockingConnection>()
            .iter(world)
            .map(|connection| connection.helm)
            .collect::<Vec<_>>(),
        vec![DockedHelmType::Held(player)],
        "the player docks and takes the helm before the burn"
    );

    app.world_mut()
        .entity_mut(player)
        .insert(FlightIntent { burn: 1.0 });
    let destination = home.offset(0, 0, -2);
    let mut crossed = vec![home];
    for _ in 0..600 {
        app.update();
        let world = app.world_mut();
        let at = world
            .get::<Position>(player)
            .expect("the player has a pose")
            .0;
        let cell = SectorCoord::containing(Meters3::from_engine(at), edge);
        if crossed.last() != Some(&cell) {
            crossed.push(cell);
            // Stand in for Materialize: the cell the player enters is live.
            world.spawn((SectorRoot(cell), Transform::default()));
        }
        walk_to::<Nothing>(world, cell);
        if world.get_entity(home_root).is_err() {
            break;
        }
    }

    let world = app.world_mut();
    assert_eq!(
        crossed,
        vec![home, home.offset(0, 0, -1), destination],
        "the helm burn flies the pair two cells out of home"
    );
    assert!(
        world.get_entity(home_root).is_err(),
        "the home cell retired"
    );
    let roots: Vec<(Entity, SectorCoord)> = world
        .query::<(Entity, &SectorRoot)>()
        .iter(world)
        .map(|(entity, root)| (entity, root.0))
        .collect();
    let partner_cell = world
        .get::<ChildOf>(partner)
        .and_then(|child_of| roots.iter().find(|(root, _)| *root == child_of.parent()))
        .map(|(_, coord)| *coord);
    let partner_at = world.get::<Position>(partner).expect("the partner lives").0;
    assert_eq!(
        partner_cell,
        Some(SectorCoord::containing(
            Meters3::from_engine(partner_at),
            edge
        )),
        "the partner belongs to the cell it stands in"
    );
    assert!(
        world.get::<ChildOf>(player).is_none(),
        "the player stays top-level"
    );
    assert!(world.get::<DockedShip>(player).is_some());
    assert!(world.get::<DockedShip>(partner).is_some());
    assert_eq!(
        world.query::<&DockingConnection>().iter(world).count(),
        1,
        "the dock survives the crossing"
    );
    assert!(
        matches!(world.resource::<FrozenSectors>().get(home), Some(FrozenSector::Visited(bodies)) if bodies.is_empty()),
        "the home cell froze with nothing in it"
    );
}

/// A retiring cell takes no new body. A ship docked to the player stands
/// across the face of a desired cell whose root is still retiring: it stays
/// with the cell it came from rather than join a root that would freeze it
/// and split the pair, and joins the cell once it comes back from its record.
#[test]
fn a_docked_partner_waits_out_a_retiring_cell_it_stands_in() {
    let mut app = ship_app();

    let home = SectorCoord::ORIGIN;
    let ahead = home.offset(0, 0, -1);
    let edge = Meters(320.0);
    arm(
        app.world_mut(),
        WorldConfig {
            seed: 1,
            sector_edge: edge,
            active_radius: 1,
            generator: Nothing,
        },
        home,
    );
    let home_root = app
        .world_mut()
        .spawn((SectorRoot(home), Transform::default()))
        .id();
    let retiring_root = app
        .world_mut()
        .spawn((SectorRoot(ahead), Transform::default()))
        .id();
    let face = (home.centre(edge).to_engine() + ahead.centre(edge).to_engine()) * 0.5;
    let player = docking_hull(&mut app, face + Vec3::Z * 0.75, Quat::IDENTITY, None);
    app.world_mut().entity_mut(player).insert((
        WorldObserver,
        PlayerSpaceshipMarker,
        FlightIntent::default(),
    ));
    let partner = docking_hull(
        &mut app,
        face + Vec3::NEG_Z * 0.75,
        Quat::from_rotation_y(std::f32::consts::PI),
        Some(home_root),
    );
    settle(&mut app);
    app.world_mut().trigger(DockingConnectionRequest {
        entity: player,
        target: partner,
    });
    settle(&mut app);

    let world = app.world_mut();
    assert!(world.get::<DockedShip>(partner).is_some(), "the pair docks");
    let cell_of = |world: &World, ship: Entity| {
        let at = world.get::<Position>(ship).expect("the ship has a pose").0;
        SectorCoord::containing(Meters3::from_engine(at), edge)
    };
    assert_eq!(
        (cell_of(world, player), cell_of(world, partner)),
        (home, ahead),
        "the pair straddles the face"
    );
    // The record already holds part of the cell, so its root is retiring.
    world
        .resource_mut::<FrozenSectors>()
        .visit(ahead, Vec::new());
    walk_to::<Nothing>(world, home);
    assert!(
        world.get_entity(retiring_root).is_err(),
        "the retiring root finished retiring"
    );
    assert_eq!(
        world.get::<ChildOf>(partner).map(ChildOf::parent),
        Some(home_root),
        "the partner stays with the cell it came from"
    );
    assert!(world.get::<DockedShip>(partner).is_some());
    assert_eq!(
        world.query::<&DockingConnection>().iter(world).count(),
        1,
        "the dock holds"
    );

    let ahead_root = materialize::<Nothing>(world, ahead, far());
    walk_to::<Nothing>(world, home);
    assert_eq!(
        world.get::<ChildOf>(partner).map(ChildOf::parent),
        Some(ahead_root),
        "the partner joins the cell once it comes back"
    );
    assert!(world.get::<DockedShip>(partner).is_some());
}

/// A rock that drifts out of the window into a cell nobody has generated is
/// frozen there as an arrival and despawned; a projectile out there is just
/// despawned. The rock's cell is read from its physics `Position`: its
/// interpolated `Transform` still stands in home. When the cell first generates, the arrival comes back where it
/// froze beside every generated rock - including the one it overlaps, which
/// is not removed.
#[test]
fn a_rock_that_leaves_the_window_arrives_in_its_ungenerated_cell_beside_its_generated_rocks() {
    let mut app = App::new();
    app.add_plugins(AsteroidPlugin { render: false });
    let world = app.world_mut();
    let config = rocks();
    let edge = config.sector_edge;
    let home = SectorCoord::ORIGIN;
    let far_cell = home.offset(5, 0, 0);
    arm(world, config.clone(), home);
    materialize::<Rocks>(world, home, far());
    app.update();

    // The far cell's own second rock stands here: the arrival overlaps it.
    let quarter = edge.get() * 0.25;
    let landing = far_cell.centre(edge) + Meters3::new(-quarter, 0.0, quarter);
    let wanderer = sector_id(home, "body", 0);
    let world = app.world_mut();
    let [rock] = body_with_id(world, &wanderer)[..] else {
        panic!("home must spawn its first rock");
    };
    assert_eq!(
        SectorCoord::containing(
            Meters3::from_engine(world.get::<Transform>(rock).unwrap().translation),
            edge
        ),
        home,
        "the rock's trailing transform stays in home"
    );
    world.entity_mut(rock).insert(Position(landing.to_engine()));
    let stray = world
        .spawn((
            TempEntity(30.0),
            Transform::from_translation(landing.to_engine()),
        ))
        .id();
    let near = world.spawn((TempEntity(30.0), Transform::default())).id();

    walk_to::<Rocks>(world, home);
    assert!(
        world.get_entity(rock).is_err(),
        "the rock froze out of the window"
    );
    assert!(
        world.get_entity(stray).is_err(),
        "a transient out there is dropped"
    );
    assert!(
        world.get_entity(near).is_ok(),
        "a transient in the window lives"
    );
    assert!(
        matches!(
            world.resource::<FrozenSectors>().get(far_cell),
            Some(FrozenSector::Arrivals(_))
        ),
        "an ungenerated cell keeps the rock as an arrival"
    );
    assert_eq!(frozen_ids(world, far_cell), vec![wanderer.to_string()]);

    walk_to::<Rocks>(world, far_cell);
    let root = materialize::<Rocks>(world, far_cell, far());
    app.update();

    let world = app.world_mut();
    let mut under: Vec<(String, Vec3)> = world
        .query_filtered::<(&EntityId, &Transform, &ChildOf), With<AsteroidMarker>>()
        .iter(world)
        .filter(|(_, _, child_of)| child_of.parent() == root)
        .map(|(id, transform, _)| (id.0.clone(), transform.translation))
        .collect();
    under.sort_by(|a, b| a.0.cmp(&b.0));
    let mut expected: Vec<String> = (0..4)
        .map(|index| sector_id(far_cell, "body", index).to_string())
        .collect();
    expected.push(wanderer.to_string());
    expected.sort();
    assert_eq!(
        under.iter().map(|(id, _)| id.clone()).collect::<Vec<_>>(),
        expected,
        "the generated rocks and the arrival all stand in the cell"
    );
    let at = |id: &GeneratedBodyId| {
        under
            .iter()
            .find(|(each, _)| *each == id.to_string())
            .unwrap()
            .1
    };
    assert_eq!(
        at(&wanderer),
        landing.to_engine(),
        "the arrival is where it froze"
    );
    assert_eq!(
        at(&sector_id(far_cell, "body", 1)),
        landing.to_engine(),
        "the overlapped generated rock is not removed"
    );
    assert!(
        world.resource::<FrozenSectors>().get(far_cell).is_none(),
        "generation consumed the arrivals"
    );
}

/// What a session ends, it forgets: cleanup drops every frozen record, and
/// the cell that held one generates again from the seed.
#[test]
fn ending_the_session_forgets_every_frozen_cell() {
    let mut app = App::new();
    app.add_plugins(AsteroidPlugin { render: false });
    let world = app.world_mut();
    let home = SectorCoord::ORIGIN;
    arm(world, rocks(), home);
    materialize::<Rocks>(world, home, far());
    app.update();

    let world = app.world_mut();
    let [rock] = body_with_id(world, &sector_id(home, "body", 0))[..] else {
        panic!("home must spawn its first rock");
    };
    world.entity_mut(rock).despawn();
    walk_to::<Rocks>(world, home.offset(5, 0, 0));
    assert_eq!(
        frozen_ids(world, home).len(),
        3,
        "home froze without the rock"
    );

    world.resource_mut::<WorldClock>().play_seconds = 40.0;
    world.remove_resource::<WorldConfig<Rocks>>();
    world
        .run_system_once(clear_sector_work::<Rocks>)
        .expect("cleanup runs");
    assert!(
        world.resource::<FrozenSectors>().is_empty(),
        "the session's records are gone"
    );
    assert_eq!(
        *world.resource::<WorldClock>(),
        WorldClock::default(),
        "the session's play time is gone with its records"
    );

    arm(world, rocks(), home);
    let root = materialize::<Rocks>(world, home, far());
    app.update();
    let world = app.world_mut();
    let ids: Vec<String> = world
        .query_filtered::<(&EntityId, &ChildOf), With<AsteroidMarker>>()
        .iter(world)
        .filter(|(_, child_of)| child_of.parent() == root)
        .map(|(id, _)| id.0.clone())
        .collect();
    assert_eq!(
        ids.len(),
        4,
        "a forgotten cell generates whole again: {ids:?}"
    );
}

/// A body thawed from a record carries the play time it stood frozen: the
/// clock at its thaw less the clock at its freeze. A body the generator
/// spawns fresh has stood frozen for no time and carries nothing.
#[test]
fn a_thawed_body_carries_the_play_time_it_stood_frozen() {
    let mut app = App::new();
    app.add_plugins(AsteroidPlugin { render: false });
    let world = app.world_mut();
    let home = SectorCoord::ORIGIN;
    let away = home.offset(5, 0, 0);
    arm(world, rocks(), home);
    let fresh = materialize::<Rocks>(world, home, far());
    app.update();

    let world = app.world_mut();
    let thawed_after = |world: &mut World, root: Entity| -> Vec<Option<f64>> {
        world
            .query_filtered::<(&ChildOf, Option<&ThawedAfter>), With<AsteroidMarker>>()
            .iter(world)
            .filter(|(child_of, _)| child_of.parent() == root)
            .map(|(_, after)| after.map(|after| after.play_seconds))
            .collect()
    };
    assert_eq!(
        thawed_after(world, fresh),
        vec![None; 4],
        "a generated rock was never frozen"
    );

    world.resource_mut::<WorldClock>().play_seconds = 10.0;
    walk_to::<Rocks>(world, away);
    let stamps: Vec<f64> = world
        .resource::<FrozenSectors>()
        .get(home)
        .expect("home froze")
        .bodies()
        .iter()
        .map(FrozenBody::frozen_at)
        .collect();
    assert_eq!(
        stamps,
        vec![10.0; 4],
        "every rock froze at the clock's reading"
    );

    world.resource_mut::<WorldClock>().play_seconds = 25.0;
    walk_to::<Rocks>(world, home);
    let root = materialize::<Rocks>(world, home, far());
    app.update();
    let world = app.world_mut();
    assert_eq!(
        thawed_after(world, root),
        vec![Some(15.0); 4],
        "each rock stood frozen for the play time between its freeze and its thaw"
    );
}

/// The turret prototype the fought ship's aft section is built from.
const TEST_TURRET_SECTION_ID: &str = "test_turret";

/// How many validated remeshes the rocks reported.
#[derive(Resource, Default)]
struct Remeshes(u32);

/// A cell of one rock, one ship to fight and loot, and one ship the
/// observer will stand in so it is held.
fn wreckable(input: SectorGenerationInput) -> SectorManifest {
    let edge = input.geometry.sector_edge;
    let centre = input.coord.centre(edge);
    let quarter = edge.get() * 0.25;
    let rock = SectorAsteroid {
        id: sector_id(input.coord, "body", 0),
        position: centre + Meters3::new(-quarter, 0.0, 0.0),
        config: rock_config(Meters(60.0), 3),
    };
    let mut fought = ship(
        sector_id(input.coord, "ship", 0),
        centre + Meters3::new(quarter, 0.0, 0.0),
        TEST_HULL_SECTION_ID,
    );
    let design = design_mut(&mut fought);
    let mut aft = design.sections[0].clone();
    design.sections[0].id = "fore".into();
    aft.id = "aft".into();
    aft.position = Vec3::Z;
    aft.source = SectionSource::prototype(TEST_TURRET_SECTION_ID);
    design.sections.push(aft);
    fought.spawn.inventory = ShipInventoryStock::new([(ItemRoleType::IronOre.into(), 10)]);
    let held = ship(
        sector_id(input.coord, "ship", 1),
        centre + Meters3::new(0.0, 0.0, quarter),
        TEST_HULL_SECTION_ID,
    );
    let mut manifest = empty(input.coord, vec![rock]);
    manifest.ships = vec![fought, held];
    manifest
}

/// The world-side ship and rock plugins, headless.
fn ship_app() -> App {
    let mut app = unfinished_ship_app();
    app.finish();
    app
}

/// [`ship_app`] before `finish`, for a test that adds plugins of its own.
fn unfinished_ship_app() -> App {
    let mut app = unfinished_integrity_physics_app();
    app.add_plugins((
        PDControllerPlugin,
        SpaceshipSectionPlugin { render: false },
        SpaceshipPlugin,
        NovaFlightPlugin,
        AsteroidPlugin { render: false },
        AsteroidCarvePlugin { render: false },
    ));
    app.insert_resource(test_sections());
    app.insert_resource(test_items());
    app.init_resource::<nova_gameplay::inventory::CargoCanisterIdAllocator>();
    app.init_resource::<Remeshes>();
    app.add_observer(
        |_: On<nova_scenario::prelude::AsteroidRemeshed>, mut remeshes: ResMut<Remeshes>| {
            remeshes.0 += 1;
        },
    );
    app
}

/// The field node under a rock root: the child that takes marks.
fn rock_node(world: &World, rock: Entity) -> Entity {
    world
        .get::<Children>(rock)
        .and_then(|children| {
            children
                .iter()
                .find(|child| world.get::<DamageMarks>(*child).is_some())
        })
        .expect("a rock has a field node")
}

/// The section ids under a ship root.
fn section_ids(world: &mut World, ship: Entity) -> Vec<String> {
    let mut ids: Vec<String> = world
        .query_filtered::<(&EntityId, &ChildOf), With<SectionMarker>>()
        .iter(world)
        .filter(|(_, child_of)| child_of.parent() == ship)
        .map(|(id, _)| id.0.clone())
        .collect();
    ids.sort();
    ids
}

/// Home, live around an observer holding its second ship: its rock carved
/// by one crater through to a validated remesh, and its first ship fought
/// (aft turret destroyed, fore hurt) and looted (7 ore and every credit
/// taken). Returns the rock and the fought ship.
fn wreck_home(app: &mut App) -> (Entity, Entity) {
    app.world_mut()
        .resource_mut::<GameSections>()
        .0
        .push(SectionConfig {
            base: BaseSectionConfig {
                id: TEST_TURRET_SECTION_ID.to_string(),
                advancement: 0.0,
                draw_weight: 1.0,
                ..default()
            },
            kind: SectionKind::Turret(TurretSectionConfig::default()),
        });
    let home = SectorCoord::ORIGIN;
    let config = answering(wreckable);
    let edge = config.sector_edge;
    let held_at = home.centre(edge) + Meters3::new(0.0, 0.0, edge.get() * 0.25);
    arm(app.world_mut(), config, home);
    materialize::<Answers>(
        app.world_mut(),
        home,
        ObserverBody {
            position: held_at,
            reach: Meters::ZERO,
        },
    );
    settle(app);

    let world = app.world_mut();
    let [rock] = body_with_id(world, &sector_id(home, "body", 0))[..] else {
        panic!("home must spawn its rock");
    };
    let [fought] = body_with_id(world, &sector_id(home, "ship", 0))[..] else {
        panic!("home must spawn the ship to fight");
    };
    // Mined: one crater, carried through the carve chain to a validated
    // remesh.
    let node = rock_node(world, rock);
    world
        .get_mut::<DamageMarks>(node)
        .expect("the node takes marks")
        .0
        .push(DamageMark {
            at: Vec3::new(6.0, 0.0, 0.0),
            radius: 3.0,
        });
    for _ in 0..600 {
        app.update();
        if app.world().resource::<Remeshes>().0 > 0 {
            break;
        }
    }
    assert!(
        app.world().resource::<Remeshes>().0 > 0,
        "the crater never remeshed"
    );
    // Fought: the aft turret is gone and the fore one is hurt.
    let world = app.world_mut();
    let sections: Vec<(Entity, String)> = world
        .query_filtered::<(Entity, &EntityId, &ChildOf), With<SectionMarker>>()
        .iter(world)
        .filter(|(_, _, child_of)| child_of.parent() == fought)
        .map(|(entity, id, _)| (entity, id.0.clone()))
        .collect();
    for (section, id) in sections {
        if id == "aft" {
            world.entity_mut(section).despawn();
        } else {
            let mut health = world
                .get_mut::<Health>(section)
                .expect("a section has health");
            health.current = health.max * 0.4;
        }
    }
    // Looted.
    world
        .get_mut::<ShipInventory>(fought)
        .expect("a ship has a hold")
        .remove(&ItemRoleType::IronOre.into(), 7);
    world.entity_mut(fought).insert(ShipCredits(0));
    settle(app);
    (rock, fought)
}

/// A cell left mined, fought and looted comes back that way, from its frozen
/// record and not from the seed: the carved rock with its marks, field,
/// radius and collider; the ship with only its surviving section at the
/// health it froze with, its pinned hull, its hold and its balance, standing
/// down with its only weapon gone; and the ship the observer was holding,
/// still held. Nothing the generator places spawns beside them.
#[test]
fn a_mined_fought_and_looted_cell_returns_as_it_was_left() {
    let mut app = ship_app();
    let (rock, fought) = wreck_home(&mut app);
    let home = SectorCoord::ORIGIN;

    let world = app.world_mut();
    let node = rock_node(world, rock);
    let marks = world.get::<DamageMarks>(node).cloned().unwrap().0;
    let body_radius = world.get::<BodyRadius>(rock).map(|radius| radius.0);
    let collider_aabb = world.get::<ColliderAabb>(node).copied();
    let fore_health = world
        .query_filtered::<(&EntityId, &Health, &ChildOf), With<SectionMarker>>()
        .iter(world)
        .find(|(id, _, child_of)| id.0 == "fore" && child_of.parent() == fought)
        .map(|(_, health, _)| (health.current, health.max))
        .expect("fore survives");
    let root_health = world
        .get::<Health>(fought)
        .map(|health| (health.current, health.max))
        .expect("the root pins its hull");
    let hold = world.get::<ShipInventory>(fought).cloned().unwrap();

    walk_to::<Answers>(world, home.offset(5, 0, 0));
    assert!(world.get_entity(rock).is_err() && world.get_entity(fought).is_err());
    let record = world
        .resource::<FrozenSectors>()
        .get(home)
        .expect("home froze");
    assert!(record.is_visited());
    assert!(
        record
            .bodies()
            .iter()
            .any(|body| matches!(body.body(), FrozenBodyType::PendingShip { .. })),
        "the held ship froze held"
    );

    walk_to::<Answers>(world, home);
    let root = materialize::<Answers>(world, home, far());
    settle(&mut app);

    let world = app.world_mut();
    let [rock] = body_with_id(world, &sector_id(home, "body", 0))[..] else {
        panic!("the rock must come back once");
    };
    let [fought] = body_with_id(world, &sector_id(home, "ship", 0))[..] else {
        panic!("the fought ship must come back once");
    };
    assert_eq!(world.get::<ChildOf>(rock).map(ChildOf::parent), Some(root));
    let node = rock_node(world, rock);
    assert_eq!(
        world.get::<DamageMarks>(node).unwrap().0,
        marks,
        "the crater is kept"
    );
    assert!(
        world.get::<AsteroidField>(node).is_some(),
        "the carved field is kept"
    );
    assert_eq!(
        world.get::<BodyRadius>(rock).map(|radius| radius.0),
        body_radius
    );
    assert_eq!(
        world.get::<ColliderAabb>(node).copied(),
        collider_aabb,
        "the carved collider is kept"
    );
    assert_eq!(
        section_ids(world, fought),
        vec!["fore".to_string()],
        "the destroyed aft turret stays destroyed"
    );
    assert!(
        world.entity(fought).contains::<AINonCombatant>(),
        "a crewed ship whose only weapon was destroyed thaws unable to fight"
    );
    let thawed_fore = world
        .query_filtered::<(&EntityId, &Health, &ChildOf), With<SectionMarker>>()
        .iter(world)
        .find(|(id, _, child_of)| id.0 == "fore" && child_of.parent() == fought)
        .map(|(_, health, _)| (health.current, health.max));
    assert_eq!(thawed_fore, Some(fore_health));
    assert_eq!(
        world
            .get::<Health>(fought)
            .map(|health| (health.current, health.max)),
        Some(root_health),
        "the pinned hull is kept"
    );
    assert_eq!(world.get::<ShipInventory>(fought), Some(&hold));
    assert_eq!(
        world.get::<ShipCredits>(fought).copied(),
        Some(ShipCredits(0))
    );
    let held: Vec<String> = world
        .query::<&PendingSectorShip>()
        .iter(world)
        .map(|held| held.ship().id.to_string())
        .collect();
    assert_eq!(held, vec![sector_id(home, "ship", 1).to_string()]);
    assert_eq!(
        world
            .query_filtered::<(), With<AsteroidMarker>>()
            .iter(world)
            .count(),
        1,
        "the generator's rock does not spawn again beside the frozen one"
    );
}

/// A cell's one generated station, at rest beside a rock, and one the
/// observer is standing on at cell quarter.
fn station_home(input: SectorGenerationInput) -> SectorManifest {
    let centre = input.coord.centre(input.geometry.sector_edge);
    let quarter = input.geometry.sector_edge.get() * 0.25;
    let rock = SectorAsteroid {
        id: sector_id(input.coord, "body", 0),
        position: centre,
        config: rock_config(Meters(40.0), 7),
    };
    let standing = station(
        sector_id(input.coord, "cluster_a_station", 0),
        centre + Meters3::new(quarter, 0.0, 0.0),
        TEST_HULL_SECTION_ID,
    );
    let held = station(
        sector_id(input.coord, "cluster_b_station", 0),
        centre + Meters3::new(0.0, 0.0, quarter),
        TEST_HULL_SECTION_ID,
    );
    let mut manifest = empty(input.coord, vec![rock]);
    manifest.stations = vec![standing, held];
    manifest
}

/// A generated station leaves and returns with its cell like a generated
/// ship: it spawns from the manifest with its typed id and no address, it
/// freezes as a station record under that id when its cell retires, and it
/// thaws from the record with the same id, pose, services and storage, with
/// the generator's copy not spawned beside it. A station the observer stands
/// on freezes as a held station and comes back held.
#[test]
fn a_generated_station_retires_and_thaws_with_its_cell() {
    let mut app = ship_app();
    let home = SectorCoord::ORIGIN;
    let config = answering(station_home);
    let edge = config.sector_edge;
    let world = app.world_mut();
    arm(world, config, home);
    let observer = ObserverBody {
        position: home.centre(edge) + Meters3::new(0.0, 0.0, edge.get() * 0.25),
        reach: Meters(100.0),
    };
    materialize::<Answers>(world, home, observer);
    settle(&mut app);

    let world = app.world_mut();
    let standing_id = sector_id(home, "cluster_a_station", 0);
    let [standing] = body_with_id(world, &standing_id)[..] else {
        panic!("the standing station spawns once");
    };
    let root = world.entity(standing);
    assert!(root.contains::<StationMarker>(), "spawned as a station");
    assert!(
        root.contains::<SpaceshipRootMarker>(),
        "a station root is a hull root"
    );
    assert!(
        !root.contains::<ScenarioAddressableMarker>(),
        "a generated station is not addressable by scenario scripts"
    );
    assert!(
        !root.contains::<DerelictShipMarker>(),
        "a living station spawns intact"
    );
    let transform = *root.get::<Transform>().expect("a station has a pose");
    let services = root.get::<StationServices>().cloned().expect("services");
    let builds = root.get::<StationBuilds>().cloned().expect("builds");
    assert_eq!(
        builds,
        StationBuilds(vec![TEST_HULL_SECTION_ID.to_string()]),
        "a generated station builds what its manifest design lists"
    );
    let storage = root.get::<StationStorage>().cloned().expect("storage");
    let held: Vec<String> = world
        .query::<&PendingSectorStation>()
        .iter(world)
        .map(|held| held.station().id.to_string())
        .collect();
    assert_eq!(
        held,
        vec![sector_id(home, "cluster_b_station", 0).to_string()],
        "the station under the observer is held"
    );

    walk_to::<Answers>(world, home.offset(5, 0, 0));
    assert!(
        world.get_entity(standing).is_err(),
        "the cell took the station"
    );
    let record = world
        .resource::<FrozenSectors>()
        .get(home)
        .expect("home froze");
    assert!(record.is_visited());
    let kinds: Vec<&str> = record
        .bodies()
        .iter()
        .map(|body| match body.body() {
            FrozenBodyType::Station(..) => "station",
            FrozenBodyType::PendingStation { .. } => "pending station",
            FrozenBodyType::Asteroid(_) => "rock",
            _ => "other",
        })
        .collect();
    assert_eq!(
        kinds.iter().filter(|kind| **kind == "station").count(),
        1,
        "the standing station froze as a station: {kinds:?}"
    );
    assert_eq!(
        kinds
            .iter()
            .filter(|kind| **kind == "pending station")
            .count(),
        1,
        "the held station froze held: {kinds:?}"
    );
    assert!(
        frozen_ids(world, home).contains(&standing_id.to_string()),
        "the record keeps the generated id"
    );

    walk_to::<Answers>(world, home);
    let root = materialize::<Answers>(world, home, observer);
    settle(&mut app);

    let world = app.world_mut();
    let [thawed] = body_with_id(world, &standing_id)[..] else {
        panic!("the standing station comes back once");
    };
    assert_eq!(
        world.get::<ChildOf>(thawed).map(ChildOf::parent),
        Some(root)
    );
    let thawed_root = world.entity(thawed);
    assert!(
        thawed_root.contains::<StationMarker>(),
        "thawed as a station"
    );
    assert!(
        !thawed_root.contains::<ScenarioAddressableMarker>(),
        "a thawed generated station stays unaddressable"
    );
    assert_eq!(
        thawed_root.get::<StationServices>().cloned(),
        Some(services)
    );
    assert_eq!(
        thawed_root.get::<StationBuilds>().cloned(),
        Some(builds),
        "a thawed station builds what it froze with"
    );
    assert_eq!(thawed_root.get::<StationStorage>().cloned(), Some(storage));
    let thawed_transform = *thawed_root.get::<Transform>().unwrap();
    assert!(
        thawed_transform
            .translation
            .abs_diff_eq(transform.translation, 1.0e-3)
            && thawed_transform
                .rotation
                .abs_diff_eq(transform.rotation, 1.0e-4),
        "the pose is kept: {thawed_transform:?} vs {transform:?}"
    );
    assert_eq!(
        world
            .query_filtered::<(), With<StationMarker>>()
            .iter(world)
            .count(),
        1,
        "the generator's station does not spawn again beside the frozen one"
    );
    let held: Vec<String> = world
        .query::<&PendingSectorStation>()
        .iter(world)
        .map(|held| held.station().id.to_string())
        .collect();
    assert_eq!(
        held,
        vec![sector_id(home, "cluster_b_station", 0).to_string()],
        "the held station comes back held"
    );
}

/// A saved world reads back as the world it was saved from. The ledger and
/// every live cell - a carved rock, a fought and looted ship, a held ship -
/// and a canister waiting top-level in a cell that is not live, write to
/// text, restore into a fresh session, write the same text again, and
/// materialize with the carved collider rebuilt from the rock's field.
#[cfg(feature = "serde")]
#[test]
fn a_snapshot_of_the_live_world_reads_back_as_the_same_world() {
    let mut app = ship_app();
    let (rock, fought) = wreck_home(&mut app);
    let home = SectorCoord::ORIGIN;
    let world = app.world_mut();
    let edge = world.resource::<WorldConfig<Answers>>().sector_edge;
    let waiting_in = home.offset(1, 0, 0);
    let id = world
        .resource_mut::<nova_gameplay::inventory::CargoCanisterIdAllocator>()
        .mint();
    let contents = CargoCanister::new(&test_items(), &ItemRoleType::IronOre.into(), 4);
    world.spawn((
        cargo_canister(
            contents.clone(),
            Transform::from_translation(waiting_in.centre(edge).to_engine()),
            Vec3::ZERO,
            AssetRef::from("canister.glb#Scene0"),
        ),
        id,
    ));
    let node = rock_node(world, rock);
    let marks = world.get::<DamageMarks>(node).cloned().unwrap().0;
    let collider_aabb = world.get::<ColliderAabb>(node).copied();
    let hold = world.get::<ShipInventory>(fought).cloned().unwrap();

    let saved = crate::snapshot_sectors::<Answers>(world).expect("every body is settled");
    assert!(
        world.get_entity(rock).is_ok() && world.get_entity(fought).is_ok(),
        "a snapshot despawns nothing"
    );
    let text = ron::to_string(&saved).expect("the ledger writes");
    let read: FrozenSectors = ron::from_str(&text).expect("the ledger reads back");

    let mut fresh = ship_app();
    fresh
        .world_mut()
        .resource_mut::<GameSections>()
        .0
        .push(SectionConfig {
            base: BaseSectionConfig {
                id: TEST_TURRET_SECTION_ID.to_string(),
                advancement: 0.0,
                draw_weight: 1.0,
                ..default()
            },
            kind: SectionKind::Turret(TurretSectionConfig::default()),
        });
    arm(fresh.world_mut(), answering(wreckable), home);
    let world = fresh.world_mut();
    world.resource_mut::<FrozenSectors>().restore(read);
    assert_eq!(
        ron::to_string(world.resource::<FrozenSectors>()).unwrap(),
        text,
        "the restored ledger writes the same text"
    );
    assert!(world
        .resource::<FrozenSectors>()
        .get(waiting_in)
        .expect("the waiting canister arrived in its cell")
        .bodies()
        .iter()
        .any(|body| matches!(body.body(), FrozenBodyType::Canister(_))));

    materialize::<Answers>(world, home, far());
    settle(&mut fresh);
    let world = fresh.world_mut();
    let [rock] = body_with_id(world, &sector_id(home, "body", 0))[..] else {
        panic!("the rock must come back once");
    };
    let [fought] = body_with_id(world, &sector_id(home, "ship", 0))[..] else {
        panic!("the fought ship must come back once");
    };
    let node = rock_node(world, rock);
    assert_eq!(world.get::<DamageMarks>(node).unwrap().0, marks);
    assert_eq!(
        world.get::<ColliderAabb>(node).copied(),
        collider_aabb,
        "the carved collider is rebuilt from the saved field"
    );
    assert_eq!(section_ids(world, fought), vec!["fore".to_string()]);
    assert_eq!(world.get::<ShipInventory>(fought), Some(&hold));
    assert_eq!(
        world.get::<ShipCredits>(fought).copied(),
        Some(ShipCredits(0))
    );
    let held: Vec<String> = world
        .query::<&PendingSectorShip>()
        .iter(world)
        .map(|held| held.ship().id.to_string())
        .collect();
    assert_eq!(held, vec![sector_id(home, "ship", 1).to_string()]);
}

/// Saving with the player docked keeps the live dock: the partner freezes
/// into home's record as a plain ship at its live pose, the connection and
/// both `DockedShip` stay on the live entities, and the record round-trips
/// through RON. The player's own dock is saved separately, outside any
/// cell's record, the same way `freeze_ship` already does for it.
#[cfg(feature = "serde")]
#[test]
fn a_save_copies_the_observer_s_partner_and_leaves_their_dock_live() {
    let mut app = ship_app();
    load_docking_section(&mut app);

    let home = SectorCoord::ORIGIN;
    let edge = Meters(320.0);
    arm(
        app.world_mut(),
        WorldConfig {
            seed: 1,
            sector_edge: edge,
            active_radius: 1,
            generator: Nothing,
        },
        home,
    );
    let home_root = app
        .world_mut()
        .spawn((SectorRoot(home), Transform::default()))
        .id();
    let centre = home.centre(edge).to_engine();
    let player = docking_hull(&mut app, centre, Quat::IDENTITY, None);
    app.world_mut().entity_mut(player).insert((
        WorldObserver,
        PlayerSpaceshipMarker,
        FlightIntent::default(),
    ));
    let partner = docking_hull(
        &mut app,
        centre + Vec3::NEG_Z * 1.5,
        Quat::from_rotation_y(std::f32::consts::PI),
        Some(home_root),
    );
    settle(&mut app);
    app.world_mut().trigger(DockingConnectionRequest {
        entity: player,
        target: partner,
    });
    settle(&mut app);

    let world = app.world_mut();
    assert_eq!(
        world.query::<&DockingConnection>().iter(world).count(),
        1,
        "the pair docks"
    );
    let partner_pose = world
        .get::<Position>(partner)
        .copied()
        .expect("the partner has a pose");

    // `freeze_ship` needs an authored design, a controller and capabilities on
    // the root, and a `Health` and `SectionAnimations` on each section; a bare
    // `docking_hull` carries none of them (`DamageMarks`, `ShipInventory` and
    // `ShipCredits` already arrive as `SpaceshipRootMarker` requirements).
    for (ship, id) in [(player, "player"), (partner, "partner")] {
        let fore = fore_section(world, ship);
        world.entity_mut(ship).insert((
            EntityId::new(id),
            docking_hull_design(),
            SpaceshipController::default(),
            ShipCapabilities::default(),
        ));
        world
            .entity_mut(fore)
            .insert((Health::new(10.0), SectionAnimations::new(Vec::new())));
    }

    // The session saves the player this same way, outside any cell's record.
    freeze_ship(world, player).expect("a docked player still freezes on its own");

    let saved = crate::snapshot_sectors::<Nothing>(world).expect("the live dock is kept");

    assert!(
        world.get_entity(player).is_ok() && world.get_entity(partner).is_ok(),
        "a snapshot despawns nothing"
    );
    assert!(world.get::<DockedShip>(player).is_some());
    assert!(world.get::<DockedShip>(partner).is_some());
    assert_eq!(
        world.query::<&DockingConnection>().iter(world).count(),
        1,
        "the live dock survives the snapshot"
    );

    let record = saved.get(home).expect("home froze");
    let partner_body = record
        .bodies()
        .iter()
        .find(|body| body.id() == Some("partner"))
        .expect("the partner froze into home's record");
    assert!(
        matches!(partner_body.body(), FrozenBodyType::Ship(_)),
        "the partner froze as a ship, not a dock"
    );
    assert_eq!(
        partner_body.transform().translation,
        partner_pose.0,
        "the partner froze at its live pose"
    );

    let text = ron::to_string(&saved).expect("the ledger writes");
    let read: FrozenSectors = ron::from_str(&text).expect("the ledger reads back");
    assert_eq!(
        ron::to_string(&read).expect("the restored ledger writes"),
        text,
        "the record round-trips through RON"
    );
}

/// A dock where neither hull is the observer, and that is not a station's
/// parked hull, cannot be kept: `snapshot_sectors` refuses the whole save,
/// naming both docked ships, rather than drop the dock silently. The live
/// dock is left exactly as it stood.
#[cfg(feature = "serde")]
#[test]
fn a_save_refuses_a_dock_the_observer_is_not_in() {
    let mut app = ship_app();
    load_docking_section(&mut app);

    let home = SectorCoord::ORIGIN;
    let edge = Meters(320.0);
    arm(
        app.world_mut(),
        WorldConfig {
            seed: 1,
            sector_edge: edge,
            active_radius: 1,
            generator: Nothing,
        },
        home,
    );
    let home_root = app
        .world_mut()
        .spawn((SectorRoot(home), Transform::default()))
        .id();
    app.world_mut().spawn(WorldObserver);
    let centre = home.centre(edge).to_engine();
    let first = docking_hull(&mut app, centre, Quat::IDENTITY, Some(home_root));
    let second = docking_hull(
        &mut app,
        centre + Vec3::NEG_Z * 1.5,
        Quat::from_rotation_y(std::f32::consts::PI),
        Some(home_root),
    );
    settle(&mut app);
    app.world_mut().trigger(DockingConnectionRequest {
        entity: first,
        target: second,
    });
    settle(&mut app);

    let world = app.world_mut();
    world
        .entity_mut(first)
        .insert(EntityId::new("first".to_string()));
    world
        .entity_mut(second)
        .insert(EntityId::new("second".to_string()));
    assert_eq!(
        world.query::<&DockingConnection>().iter(world).count(),
        1,
        "the pair docks"
    );

    let error = crate::snapshot_sectors::<Nothing>(world)
        .expect_err("neither hull is the observer, nor a station's parked hull");
    let SectorSnapshotError::UnsavedDock {
        first: named_first,
        second: named_second,
    } = error
    else {
        panic!("expected UnsavedDock, got {error}");
    };
    let mut named = [named_first, named_second];
    named.sort();
    assert_eq!(
        named,
        ["first".to_string(), "second".to_string()],
        "the error names both docked ships"
    );
    assert_eq!(
        world.query::<&DockingConnection>().iter(world).count(),
        1,
        "the live dock is untouched"
    );
}

/// A cell's one generated station with a live docking port, and one
/// generated visitor ship 15 m off its stern, rotated to face it: the same
/// face-gap geometry the bare `docking_hull` pair captures at (1.5 engine
/// units), scaled into meters at this crate's own engine boundary. Both
/// clearances are shrunk to 5 m so the two bodies' spheres (sum 10 m) stay
/// clear of each other at the 15 m root separation `validate` demands
/// (margin 0); the docking envelope never looks at clearance, it measures
/// the port faces.
fn station_dock_home(input: SectorGenerationInput) -> SectorManifest {
    let centre = input.coord.centre(input.geometry.sector_edge);
    let mut manifest = empty(input.coord, Vec::new());
    let mut dock = station(
        sector_id(input.coord, "station", 0),
        centre,
        TEST_DOCKING_SECTION_ID,
    );
    dock.clearance = Meters(5.0);
    manifest.stations.push(dock);
    let mut visitor = ship(
        sector_id(input.coord, "ship", 0),
        centre + Meters3::new(0.0, 0.0, -15.0),
        TEST_DOCKING_SECTION_ID,
    );
    visitor.clearance = Meters(5.0);
    visitor.rotation = Quat::from_rotation_y(std::f32::consts::PI);
    manifest.ships.push(visitor);
    manifest
}

/// Arms a home cell with [`station_dock_home`], materializes it with the
/// observer clear of both bodies, docks the visitor to the station through
/// the real [`DockingConnectionRequest`] path, and settles. Returns the app,
/// the visitor and station entities, and the observer used to materialize.
fn build_station_dock() -> (App, Entity, Entity, ObserverBody) {
    dock_visitor_at_station(ship_app(), station_dock_home)
}

/// [`build_station_dock`] in `app`, with the cell `home_of` generates.
fn dock_visitor_at_station(
    mut app: App,
    home_of: fn(SectorGenerationInput) -> SectorManifest,
) -> (App, Entity, Entity, ObserverBody) {
    load_docking_section(&mut app);

    let home = SectorCoord::ORIGIN;
    let config = answering(home_of);
    let edge = config.sector_edge;
    let world = app.world_mut();
    arm(world, config, home);
    let observer = ObserverBody {
        position: home.centre(edge) + Meters3::new(0.0, 0.0, edge.get() * 0.25),
        reach: Meters(100.0),
    };
    materialize::<Answers>(world, home, observer);
    settle(&mut app);

    let world = app.world_mut();
    let [visitor] = body_with_id(world, &sector_id(home, "ship", 0))[..] else {
        panic!("the visitor ship must spawn once");
    };
    let [station] = body_with_id(world, &sector_id(home, "station", 0))[..] else {
        panic!("the station must spawn once");
    };
    world.trigger(DockingConnectionRequest {
        entity: visitor,
        target: station,
    });
    settle(&mut app);

    let world = app.world_mut();
    assert_eq!(
        world.query::<&DockingConnection>().iter(world).count(),
        1,
        "the visitor must dock to the station"
    );
    assert!(
        world.get::<DockedShip>(visitor).is_some(),
        "the visitor carries the dock"
    );

    (app, visitor, station, observer)
}

/// The dock between a ship and a station, read live: the connection's order
/// and both ports' ids, the fixed joint's local frames, the ship's pose
/// relative to the station's, and both linear velocities.
struct DockSnapshot {
    station_first: bool,
    ship_port: String,
    station_port: String,
    ship_frame: SavedJointFrame,
    station_frame: SavedJointFrame,
    relative_position: Vec3,
    relative_rotation: Quat,
    ship_velocity: Vec3,
    station_velocity: Vec3,
}

/// Reads the one live dock between `ship` and `station` into a
/// [`DockSnapshot`].
///
/// # Panics
///
/// When the pair carries no connection, or either hull has no pose.
fn dock_snapshot(world: &mut World, ship: Entity, station: Entity) -> DockSnapshot {
    let connection_entity = world
        .query::<(Entity, &DockingConnection)>()
        .iter(world)
        .find(|(_, connection)| {
            (connection.first_ship, connection.second_ship) == (ship, station)
                || (connection.first_ship, connection.second_ship) == (station, ship)
        })
        .map(|(entity, _)| entity)
        .expect("the pair has a connection");
    let connection = *world
        .get::<DockingConnection>(connection_entity)
        .expect("the found connection is live");
    let joint = world
        .get::<FixedJoint>(connection_entity)
        .expect("a docking connection holds its fixed joint");
    let station_first = connection.first_ship == station;
    let (station_section, ship_section, station_frame_raw, ship_frame_raw) = if station_first {
        (
            connection.first_section,
            connection.second_section,
            &joint.frame1,
            &joint.frame2,
        )
    } else {
        (
            connection.second_section,
            connection.first_section,
            &joint.frame2,
            &joint.frame1,
        )
    };
    let port_id = |section: Entity| {
        world
            .get::<EntityId>(section)
            .expect("a docking port has an id")
            .0
            .clone()
    };
    let ship_port = port_id(ship_section);
    let station_port = port_id(station_section);
    let ship_frame = SavedJointFrame::of(ship_frame_raw);
    let station_frame = SavedJointFrame::of(station_frame_raw);
    let ship_position = world.get::<Position>(ship).expect("the ship has a pose").0;
    let ship_rotation = world
        .get::<Rotation>(ship)
        .expect("the ship has a rotation")
        .0;
    let station_position = world
        .get::<Position>(station)
        .expect("the station has a pose")
        .0;
    let station_rotation = world
        .get::<Rotation>(station)
        .expect("the station has a rotation")
        .0;
    DockSnapshot {
        station_first,
        ship_port,
        station_port,
        ship_frame,
        station_frame,
        relative_position: station_rotation.inverse() * (ship_position - station_position),
        relative_rotation: station_rotation.inverse() * ship_rotation,
        ship_velocity: world
            .get::<LinearVelocity>(ship)
            .map_or(Vec3::ZERO, |velocity| velocity.0),
        station_velocity: world
            .get::<LinearVelocity>(station)
            .map_or(Vec3::ZERO, |velocity| velocity.0),
    }
}

/// Whether `got` is `expected`'s rotation: equal, or equal up to the sign
/// `restore_docking_connection`'s quaternion-cover negation may flip
/// (`crates/nova_ship/src/sections/docking_section/connection.rs:575`).
fn basis_matches(got: Quat, expected: Quat) -> bool {
    got.abs_diff_eq(expected, 1.0e-4) || got.abs_diff_eq(-expected, 1.0e-4)
}

/// Asserts the live dock between `ship` and `station` still matches
/// `expected`, identifying the failure as `context`.
fn assert_dock_matches(
    world: &mut World,
    ship: Entity,
    station: Entity,
    expected: &DockSnapshot,
    context: &str,
) {
    assert_dock_pose_matches(world, ship, station, expected, context);
    let got = dock_snapshot(world, ship, station);
    assert!(
        got.ship_velocity
            .abs_diff_eq(expected.ship_velocity, 1.0e-3),
        "{context}: the ship's linear velocity changed: {:?} vs {:?}",
        got.ship_velocity,
        expected.ship_velocity
    );
    assert!(
        got.station_velocity
            .abs_diff_eq(expected.station_velocity, 1.0e-3),
        "{context}: the station's linear velocity changed: {:?} vs {:?}",
        got.station_velocity,
        expected.station_velocity
    );
}

/// [`assert_dock_matches`] without the velocities: the connection's order and
/// ports, the joint frames and the station-relative pose.
fn assert_dock_pose_matches(
    world: &mut World,
    ship: Entity,
    station: Entity,
    expected: &DockSnapshot,
    context: &str,
) {
    let got = dock_snapshot(world, ship, station);
    assert_eq!(
        got.station_first, expected.station_first,
        "{context}: the connection's order flipped"
    );
    assert_eq!(
        got.ship_port, expected.ship_port,
        "{context}: the ship's port id changed"
    );
    assert_eq!(
        got.station_port, expected.station_port,
        "{context}: the station's port id changed"
    );
    assert_eq!(
        got.ship_frame.anchor, expected.ship_frame.anchor,
        "{context}: the ship's joint anchor moved"
    );
    assert_eq!(
        got.station_frame.anchor, expected.station_frame.anchor,
        "{context}: the station's joint anchor moved"
    );
    assert!(
        basis_matches(got.ship_frame.basis, expected.ship_frame.basis),
        "{context}: the ship's joint basis is neither the saved one nor its cover negation: \
         {:?} vs {:?}",
        got.ship_frame.basis,
        expected.ship_frame.basis
    );
    assert!(
        basis_matches(got.station_frame.basis, expected.station_frame.basis),
        "{context}: the station's joint basis is neither the saved one nor its cover negation: \
         {:?} vs {:?}",
        got.station_frame.basis,
        expected.station_frame.basis
    );
    assert!(
        got.relative_position
            .abs_diff_eq(expected.relative_position, 1.0e-3),
        "{context}: the station-relative position moved: {:?} vs {:?}",
        got.relative_position,
        expected.relative_position
    );
    assert!(
        basis_matches(got.relative_rotation, expected.relative_rotation),
        "{context}: the station-relative rotation moved: {:?} vs {:?}",
        got.relative_rotation,
        expected.relative_rotation
    );
}

/// `ship`'s balance, market and hold, read live.
fn ship_cargo(
    world: &mut World,
    ship: Entity,
) -> (ShipCredits, Option<TraderMarket>, ShipInventory) {
    (
        *world
            .get::<ShipCredits>(ship)
            .expect("a ship root has credits"),
        world.get::<TraderMarket>(ship).cloned(),
        world
            .get::<ShipInventory>(ship)
            .cloned()
            .expect("a ship root has a hold"),
    )
}

/// `text`, a [`FrozenShip`](nova_scenario::prelude::FrozenShip)'s RON, with
/// its RCS `idle` field blanked: idle time keeps advancing while the dock
/// sits still, so a snapshot taken after a different number of settle ticks
/// never matches bit-for-bit on this one field alone.
#[cfg(feature = "serde")]
fn normalize_rcs_idle(text: &str) -> String {
    let Some(start) = text.find("idle:") else {
        return text.to_string();
    };
    let value_start = start + "idle:".len();
    let value_end = text[value_start..]
        .find([',', ')'])
        .map_or(text.len(), |offset| value_start + offset);
    format!("{}idle:_{}", &text[..start], &text[value_end..])
}

/// A ship docked at a station through the real request freezes inside the
/// station's record when home retires, and thaws docked again when home
/// comes back: the same connection order and both ports, the same joint
/// frames up to `restore_docking_connection`'s quaternion-cover negation, the
/// same station-relative pose right after thaw and after settling more, no
/// leftover [`UndockSeparation`], and the same controller, credits, market
/// and hold.
#[test]
fn a_ship_docked_at_a_station_freezes_and_thaws_docked() {
    let (mut app, visitor, station, observer) = build_station_dock();
    let home = SectorCoord::ORIGIN;
    let visitor_id = sector_id(home, "ship", 0);
    let station_id = sector_id(home, "station", 0);

    let world = app.world_mut();
    let before_dock = dock_snapshot(world, visitor, station);
    let before_cargo = ship_cargo(world, visitor);
    #[cfg(feature = "serde")]
    let before_ship = freeze_ship(world, visitor).expect("the docked visitor is settled");

    walk_to::<Answers>(world, home.offset(5, 0, 0));
    assert!(
        body_with_id(world, &visitor_id).is_empty(),
        "the visitor despawns with the station it is docked to"
    );
    let record = world
        .resource::<FrozenSectors>()
        .get(home)
        .expect("home froze");
    let stations: Vec<&FrozenBody> = record
        .bodies()
        .iter()
        .filter(|body| matches!(body.body(), FrozenBodyType::Station(..)))
        .collect();
    let [station_body] = stations[..] else {
        panic!("home must keep exactly one station body: {stations:?}");
    };
    assert_eq!(
        station_body
            .docked()
            .iter()
            .map(|docked| docked.body().id())
            .collect::<Vec<_>>(),
        vec![Some(visitor_id.to_string().as_str())],
        "the station's record keeps exactly the visitor docked"
    );
    assert!(
        !record
            .bodies()
            .iter()
            .any(|body| body.id() == Some(visitor_id.to_string().as_str())),
        "the visitor is not also a top-level body in the record"
    );

    walk_to::<Answers>(world, home);
    materialize::<Answers>(world, home, observer);

    let world = app.world_mut();
    let [thawed_visitor] = body_with_id(world, &visitor_id)[..] else {
        panic!("the visitor must come back once");
    };
    let [thawed_station] = body_with_id(world, &station_id)[..] else {
        panic!("the station must come back once");
    };
    assert!(
        world.get::<DockedShip>(thawed_visitor).is_some(),
        "the thawed visitor carries the dock"
    );
    assert_eq!(
        world.query::<&DockingConnection>().iter(world).count(),
        1,
        "exactly one connection thaws"
    );
    assert_eq!(
        world.query::<&UndockSeparation>().iter(world).count(),
        0,
        "a thaw never leaves an undock separation behind"
    );
    assert_dock_matches(
        world,
        thawed_visitor,
        thawed_station,
        &before_dock,
        "right after thaw",
    );

    settle(&mut app);
    let world = app.world_mut();
    assert_dock_matches(
        world,
        thawed_visitor,
        thawed_station,
        &before_dock,
        "after settling more",
    );

    let after_cargo = ship_cargo(world, thawed_visitor);
    assert_eq!(
        after_cargo, before_cargo,
        "the balance, market and hold thaw unchanged"
    );
    #[cfg(feature = "serde")]
    {
        let after_ship = freeze_ship(world, thawed_visitor).expect("the thawed visitor is settled");
        let before_text =
            normalize_rcs_idle(&ron::to_string(&before_ship).expect("a ship freezes to RON"));
        let after_text =
            normalize_rcs_idle(&ron::to_string(&after_ship).expect("a thawed ship freezes to RON"));
        assert_eq!(
            before_text, after_text,
            "the ship's frozen record, including its controller, is unchanged but for RCS idle \
             catch-up"
        );
    }
}

/// Radius, in engine units, of the orbit the orbiting-station proof stands on:
/// the radius of a live seed whose visitor jumped when its dock thawed.
const ORBIT_RADIUS: f32 = 147.0;

/// Period, in seconds, of that orbit.
const ORBIT_PERIOD_S: f32 = 72.0;

/// The gravitational parameter that gives [`ORBIT_RADIUS`] a circular orbit of
/// [`ORBIT_PERIOD_S`]: `mu = omega^2 * r^3`.
fn orbit_mu() -> f32 {
    let omega = std::f32::consts::TAU / ORBIT_PERIOD_S;
    omega * omega * ORBIT_RADIUS.powi(3)
}

/// Where the orbited well stands for a station at `centre`: [`ORBIT_RADIUS`]
/// along +X, engine units.
fn orbit_well_position(centre: Meters3) -> Vec3 {
    centre.to_engine() + Vec3::X * ORBIT_RADIUS
}

/// [`station_dock_home`] with the station and its visitor both riding the
/// circular orbit of the well at [`orbit_well_position`], at the speed the
/// gravity layer integrates.
fn orbiting_station_dock_home(input: SectorGenerationInput) -> SectorManifest {
    let centre = input.coord.centre(input.geometry.sector_edge);
    let mut manifest = station_dock_home(input);
    let velocity =
        station_orbit_velocity(orbit_mu(), orbit_well_position(centre), centre.to_engine())
            .expect("the station stands off the well's centre");
    manifest.stations[0].spawn.initial_velocity = velocity;
    manifest.ships[0].spawn.initial_velocity = velocity;
    manifest
}

/// [`build_station_dock`] in orbit: the gravity layer runs, a static well
/// stands at [`orbit_well_position`] with [`orbit_mu`], and the station and
/// its docked visitor ride a circular orbit about it.
fn build_orbiting_station_dock() -> (App, Entity, Entity, ObserverBody) {
    let mut app = unfinished_ship_app();
    app.add_plugins(NovaGravityPlugin);
    app.finish();
    let centre = SectorCoord::ORIGIN.centre(answering(orbiting_station_dock_home).sector_edge);
    let well = orbit_well_position(centre);
    app.world_mut().spawn((
        RigidBody::Static,
        Transform::from_translation(well),
        Position(well),
        GravityWell::from_mass(orbit_mu(), 85.0, &GravitySettings::default()),
    ));
    dock_visitor_at_station(app, orbiting_station_dock_home)
}

/// `entity`'s absolute position, linear velocity and angular velocity.
fn absolute_motion(world: &World, entity: Entity) -> (Vec3, Vec3, Vec3) {
    (
        world.get::<Position>(entity).expect("a pose").0,
        world.get::<LinearVelocity>(entity).expect("a velocity").0,
        world.get::<AngularVelocity>(entity).expect("a spin").0,
    )
}

/// A visitor docked at a station that rides a circular orbit thaws with the
/// motion it froze with: right after the thaw both hulls hold their absolute
/// pose, linear velocity and spin, and after more frames of gravity they
/// match a pair that never froze, with the dock's frames and the
/// station-relative pose unchanged.
#[test]
fn a_visitor_docked_at_an_orbiting_station_thaws_with_its_motion() {
    let (mut app, visitor, station, observer) = build_orbiting_station_dock();
    let (mut control, control_visitor, control_station, _) = build_orbiting_station_dock();
    let home = SectorCoord::ORIGIN;
    let visitor_id = sector_id(home, "ship", 0);
    let station_id = sector_id(home, "station", 0);

    let world = app.world_mut();
    let before_dock = dock_snapshot(world, visitor, station);
    let before_station = absolute_motion(world, station);
    let before_visitor = absolute_motion(world, visitor);
    let circular = circular_orbit_speed(orbit_mu(), ORBIT_RADIUS);
    assert!(
        (before_station.1.length() - circular).abs() < 0.05 * circular && circular > 10.0,
        "the station rides the circular orbit, not rest: speed {} vs circular {circular}",
        before_station.1.length()
    );

    walk_to::<Answers>(world, home.offset(5, 0, 0));
    assert!(
        body_with_id(world, &visitor_id).is_empty(),
        "the visitor freezes with the station it is docked to"
    );
    walk_to::<Answers>(world, home);
    materialize::<Answers>(world, home, observer);

    let world = app.world_mut();
    let [thawed_visitor] = body_with_id(world, &visitor_id)[..] else {
        panic!("the visitor must come back once");
    };
    let [thawed_station] = body_with_id(world, &station_id)[..] else {
        panic!("the station must come back once");
    };
    assert!(
        world.get::<DockedShip>(thawed_visitor).is_some(),
        "the thawed visitor carries the dock"
    );
    assert_eq!(
        world.query::<&UndockSeparation>().iter(world).count(),
        0,
        "a thaw never leaves an undock separation behind"
    );
    assert_dock_matches(
        world,
        thawed_visitor,
        thawed_station,
        &before_dock,
        "right after thaw",
    );
    for (name, got, expected) in [
        (
            "station",
            absolute_motion(world, thawed_station),
            before_station,
        ),
        (
            "visitor",
            absolute_motion(world, thawed_visitor),
            before_visitor,
        ),
    ] {
        assert!(
            got.0.abs_diff_eq(expected.0, 1.0e-3),
            "right after thaw: the {name}'s position moved: {:?} vs {:?}",
            got.0,
            expected.0
        );
        assert!(
            got.1.abs_diff_eq(expected.1, 1.0e-3),
            "right after thaw: the {name}'s linear velocity changed: {:?} vs {:?}",
            got.1,
            expected.1
        );
        assert!(
            got.2.abs_diff_eq(expected.2, 1.0e-3),
            "right after thaw: the {name}'s spin changed: {:?} vs {:?}",
            got.2,
            expected.2
        );
    }

    settle(&mut app);
    settle(&mut control);
    let world = app.world_mut();
    let after_dock = dock_snapshot(world, thawed_visitor, thawed_station);
    assert_dock_pose_matches(
        world,
        thawed_visitor,
        thawed_station,
        &before_dock,
        "after settling more",
    );
    assert!(
        world.get::<DockedShip>(thawed_visitor).is_some(),
        "the visitor is still docked after settling"
    );
    let control_world = control.world_mut();
    assert_dock_pose_matches(
        control_world,
        control_visitor,
        control_station,
        &after_dock,
        "against the pair that never froze",
    );
    let world = app.world_mut();
    for (name, thawed, never_froze) in [
        (
            "station",
            absolute_motion(world, thawed_station),
            absolute_motion(control.world(), control_station),
        ),
        (
            "visitor",
            absolute_motion(world, thawed_visitor),
            absolute_motion(control.world(), control_visitor),
        ),
    ] {
        assert!(
            thawed.1.abs_diff_eq(never_froze.1, 1.0e-3),
            "after settling: the {name}'s linear velocity differs from the pair that never \
             froze: {:?} vs {:?}",
            thawed.1,
            never_froze.1
        );
        assert!(
            thawed.2.abs_diff_eq(never_froze.2, 1.0e-3),
            "after settling: the {name}'s spin differs from the pair that never froze: \
             {:?} vs {:?}",
            thawed.2,
            never_froze.2
        );
    }
}

/// A saved station dock reads back and thaws docked: the live dock between a
/// generated station and a generated visitor is kept, not refused, the
/// ledger round-trips through RON, and restoring it into a fresh session
/// thaws docked with the same invariants a retire-and-thaw keeps.
#[cfg(feature = "serde")]
#[test]
fn a_saved_station_dock_reads_back_and_thaws_docked() {
    let (mut app, visitor, station, observer) = build_station_dock();
    let home = SectorCoord::ORIGIN;
    let visitor_id = sector_id(home, "ship", 0);
    let station_id = sector_id(home, "station", 0);

    let world = app.world_mut();
    let before_dock = dock_snapshot(world, visitor, station);

    let saved =
        crate::snapshot_sectors::<Answers>(world).expect("a station dock is kept, not refused");
    assert!(
        world.get_entity(visitor).is_ok() && world.get_entity(station).is_ok(),
        "a snapshot despawns nothing"
    );
    assert_eq!(
        world.query::<&DockingConnection>().iter(world).count(),
        1,
        "the live dock survives the snapshot"
    );

    let text = ron::to_string(&saved).expect("the ledger writes");
    let read: FrozenSectors = ron::from_str(&text).expect("the ledger reads back");
    assert_eq!(
        ron::to_string(&read).expect("the restored ledger writes"),
        text,
        "the restored ledger writes the same text"
    );

    let mut fresh = ship_app();
    load_docking_section(&mut fresh);
    arm(fresh.world_mut(), answering(station_dock_home), home);
    let world = fresh.world_mut();
    world.resource_mut::<FrozenSectors>().restore(read);
    materialize::<Answers>(world, home, observer);

    let world = fresh.world_mut();
    let [thawed_visitor] = body_with_id(world, &visitor_id)[..] else {
        panic!("the visitor must come back once");
    };
    let [thawed_station] = body_with_id(world, &station_id)[..] else {
        panic!("the station must come back once");
    };
    assert!(
        world.get::<DockedShip>(thawed_visitor).is_some(),
        "the thawed visitor carries the dock"
    );
    assert_eq!(
        world.query::<&DockingConnection>().iter(world).count(),
        1,
        "exactly one connection thaws"
    );
    assert_eq!(
        world.query::<&UndockSeparation>().iter(world).count(),
        0,
        "a thaw never leaves an undock separation behind"
    );
    assert_dock_matches(
        world,
        thawed_visitor,
        thawed_station,
        &before_dock,
        "right after restoring",
    );

    settle(&mut fresh);
    let world = fresh.world_mut();
    assert_dock_matches(
        world,
        thawed_visitor,
        thawed_station,
        &before_dock,
        "after settling more",
    );
}

/// [`station_dock_home`] with two more station ports, `bow` facing +Z and
/// `side` facing +X, ahead of the visitor's `hull` port, and one hull for
/// sale: the parked hull takes `bow`, the first free port it fits at, and the
/// visitor docks at `hull`.
#[cfg(feature = "serde")]
fn parked_and_visitor_home(input: SectorGenerationInput) -> SectorManifest {
    let mut manifest = station_dock_home(input);
    let dock = &mut manifest.stations[0];
    let StationDesignSource::Inline(design) = &mut dock.spawn.design else {
        unreachable!("the test station is inline");
    };
    let port = |id: &str, position: Vec3, rotation: Quat| SpaceshipSectionConfig {
        id: id.into(),
        position,
        rotation,
        source: SectionSource::prototype(TEST_DOCKING_SECTION_ID),
    };
    design.hull.sections.splice(
        0..0,
        [
            port(
                "bow",
                Vec3::new(0.0, 0.0, 1.0),
                Quat::from_rotation_y(std::f32::consts::PI),
            ),
            port(
                "side",
                Vec3::new(1.0, 0.0, 0.0),
                Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
            ),
        ],
    );
    dock.hulls.push(StationHullOffer {
        name: "parked hull".into(),
        terms: HullOfferType::Sale,
        design: ShipDesign {
            sections: vec![port("fore", Vec3::ZERO, Quat::IDENTITY)],
            ..default()
        },
    });
    manifest
}

/// [`ship_app`] with the station's parked hulls, and a count of every
/// docking connection added.
#[cfg(feature = "serde")]
fn parked_hull_app() -> App {
    let mut app = unfinished_ship_app();
    app.add_plugins(ParkedHullsPlugin);
    app.init_resource::<ConnectionsAdded>();
    app.add_observer(
        |_: On<Add, DockingConnection>, mut added: ResMut<ConnectionsAdded>| {
            added.0 += 1;
        },
    );
    app.finish();
    load_docking_section(&mut app);
    app
}

/// How many docking connections were added: a restore adds one, and a
/// second for the same pair would be a duplicate dock.
#[cfg(feature = "serde")]
#[derive(Resource, Default)]
struct ConnectionsAdded(usize);

/// The one parked hull at `station`.
#[cfg(feature = "serde")]
fn parked_hull(world: &mut World, station: Entity) -> Entity {
    let hulls: Vec<Entity> = world
        .query::<(Entity, &ParkedHull)>()
        .iter(world)
        .filter(|(_, parked)| parked.0 == station)
        .map(|(hull, _)| hull)
        .collect();
    let [hull] = hulls[..] else {
        panic!("the station must park exactly one hull: {hulls:?}");
    };
    hull
}

/// Asserts the live world holds the visitor and the parked hull docked at
/// the one station, each by its own connection with the ports it docked
/// with, the visitor still as `visitor_dock` holds it, and returns the
/// station.
#[cfg(feature = "serde")]
fn assert_parked_and_visitor_docked(
    world: &mut World,
    visitor_dock: &DockSnapshot,
    context: &str,
) -> Entity {
    let home = SectorCoord::ORIGIN;
    let [visitor] = body_with_id(world, &sector_id(home, "ship", 0))[..] else {
        panic!("{context}: the visitor must be live once");
    };
    let [station] = body_with_id(world, &sector_id(home, "station", 0))[..] else {
        panic!("{context}: the station must be live once");
    };
    let parked = parked_hull(world, station);
    assert_eq!(
        world.query::<&DockingConnection>().iter(world).count(),
        2,
        "{context}: one connection each for the visitor and the parked hull"
    );
    assert_eq!(
        world.query::<&UndockSeparation>().iter(world).count(),
        0,
        "{context}: no undock separation"
    );
    assert_dock_matches(world, visitor, station, visitor_dock, context);
    let parked_dock = dock_snapshot(world, parked, station);
    assert_eq!(
        (
            parked_dock.ship_port.as_str(),
            parked_dock.station_port.as_str()
        ),
        ("fore", "bow"),
        "{context}: the parked hull docks at its own port"
    );
    station
}

/// A station that holds a parked hull and a docked visitor brings both back
/// docked, each at its own port, after a retire and thaw and after a save
/// and Load. The visitor docks again in the thaw's own command flush, before
/// the parked-hull pass runs, so the parked hull never takes its port: the
/// visitor's connection is the one restore adds, and the parked hull's the
/// one capture adds, with no second connection for either.
#[cfg(feature = "serde")]
#[test]
fn a_station_brings_back_its_parked_hull_and_its_visitor_each_at_its_own_port() {
    let mut app = parked_hull_app();
    let home = SectorCoord::ORIGIN;
    let config = answering(parked_and_visitor_home);
    let edge = config.sector_edge;
    let world = app.world_mut();
    arm(world, config, home);
    let observer = ObserverBody {
        position: home.centre(edge) + Meters3::new(0.0, 0.0, edge.get() * 0.25),
        reach: Meters(100.0),
    };
    materialize::<Answers>(world, home, observer);
    settle(&mut app);
    let world = app.world_mut();
    let [visitor] = body_with_id(world, &sector_id(home, "ship", 0))[..] else {
        panic!("the visitor ship must spawn once");
    };
    let [station] = body_with_id(world, &sector_id(home, "station", 0))[..] else {
        panic!("the station must spawn once");
    };
    world.trigger(DockingConnectionRequest {
        entity: visitor,
        target: station,
    });
    settle(&mut app);
    let world = app.world_mut();
    let visitor_dock = dock_snapshot(world, visitor, station);
    assert_eq!(
        (
            visitor_dock.ship_port.as_str(),
            visitor_dock.station_port.as_str()
        ),
        ("hull", "hull"),
        "the visitor docks at the station's hull port"
    );
    assert_parked_and_visitor_docked(world, &visitor_dock, "before freezing");
    let saved = crate::snapshot_sectors::<Answers>(world).expect("both docks are kept");

    world.resource_mut::<ConnectionsAdded>().0 = 0;
    walk_to::<Answers>(world, home.offset(5, 0, 0));
    assert_eq!(
        world.query::<&DockingConnection>().iter(world).count(),
        0,
        "both docks despawn with the station"
    );
    walk_to::<Answers>(world, home);
    materialize::<Answers>(world, home, observer);
    let world = app.world_mut();
    assert_eq!(
        world.resource::<ConnectionsAdded>().0,
        1,
        "the thaw flush docks the visitor before any parked hull spawns"
    );
    settle(&mut app);
    assert_parked_and_visitor_docked(app.world_mut(), &visitor_dock, "after a thaw");
    assert_eq!(
        app.world().resource::<ConnectionsAdded>().0,
        2,
        "a thaw adds one connection per dock"
    );

    let text = ron::to_string(&saved).expect("the ledger writes");
    let read: FrozenSectors = ron::from_str(&text).expect("the ledger reads back");
    let mut fresh = parked_hull_app();
    arm(fresh.world_mut(), answering(parked_and_visitor_home), home);
    let world = fresh.world_mut();
    world.resource_mut::<FrozenSectors>().restore(read);
    materialize::<Answers>(world, home, observer);
    assert_eq!(
        world.resource::<ConnectionsAdded>().0,
        1,
        "the Load's thaw flush docks the visitor before any parked hull spawns"
    );
    settle(&mut fresh);
    assert_parked_and_visitor_docked(fresh.world_mut(), &visitor_dock, "after a Load");
    assert_eq!(
        fresh.world().resource::<ConnectionsAdded>().0,
        2,
        "a Load adds one connection per dock"
    );
}

/// A canister is persistent cargo: it joins the cell it drifts in, freezes
/// with it, and comes back with its own runtime id, contents and health.
#[test]
fn a_loose_canister_freezes_with_the_cell_it_drifts_in() {
    let mut app = ship_app();
    let home = SectorCoord::ORIGIN;
    let config = rocks();
    let edge = config.sector_edge;
    arm(app.world_mut(), config, home);
    let root = materialize::<Rocks>(app.world_mut(), home, far());
    settle(&mut app);

    let world = app.world_mut();
    let id = world
        .resource_mut::<nova_gameplay::inventory::CargoCanisterIdAllocator>()
        .mint();
    let items = test_items();
    let mut contents = CargoCanister::new(&items, &ItemRoleType::IronOre.into(), 4);
    contents.add(&items, &ItemRoleType::WaterIce.into(), 2);
    let at = Transform::from_translation(home.centre(edge).to_engine() + Vec3::Y * 40.0);
    let canister = world
        .spawn((
            cargo_canister(contents.clone(), at, Vec3::X, AssetRef::default()),
            id,
        ))
        .id();
    world.get_mut::<Health>(canister).unwrap().current = 11.0;

    walk_to::<Rocks>(world, home);
    assert_eq!(
        world.get::<ChildOf>(canister).map(ChildOf::parent),
        Some(root),
        "a top-level canister joins the cell it stands in"
    );

    walk_to::<Rocks>(world, home.offset(5, 0, 0));
    assert!(world.get_entity(canister).is_err(), "the canister froze");
    assert!(world
        .resource::<FrozenSectors>()
        .get(home)
        .unwrap()
        .bodies()
        .iter()
        .any(|body| matches!(body.body(), FrozenBodyType::Canister(_))));

    walk_to::<Rocks>(world, home);
    materialize::<Rocks>(world, home, far());
    settle(&mut app);
    let world = app.world_mut();
    let thawed: Vec<(CargoCanisterRuntimeId, CargoCanister, f32)> = world
        .query::<(&CargoCanisterRuntimeId, &CargoCanister, &Health)>()
        .iter(world)
        .map(|(id, contents, health)| (*id, contents.clone(), health.current))
        .collect();
    assert_eq!(thawed, vec![(id, contents, 11.0)]);
}

/// [`one_ship`] holding five iron ore and 200 PDC rounds: 90 kg of its one
/// hull section's 100 kg hold.
fn laden_ship(input: SectorGenerationInput) -> SectorManifest {
    let mut manifest = one_ship(input);
    manifest.ships[0].spawn.inventory = ShipInventoryStock::new([
        (ItemRoleType::IronOre.into(), 5),
        (ItemRoleType::PdcRound.into(), 200),
    ]);
    manifest
}

/// The wreck cargo live in a world: every drifting canister with its id, by
/// id, and every waiting drop's canisters, front first, in a stable order.
fn wreck_cargo(world: &mut World) -> (Vec<(u64, CargoCanister)>, Vec<Vec<CargoCanister>>) {
    let mut drifting: Vec<(u64, CargoCanister)> = world
        .query::<(&CargoCanisterRuntimeId, &CargoCanister)>()
        .iter(world)
        .map(|(id, contents)| (id.0, contents.clone()))
        .collect();
    drifting.sort_by_key(|(id, _)| *id);
    let mut waiting: Vec<Vec<CargoCanister>> = world
        .query_filtered::<&MinedCanisterQueue, With<MinedOreDrop>>()
        .iter(world)
        .map(|queue| queue.canisters().cloned().collect())
        .collect();
    waiting.sort_by_key(|queue| format!("{queue:?}"));
    (drifting, waiting)
}

/// A destroyed laden ship's cargo is persistent: its section's parts share
/// and its hold, some already drifting and some still waiting in their drops,
/// freeze with the cell, come back with the same ids and contents (through a
/// written and read ledger where the save format is built), and then drain to
/// exactly the hold plus `floor(0.5 * count)` of each recipe part.
#[test]
fn a_destroyed_ships_cargo_freezes_and_thaws_with_its_cell() {
    let mut app = unfinished_ship_app();
    app.add_plugins((MiningPlugin { render: false }, WreckCargoPlugin));
    app.finish();
    {
        let mut sections = app.world_mut().resource_mut::<GameSections>();
        let hull = &mut sections.0[0].base;
        // The test hull has no health by default, and a hit of nothing
        // destroys nothing.
        hull.health = 100.0;
        hull.build = BuildRecipe {
            credits: 80,
            parts: [("FrameSpar", 4), ("SectionPanel", 3)]
                .into_iter()
                .map(|(item, count)| (ItemDesignId::from(item), count))
                .collect(),
            strip_yield: 0.5,
        };
    }
    let home = SectorCoord::ORIGIN;
    arm(app.world_mut(), answering(laden_ship), home);
    materialize::<Answers>(app.world_mut(), home, far());
    settle(&mut app);

    let world = app.world_mut();
    let ship_id = sector_id(home, "ship", 0);
    let [ship] = body_with_id(world, &ship_id)[..] else {
        panic!("home must spawn its ship");
    };
    let [section] = world
        .query_filtered::<(Entity, &ChildOf), With<SectionMarker>>()
        .iter(world)
        .filter(|(_, child_of)| child_of.parent() == ship)
        .map(|(section, _)| section)
        .collect::<Vec<_>>()[..]
    else {
        panic!("the ship has one section");
    };
    let amount = world.get::<Health>(section).unwrap().max;
    world.trigger(HealthApplyDamage {
        entity: section,
        source: None,
        amount,
    });
    // Both drops stand where the one section was and leave along the same
    // line, so the second waits on the first's canister: some cargo drifts
    // and some still waits when the cell retires.
    for _ in 0..120 {
        app.update();
        let world = app.world_mut();
        let (drifting, waiting) = wreck_cargo(world);
        if world.get_entity(ship).is_err() && !drifting.is_empty() && !waiting.is_empty() {
            break;
        }
    }
    let world = app.world_mut();
    assert!(world.get_entity(ship).is_err(), "the ship never died");
    let before = wreck_cargo(world);
    assert!(
        !before.0.is_empty() && !before.1.is_empty(),
        "the cell must retire with cargo both drifting and waiting: {before:?}"
    );

    walk_to::<Answers>(world, home);
    walk_to::<Answers>(world, home.offset(5, 0, 0));
    assert_eq!(
        wreck_cargo(world),
        (Vec::new(), Vec::new()),
        "frozen cargo must not stay in the world"
    );
    let record = world.resource::<FrozenSectors>().get(home).unwrap();
    assert!(record
        .bodies()
        .iter()
        .any(|body| matches!(body.body(), FrozenBodyType::Canister(_))));
    assert!(record
        .bodies()
        .iter()
        .any(|body| matches!(body.body(), FrozenBodyType::OreDrop(_))));
    #[cfg(feature = "serde")]
    {
        let text = ron::to_string(world.resource::<FrozenSectors>()).expect("the ledger writes");
        let read: FrozenSectors = ron::from_str(&text).expect("the ledger reads back");
        // The same session thaws the ledger it read back, so the canister id
        // allocator stays this session's.
        *world.resource_mut::<FrozenSectors>() = read;
    }

    walk_to::<Answers>(world, home);
    materialize::<Answers>(world, home, far());
    settle(&mut app);
    let world = app.world_mut();
    assert!(
        body_with_id(world, &ship_id).is_empty(),
        "a destroyed ship stays destroyed"
    );
    let thawed = wreck_cargo(world);
    assert_eq!(thawed, before, "the cargo comes back as it froze");

    for _ in 0..20_000 {
        app.update();
        let world = app.world_mut();
        if world.query::<&MinedOreDrop>().iter(world).next().is_none() {
            break;
        }
    }
    let world = app.world_mut();
    let (drifting, waiting) = wreck_cargo(world);
    assert!(waiting.is_empty(), "the thawed drops never drained");
    let mut ids: Vec<u64> = drifting.iter().map(|(id, _)| *id).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), drifting.len(), "every canister keeps its own id");
    let mut totals: Vec<(String, u32)> = Vec::new();
    for (item, count) in drifting.iter().flat_map(|(_, contents)| contents.stacks()) {
        match totals.iter_mut().find(|(each, _)| each == item.as_str()) {
            Some((_, total)) => *total += count,
            None => totals.push((item.to_string(), count)),
        }
    }
    totals.sort();
    assert_eq!(
        totals,
        vec![
            ("FrameSpar".to_string(), 2),
            ("IronOre".to_string(), 5),
            ("PdcRound".to_string(), 200),
            ("SectionPanel".to_string(), 1),
        ]
    );
}

/// One advancing frame of virtual time.
const FRAME: Duration = Duration::from_millis(16);

/// A retiring cell freezes every rock that can freeze on the frame it
/// retires, and keeps only the rock still being destroyed live under its
/// root. Walking back does not stop the retirement: the cell already froze
/// part of itself, so it stays retiring rather than live with half its rocks.
#[test]
fn settled_rocks_freeze_while_an_unsettled_sibling_holds_its_cell() {
    let mut app = App::new();
    app.add_plugins(AsteroidPlugin { render: false });
    let world = app.world_mut();
    let home = SectorCoord::ORIGIN;
    arm(world, rocks(), home);
    let root = materialize::<Rocks>(world, home, far());
    app.update();

    let world = app.world_mut();
    let unsettled_id = sector_id(home, "body", 0);
    let [unsettled] = body_with_id(world, &unsettled_id)[..] else {
        panic!("home must spawn its first rock");
    };
    let node = rock_node(world, unsettled);
    world.entity_mut(node).insert(IntegrityDestroyMarker);

    walk_to::<Rocks>(world, home.offset(5, 0, 0));
    let settled: Vec<String> = (1..4)
        .map(|index| sector_id(home, "body", index).to_string())
        .collect();
    assert_eq!(frozen_ids(world, home), settled, "the settled rocks froze");
    assert_eq!(
        world.get::<ChildOf>(unsettled).map(ChildOf::parent),
        Some(root),
        "the unsettled rock stays live under its root"
    );
    let live: Vec<Entity> = world
        .query_filtered::<Entity, With<AsteroidMarker>>()
        .iter(world)
        .collect();
    assert_eq!(live, vec![unsettled], "only the unsettled rock is live");

    walk_to::<Rocks>(world, home);
    assert_eq!(
        frozen_ids(world, home),
        settled,
        "the cell keeps retiring when it is desired again"
    );
    assert!(world.get_entity(unsettled).is_ok());
    assert_eq!(
        world
            .query_filtered::<(), With<SectorRoot>>()
            .iter(world)
            .count(),
        1,
        "the retiring root is the only one"
    );
}

/// The frame the last unsettled rock settles, it freezes into the record its
/// siblings froze into and its root retires. The cell then comes back whole.
#[test]
fn an_unsettled_rock_retires_its_cell_once_it_settles() {
    let mut app = App::new();
    app.add_plugins(AsteroidPlugin { render: false });
    let world = app.world_mut();
    let home = SectorCoord::ORIGIN;
    arm(world, rocks(), home);
    let root = materialize::<Rocks>(world, home, far());
    app.update();

    let world = app.world_mut();
    let [unsettled] = body_with_id(world, &sector_id(home, "body", 0))[..] else {
        panic!("home must spawn its first rock");
    };
    let node = rock_node(world, unsettled);
    world.entity_mut(node).insert(IntegrityDestroyMarker);
    let away = home.offset(5, 0, 0);
    walk_to::<Rocks>(world, away);
    assert!(
        world.get_entity(root).is_ok(),
        "the unsettled rock holds it"
    );

    world.entity_mut(node).remove::<IntegrityDestroyMarker>();
    walk_to::<Rocks>(world, away);
    assert!(world.get_entity(unsettled).is_err(), "the rock froze");
    assert!(world.get_entity(root).is_err(), "the cell retired");
    let mut ids = frozen_ids(world, home);
    ids.sort();
    let expected: Vec<String> = (0..4)
        .map(|index| sector_id(home, "body", index).to_string())
        .collect();
    assert_eq!(ids, expected, "one record holds every rock");

    walk_to::<Rocks>(world, home);
    let root = materialize::<Rocks>(world, home, far());
    app.update();
    let world = app.world_mut();
    let mut back: Vec<String> = world
        .query_filtered::<(&EntityId, &ChildOf), With<AsteroidMarker>>()
        .iter(world)
        .filter(|(_, child_of)| child_of.parent() == root)
        .map(|(id, _)| id.0.clone())
        .collect();
    back.sort();
    assert_eq!(back, expected, "the cell comes back whole");
}

/// A rock that stays unsettled holds its cell for a bounded number of
/// advancing frames and then fails loudly. A paused frame neither counts nor
/// drops the wait: a thousand paused frames pass, the advancing ones are
/// interleaved with more, and the bound still falls on the 601st advancing
/// frame. A counted paused frame would panic before the advancing run.
#[test]
fn a_rock_that_never_settles_fails_at_the_bound() {
    let mut app = App::new();
    app.add_plugins(AsteroidPlugin { render: false });
    let world = app.world_mut();
    let home = SectorCoord::ORIGIN;
    arm(world, rocks(), home);
    materialize::<Rocks>(world, home, far());
    app.update();

    let world = app.world_mut();
    let [unsettled] = body_with_id(world, &sector_id(home, "body", 0))[..] else {
        panic!("home must spawn its first rock");
    };
    let node = rock_node(world, unsettled);
    world.entity_mut(node).insert(IntegrityDestroyMarker);
    let away = home.offset(5, 0, 0);
    let paused = |world: &mut World| {
        world
            .resource_mut::<Time<Virtual>>()
            .advance_by(Duration::ZERO);
        walk_to::<Rocks>(world, away);
    };
    for _ in 0..1_000 {
        paused(world);
    }
    for _ in 0..600 {
        world.resource_mut::<Time<Virtual>>().advance_by(FRAME);
        walk_to::<Rocks>(world, away);
        paused(world);
    }
    assert!(
        world.get_entity(unsettled).is_ok(),
        "600 advancing frames are within the bound"
    );
    world.resource_mut::<Time<Virtual>>().advance_by(FRAME);
    let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        walk_to::<Rocks>(world, away);
    }))
    .expect_err("the 601st advancing frame fails loudly");
    let message = failure
        .downcast_ref::<String>()
        .expect("the bound panics with a formatted message");
    assert!(
        message.contains(
            "in (0, 0, 0) stayed unsettled for 601 advancing frames, more than 600: \
             freeze_asteroid: the node is exhausting"
        ),
        "{message}"
    );
}

/// Replacing the world takes the bodies waiting top-level for a cell with
/// its roots: one that waits for a desired cell to materialize, and one that
/// waits off-window for its owner to settle it. Neither is adopted into the
/// new world's cell. The observer's ship and an authored, addressable body
/// stand top-level too, and the scenario owns them, so both stay.
#[test]
fn replacing_the_world_takes_the_bodies_waiting_top_level() {
    let mut app = App::new();
    app.add_plugins(AsteroidPlugin { render: false });
    let world = app.world_mut();
    let config = rocks();
    let edge = config.sector_edge;
    let home = SectorCoord::ORIGIN;
    arm(world, config, home);
    let root = materialize::<Rocks>(world, home, far());
    app.update();

    let world = app.world_mut();
    let [desired, unsettled, authored] = [0, 1, 2].map(|index| {
        let [rock] = body_with_id(world, &sector_id(home, "body", index))[..] else {
            panic!("home must spawn rock {index}");
        };
        rock
    });
    let next_door = home.offset(1, 0, 0).centre(edge).to_engine();
    let beyond = home.offset(5, 0, 0).centre(edge).to_engine();
    world
        .entity_mut(desired)
        .remove::<ChildOf>()
        .insert((Transform::from_translation(next_door), Position(next_door)));
    let node = rock_node(world, unsettled);
    world.entity_mut(node).insert(IntegrityDestroyMarker);
    world
        .entity_mut(unsettled)
        .remove::<ChildOf>()
        .insert((Transform::from_translation(beyond), Position(beyond)));
    world.resource_mut::<Time<Virtual>>().advance_by(FRAME);
    walk_to::<Rocks>(world, home);
    for rock in [desired, unsettled] {
        assert!(
            world.get::<ChildOf>(rock).is_none(),
            "{rock} waits top-level"
        );
    }
    // A streamed rock is the authored seam's bundle without the addressable
    // marker: `base_scenario_object` plus the asteroid.
    world
        .entity_mut(authored)
        .remove::<ChildOf>()
        .insert(ScenarioAddressableMarker);
    let player = world
        .spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            WorldObserver,
            Transform::from_translation(home.centre(edge).to_engine()),
        ))
        .id();

    world.remove_resource::<WorldConfig<Rocks>>();
    world
        .run_system_once(clear_sector_work::<Rocks>)
        .expect("cleanup runs");
    assert!(world.get_entity(root).is_err(), "the root retired");
    assert!(
        world.get_entity(desired).is_err(),
        "the rock waiting for a desired cell went with the world"
    );
    assert!(
        world.get_entity(unsettled).is_err(),
        "the unsettled rock waiting off-window went with the world"
    );
    assert!(
        world.get_entity(authored).is_ok(),
        "the authored, addressable rock stays with the scenario"
    );
    assert!(
        world.get_entity(player).is_ok(),
        "the observer's ship stays with the scenario"
    );
}
