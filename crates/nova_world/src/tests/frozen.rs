//! What a cell keeps while it is off-window, and which cell keeps a body.
//!
//! Each proof drives the real stage systems - adoption, retirement, cleanup -
//! against a bare or headless app and stands in for the worker with the same
//! preparation a job runs, so a record goes through exactly the path a live
//! session takes it through.

use std::time::Duration;

use avian3d::prelude::*;
use bevy::{ecs::system::RunSystemOnce, prelude::*};
use nova_events::prelude::{
    EntityId, Meters, Meters3, MetersPerSecond3, ScenarioAddressableMarker,
};
use nova_gameplay::{
    prelude::{
        AssetRef, CargoCanister, CargoCanisterRuntimeId, DamageMark, DamageMarks, Health,
        IntegrityDestroyMarker, PlayerSpaceshipMarker, SectionMarker, ShipCredits, ShipInventory,
        ShipInventoryStock, SpaceshipRootMarker, TempEntity, ITEM_IRON_ORE, ITEM_WATER_ICE,
    },
    test_support::{settle, test_items, unfinished_integrity_physics_app},
};
use nova_scenario::prelude::{
    freeze_ship, AsteroidCarvePlugin, AsteroidField, AsteroidMarker, AsteroidPlugin, SectionSource,
    ShipDesign, ShipDesignSource, SpaceshipController, SpaceshipDesign, SpaceshipPlugin,
    SpaceshipSectionConfig, KIND_ROCK,
};
use nova_ship::prelude::{
    cargo_canister, docking_section, thruster_section, AINonCombatant, BaseSectionConfig,
    BodyRadius, DockedHelmType, DockedShip, DockingConnection, DockingConnectionRequest,
    DockingHelmRequest, DockingSectionConfig, FlightIntent, GameSections, NovaFlightPlugin,
    PDControllerPlugin, SectionAnimations, SectionConfig, SectionKind, ShipCapabilities,
    SpaceshipSectionPlugin, ThrusterSectionConfig, TurretSectionConfig,
};

use super::{answering, empty, rocks, ship, test_sections, Answers, Rocks, TEST_HULL_SECTION_ID};
use crate::{
    adopt_moving_bodies, clear_sector_work, frozen::SettlingBodies, generation::prepare_cell,
    materialize_sector, retire_sectors, sector_id, streaming::ClearedConfig, CurrentSector,
    FrozenBodyType, FrozenSector, FrozenSectors, ObserverBody, PendingSectorShip, ReadySectors,
    SectorAsteroid, SectorCoord, SectorGenerationInput, SectorGenerator, SectorJobStats,
    SectorManifest, SectorRoot, WorldConfig, WorldObserver,
};

/// Arm `config` around `centre` with nothing live yet, so the arming frame's
/// cleanup has nothing to take and every stage after it accepts the config.
fn arm<G: SectorGenerator>(world: &mut World, config: WorldConfig<G>, centre: SectorCoord) {
    world.insert_resource(config);
    world.init_resource::<ReadySectors>();
    world.init_resource::<SectorJobStats>();
    world.init_resource::<ClearedConfig>();
    world.init_resource::<FrozenSectors>();
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
    let record = world.resource_mut::<FrozenSectors>().take(coord);
    let prepared = prepare_cell(config, coord, record.as_ref()).expect("the cell prepares");
    let sections = world
        .get_resource::<GameSections>()
        .cloned()
        .unwrap_or_default();
    let root = materialize_sector(
        &mut world.commands(),
        prepared,
        record,
        &AssetRef::Path(nova_assets::prelude::ASTEROID_TEXTURE_PATH.to_string()),
        &sections,
        observer,
    );
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
fn body_with_id(world: &mut World, id: &str) -> Vec<Entity> {
    world
        .query::<(Entity, &EntityId)>()
        .iter(world)
        .filter(|(_, each)| each.0 == id)
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
    assert_eq!(frozen_ids(world, far_cell), vec![wanderer.clone()]);

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
        .map(|index| sector_id(far_cell, "body", index))
        .collect();
    expected.push(wanderer.clone());
    expected.sort();
    assert_eq!(
        under.iter().map(|(id, _)| id.clone()).collect::<Vec<_>>(),
        expected,
        "the generated rocks and the arrival all stand in the cell"
    );
    let at = |id: &str| under.iter().find(|(each, _)| each == id).unwrap().1;
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

    world.remove_resource::<WorldConfig<Rocks>>();
    world
        .run_system_once(clear_sector_work::<Rocks>)
        .expect("cleanup runs");
    assert!(
        world.resource::<FrozenSectors>().is_empty(),
        "the session's records are gone"
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
        radius: Meters(60.0),
        initial_velocity: MetersPerSecond3::ZERO,
        kind: KIND_ROCK.into(),
        seed: 3,
    };
    let mut fought = ship(
        sector_id(input.coord, "ship", 0),
        centre + Meters3::new(quarter, 0.0, 0.0),
        TEST_HULL_SECTION_ID,
    );
    let mut aft = fought.design.sections[0].clone();
    fought.design.sections[0].id = "fore".into();
    aft.id = "aft".into();
    aft.position = Vec3::Z;
    aft.source = SectionSource::prototype(TEST_TURRET_SECTION_ID);
    fought.design.sections.push(aft);
    fought.stock = ShipInventoryStock::new([(ITEM_IRON_ORE.into(), 10)]);
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
    app.finish();
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
        .remove(&ITEM_IRON_ORE.into(), 7);
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
            .any(|body| matches!(body.body(), FrozenBodyType::PendingShip(_))),
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
        .map(|held| held.ship().id.clone())
        .collect();
    assert_eq!(held, vec![sector_id(home, "ship", 1)]);
    assert_eq!(
        world
            .query_filtered::<(), With<AsteroidMarker>>()
            .iter(world)
            .count(),
        1,
        "the generator's rock does not spawn again beside the frozen one"
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
    let contents = CargoCanister::new(&test_items(), &ITEM_IRON_ORE.into(), 4);
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
        .map(|held| held.ship().id.clone())
        .collect();
    assert_eq!(held, vec![sector_id(home, "ship", 1)]);
}

/// A docked pair is undocked ON PAPER before it freezes: `snapshot_sectors`
/// keeps both ships at the poses they stood at, with no dock between them in
/// the record, and leaves the live, still-docked pair untouched. Restoring
/// and materializing the ledger brings the partner back alone and undocked,
/// where it froze.
#[cfg(feature = "serde")]
#[test]
fn a_docked_pair_saves_both_ships_undocked_at_their_poses() {
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
    assert!(world.get::<DockedShip>(player).is_some());
    assert!(world.get::<DockedShip>(partner).is_some());
    let player_pose = world
        .get::<Position>(player)
        .copied()
        .expect("the player has a pose");
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
    assert_eq!(
        world.get::<Position>(player).copied(),
        Some(player_pose),
        "freeze_ship only reads the player"
    );

    let saved =
        crate::snapshot_sectors::<Nothing>(world).expect("the docked pair freezes undocked");

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
    assert_eq!(
        world.get::<ChildOf>(partner).map(ChildOf::parent),
        Some(home_root),
        "the live partner stays under home"
    );
    assert_eq!(world.get::<Position>(player).copied(), Some(player_pose));
    assert_eq!(world.get::<Position>(partner).copied(), Some(partner_pose));

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

    let mut fresh = ship_app();
    load_docking_section(&mut fresh);
    arm(
        fresh.world_mut(),
        WorldConfig {
            seed: 1,
            sector_edge: edge,
            active_radius: 1,
            generator: Nothing,
        },
        home,
    );
    fresh
        .world_mut()
        .resource_mut::<FrozenSectors>()
        .restore(read);
    let root = materialize::<Nothing>(fresh.world_mut(), home, far());
    settle(&mut fresh);

    let world = fresh.world_mut();
    let [restored_partner] = body_with_id(world, "partner")[..] else {
        panic!("the partner must come back once");
    };
    assert_eq!(
        world.get::<ChildOf>(restored_partner).map(ChildOf::parent),
        Some(root)
    );
    assert_eq!(
        world
            .get::<Transform>(restored_partner)
            .map(|transform| transform.translation),
        Some(partner_pose.0),
        "the partner comes back at the pose it froze with"
    );
    assert!(
        world.get::<DockedShip>(restored_partner).is_none(),
        "the partner comes back undocked"
    );
    assert_eq!(
        world.query::<&DockingConnection>().iter(world).count(),
        0,
        "no dock exists in the fresh world"
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
    let mut contents = CargoCanister::new(&items, &ITEM_IRON_ORE.into(), 4);
    contents.add(&items, &ITEM_WATER_ICE.into(), 2);
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
    let settled: Vec<String> = (1..4).map(|index| sector_id(home, "body", index)).collect();
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
    let expected: Vec<String> = (0..4).map(|index| sector_id(home, "body", index)).collect();
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
