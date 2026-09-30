use std::{collections::BTreeMap, panic::AssertUnwindSafe};

use bevy::ecs::system::RunSystemOnce;
use nova_assets::prelude::{ContentCatalogDigest, LoadedSectionPack};
use nova_gameplay::prelude::{Fnv64, ItemType};
use nova_scenario::prelude::{
    resolve_ship_design, GameShipDesigns, ScenarioConfig, SectionSource, ShipDesign,
    ShipDesignSource, SpaceshipSectionConfig,
};
use nova_ship::prelude::GameSections;

use super::*;

const SEED: u32 = 20_260_923;

/// The ship-layout test packs as the merge would register them, under
/// `digest`.
fn loaded_with(digest: u64) -> LoadedSectionPacks {
    LoadedSectionPacks {
        packs: crate::ship_layout::tests::packs()
            .into_iter()
            .map(|pack| LoadedSectionPack {
                id: pack.id,
                dependencies: pack.dependencies,
                sections: pack.sections,
            })
            .collect(),
        digest: ContentCatalogDigest(digest),
    }
}

/// The loaded catalog every test world arms over.
fn loaded() -> LoadedSectionPacks {
    loaded_with(1)
}

/// A generator pinned to [`loaded`].
pub(crate) fn fixture_world() -> NovaLayeredWorld {
    NovaLayeredWorld::from_loaded(&loaded()).expect("the fixture packs build a snapshot")
}

fn scenario(role: ScenarioRole) -> CurrentScenario {
    CurrentScenario(Some(ScenarioConfig {
        role,
        ..ScenarioConfig::new("under_test", "Under Test", "sky.png".into())
    }))
}

/// A world holding a live scenario of `role` and the New Game session.
fn world(role: ScenarioRole) -> World {
    let mut world = World::new();
    world.insert_resource(scenario(role));
    world.insert_resource(OpenWorldSession { seed: SEED });
    world.insert_resource(loaded());
    world
}

fn sync(world: &mut World) {
    world
        .run_system_once(sync_open_world)
        .expect("sync_open_world runs");
}

fn armed(world: &World) -> Option<&WorldConfig<NovaLayeredWorld>> {
    world.get_resource::<WorldConfig<NovaLayeredWorld>>()
}

fn session_config() -> WorldConfig<NovaLayeredWorld> {
    WorldConfig {
        seed: SEED,
        sector_edge: OPEN_WORLD_SECTOR_EDGE,
        active_radius: OPEN_WORLD_ACTIVE_RADIUS,
        generator: fixture_world(),
    }
}

/// The promise a world seed makes: the whole live window around the start
/// describes the same pristine sectors whichever order it is walked in.
#[test]
fn the_open_world_describes_the_same_sectors_in_any_visit_order() {
    let config = session_config();
    let cells: Vec<SectorCoord> = desired_sectors(SectorCoord::ORIGIN, config.active_radius)
        .into_iter()
        .collect();
    let describe_all = |order: &[SectorCoord]| {
        order
            .iter()
            .map(|coord| {
                let description = generate_sector(&config, *coord)
                    .unwrap_or_else(|fault| panic!("sector {coord:?}: {fault}"));
                (*coord, description.canonical())
            })
            .collect::<BTreeMap<_, _>>()
    };

    let forward = describe_all(&cells);
    let reverse = describe_all(&cells.iter().rev().copied().collect::<Vec<_>>());
    let strided = describe_all(
        &(0..cells.len())
            .map(|step| cells[step * 7 % cells.len()])
            .collect::<Vec<_>>(),
    );

    assert_eq!(forward.len(), 125, "radius 2 keeps a 5x5x5 window live");
    assert!(
        forward.values().any(|text| text.contains("\n  section ")),
        "the window must hold a generated ship"
    );
    assert_eq!(forward, reverse, "a reverse walk changed a sector");
    assert_eq!(forward, strided, "a strided walk changed a sector");
}

/// A wreck is a loot source: every derelict in the live window carries 1 to 8
/// hull plates that fit the hold its resolved design gives it, and an intact
/// ship carries nothing. A wreck with no hull section left holds no plate, so
/// it gets no stock rather than an empty lootable hold.
#[test]
fn every_planned_wreck_carries_one_to_eight_plates_its_hull_holds_and_an_intact_ship_none() {
    let config = session_config();
    let parts = config.generator.parts();
    let sections = GameSections(
        parts
            .parts()
            .iter()
            .map(|part| part.config.clone())
            .collect(),
    );
    let mut wrecks = 0;
    for coord in desired_sectors(SectorCoord::ORIGIN, config.active_radius) {
        let description = generate_sector(&config, coord)
            .unwrap_or_else(|fault| panic!("sector {coord:?}: {fault}"));
        for ship in description.ships() {
            let stacks: Vec<(ItemType, u32)> = ship.stock.stacks().collect();
            match ship.condition {
                SectorShipConditionType::Intact => {
                    assert!(
                        stacks.is_empty(),
                        "intact ship {} carries {stacks:?}",
                        ship.id
                    );
                }
                SectorShipConditionType::Derelict => {
                    wrecks += 1;
                    let [(ItemType::HullPlate, plates)] = stacks[..] else {
                        panic!("wreck {} carries {stacks:?}, not one plate stack", ship.id);
                    };
                    assert!(
                        WRECK_PLATES.contains(&plates),
                        "wreck {}: {plates}",
                        ship.id
                    );
                    let (resolved, errors) = resolve_ship_design(
                        &ShipDesignSource::Inline(ship.design.clone()),
                        &GameShipDesigns::default(),
                        &sections,
                    );
                    assert!(errors.is_empty(), "wreck {}: {errors:?}", ship.id);
                    assert!(
                        ship.stock.mass_g() <= u64::from(resolved.cargo_capacity_g()),
                        "wreck {} overfills its hold",
                        ship.id
                    );
                }
            }
        }
    }
    assert!(wrecks > 0, "the window must hold a wreck");

    let docks_only = ShipDesign {
        sections: parts
            .parts()
            .iter()
            .filter(|part| part.family == ShipPartFamilyType::Docking)
            .take(1)
            .map(|part| SpaceshipSectionConfig {
                id: "dock".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                source: SectionSource::prototype(part.id()),
            })
            .collect(),
        ..default()
    };
    assert_eq!(
        docks_only.sections.len(),
        1,
        "the fixture has a docking port"
    );
    assert_eq!(wreck_stock(parts, &docks_only, 0), None);
}

/// A pinned window generates the bodies it was recorded with: every rock,
/// planetoid and ship, to the canonical text, in the window the examples fly.
///
/// The digest is FNV-1a 64 over the concatenated canonical descriptions of
/// the 125 cells around the origin, walked in `desired_sectors` order, for
/// seed 20,260,922 at a 32 km edge. Every input is written out here rather
/// than read from the session constants, so retuning New Game cannot move the
/// recorded window. A deliberate change to the generator's output changes
/// this digest; record the new value from the left side of this test's
/// failure message.
#[test]
fn a_pinned_window_generates_the_recorded_bodies() {
    let config = WorldConfig {
        seed: 20_260_922,
        sector_edge: Meters(32_000.0),
        active_radius: 2,
        generator: fixture_world(),
    };
    let canonical: String = desired_sectors(SectorCoord::ORIGIN, config.active_radius)
        .into_iter()
        .map(|coord| {
            generate_sector(&config, coord)
                .unwrap_or_else(|fault| panic!("sector {coord:?}: {fault}"))
                .canonical()
        })
        .collect();
    assert_eq!(
        Fnv64::new().write(canonical.as_bytes()).finish(),
        0xed7e_d8f2_b5b5_92fd,
        "the pinned window's bodies changed"
    );
}

#[test]
fn an_open_world_scenario_with_one_player_arms_the_world_around_it() {
    let mut world = world(ScenarioRole::OpenWorld);
    let player = world.spawn(PlayerSpaceshipMarker).id();

    sync(&mut world);

    assert_eq!(armed(&world), Some(&session_config()));
    assert!(world.entity(player).contains::<WorldObserver>());
}

/// `nova_world` clears every sector when the config changes, so a frame that
/// re-inserted an identical config would tear the world down every frame.
#[test]
fn an_armed_world_is_not_rewritten_by_the_next_sync() {
    let mut world = world(ScenarioRole::OpenWorld);
    world.spawn(PlayerSpaceshipMarker);
    sync(&mut world);
    world.clear_trackers();

    sync(&mut world);

    assert!(!world.is_resource_changed::<WorldConfig<NovaLayeredWorld>>());
}

/// The merge runs again whenever the mod set or a bundle load changes. The
/// same effective catalog merged again is the same world.
#[test]
fn a_remerged_identical_catalog_does_not_rewrite_the_armed_world() {
    let mut world = world(ScenarioRole::OpenWorld);
    world.spawn(PlayerSpaceshipMarker);
    sync(&mut world);
    world.clear_trackers();

    world.insert_resource(loaded());
    sync(&mut world);

    assert!(!world.is_resource_changed::<WorldConfig<NovaLayeredWorld>>());
}

/// A new config would retire every sector and regenerate the world from
/// content it was not armed over, so the refusal comes before any write.
#[test]
fn a_changed_catalog_under_an_armed_world_is_refused() {
    let mut world = world(ScenarioRole::OpenWorld);
    world.spawn(PlayerSpaceshipMarker);
    sync(&mut world);
    world.clear_trackers();

    world.insert_resource(loaded_with(2));
    let refused = std::panic::catch_unwind(AssertUnwindSafe(|| sync(&mut world)))
        .expect_err("a changed catalog under an armed world must be refused");

    let message = refused
        .downcast_ref::<String>()
        .expect("the refusal is a formatted message");
    assert!(
        message.contains("loaded content changed under an armed open world"),
        "{message}"
    );
    assert_eq!(armed(&world), Some(&session_config()));
    assert!(!world.is_resource_changed::<WorldConfig<NovaLayeredWorld>>());
}

#[test]
#[should_panic(expected = "no LoadedSectionPacks")]
fn an_open_world_with_no_loaded_catalog_is_refused() {
    let mut world = world(ScenarioRole::OpenWorld);
    world.remove_resource::<LoadedSectionPacks>();
    world.spawn(PlayerSpaceshipMarker);

    sync(&mut world);
}

#[test]
#[should_panic(expected = "do not build a snapshot")]
fn an_open_world_whose_ship_parts_do_not_build_is_refused() {
    let mut world = world(ScenarioRole::OpenWorld);
    world.insert_resource(LoadedSectionPacks {
        packs: Vec::new(),
        digest: ContentCatalogDigest(1),
    });
    world.spawn(PlayerSpaceshipMarker);

    sync(&mut world);
}

#[test]
fn a_scenario_of_another_role_or_none_disarms_the_world() {
    for current in [
        scenario(ScenarioRole::Backdrop),
        scenario(ScenarioRole::Chapter),
        CurrentScenario(None),
    ] {
        let mut world = world(ScenarioRole::OpenWorld);
        world.spawn(PlayerSpaceshipMarker);
        sync(&mut world);
        world.insert_resource(current.clone());

        sync(&mut world);

        assert!(
            armed(&world).is_none(),
            "{:?} left the world armed",
            current.0.map(|s| s.role)
        );
    }
}

/// A player ship that is still spawning, or already destroyed, leaves no
/// observer: the world stands down instead of tripping `nova_world`'s
/// one-observer rule.
#[test]
fn an_open_world_with_no_player_ship_disarms_the_world() {
    let mut world = world(ScenarioRole::OpenWorld);
    let player = world.spawn(PlayerSpaceshipMarker).id();
    sync(&mut world);
    world.despawn(player);

    sync(&mut world);

    assert!(armed(&world).is_none());
}

#[test]
#[should_panic(expected = "no OpenWorldSession")]
fn an_open_world_scenario_with_no_session_is_refused() {
    let mut world = World::new();
    world.insert_resource(scenario(ScenarioRole::OpenWorld));
    world.spawn(PlayerSpaceshipMarker);

    sync(&mut world);
}

#[test]
#[should_panic(expected = "holds 2 player ships")]
fn an_open_world_with_two_player_ships_is_refused() {
    let mut world = world(ScenarioRole::OpenWorld);
    world.spawn(PlayerSpaceshipMarker);
    world.spawn(PlayerSpaceshipMarker);

    sync(&mut world);
}
