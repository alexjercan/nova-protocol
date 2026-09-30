//! The merge's section gate: every section an enabled bundle carries is
//! validated before anything is published. Invalid base content refuses the
//! load; an invalid mod is quarantined whole, with every enabled mod that
//! depends on it, and the merge goes on without them.
//!
//! The base bundle is the real one under `assets/`; the mods are built in
//! memory, so each test names exactly the authoring mistake it proves.

use std::time::{Duration, Instant};

use bevy::{
    asset::{AssetPlugin, RecursiveDependencyLoadState, UntypedAssetId},
    ecs::system::RunSystemOnce,
    prelude::*,
    state::app::StatesPlugin,
};
use nova_assets::prelude::*;
use nova_events::units::prelude::{Meters, Meters3};
use nova_gameplay::prelude::{AssetRef, GameStates};
use nova_modding::prelude::{
    BundleAsset, CatalogEntry, Content, ContentAsset, InstalledCatalog, ModEntry, ModMeta,
    NovaModdingPlugin,
};
use nova_scenario::prelude::{
    BaseScenarioObjectConfig, EventActionConfig, EventConfig, GameScenarios, GameShipDesigns,
    ScenarioConfig, ScenarioEventConfig, ScenarioObjectConfig, ScenarioObjectKind, SectionSource,
    ShipDesign, ShipDesignPrototype, ShipDesignSource, SpaceshipConfig, SpaceshipSectionConfig,
};
use nova_ship::prelude::*;

fn headless_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin {
            file_path: "../../assets".to_string(),
            ..default()
        },
    ));
    app.add_plugins(NovaModdingPlugin);
    // The merge reads the game state; this rig is never in a scenario.
    app.add_plugins(StatesPlugin);
    app.init_state::<GameStates>();
    app.init_resource::<DownloadedMods>();
    app.init_resource::<OptionalBundles>();
    app.init_resource::<ModQuarantine>();
    app
}

/// Pump updates until `handle` is recursively loaded, panicking on failure/timeout.
fn wait_recursive_loaded(app: &mut App, server: &AssetServer, handle: UntypedAssetId, what: &str) {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        app.update();
        match server.get_recursive_dependency_load_state(handle) {
            Some(RecursiveDependencyLoadState::Loaded) => break,
            Some(RecursiveDependencyLoadState::Failed(err)) => panic!("{what} failed: {err}"),
            _ => {}
        }
        assert!(Instant::now() < deadline, "timed out loading {what}");
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// The shipped catalog's `base` entry, its bundle loaded.
fn shipped_base(app: &mut App) -> CatalogEntry {
    let server = app.world().resource::<AssetServer>().clone();
    let catalog: Handle<InstalledCatalog> = server.load("mods.catalog.ron");
    wait_recursive_loaded(app, &server, catalog.id().untyped(), "the mods catalog");
    app.world()
        .resource::<Assets<InstalledCatalog>>()
        .get(&catalog)
        .expect("catalog loaded")
        .entries
        .iter()
        .find(|entry| entry.decl.base)
        .expect("the catalog declares base")
        .clone()
}

/// One in-memory bundle, cataloged under `id`.
fn bundle_entry(
    app: &mut App,
    id: &str,
    base: bool,
    dependencies: &[&str],
    items: Vec<Content>,
) -> CatalogEntry {
    let content = app
        .world_mut()
        .resource_mut::<Assets<ContentAsset>>()
        .add(ContentAsset(items));
    let bundle = app
        .world_mut()
        .resource_mut::<Assets<BundleAsset>>()
        .add(BundleAsset {
            content: vec![content],
            meta: ModMeta {
                dependencies: dependencies.iter().map(ToString::to_string).collect(),
                ..default()
            },
            new_game_scenario: None,
            resources: vec![],
            resource_base: format!("mods/{id}"),
        });
    CatalogEntry {
        decl: ModEntry {
            id: id.to_string(),
            bundle: format!("mods/{id}/{id}.bundle.ron"),
            base,
        },
        bundle: Some(bundle),
    }
}

/// Run the production merge once over `entries` with every entry enabled.
fn merge(app: &mut App, entries: Vec<CatalogEntry>) {
    let enabled = entries.iter().map(|entry| entry.decl.id.clone()).collect();
    let catalog = app
        .world_mut()
        .resource_mut::<Assets<InstalledCatalog>>()
        .add(InstalledCatalog { entries });
    app.world_mut().insert_resource(GameAssets {
        cubemap: Handle::default(),
        asteroid_texture: Handle::default(),
        hull_01: Handle::default(),
        turret_yaw_01: Handle::default(),
        turret_pitch_01: Handle::default(),
        turret_barrel_01: Handle::default(),
        torpedo_bay_01: Handle::default(),
        fps_icon: Handle::default(),
        target_sprite: Handle::default(),
        nova_crt_mark: Handle::default(),
        ui_sfx: Default::default(),
        key_glyphs: Default::default(),
        catalog,
    });
    app.world_mut().insert_resource(EnabledMods(enabled));
    app.world_mut()
        .run_system_once(nova_assets::register_bundles_for_test)
        .expect("register bundles");
}

/// A mining section with a NaN reach, no collider and neither stow track.
fn bad_miner(id: &str) -> Content {
    Content::Section(Box::new(SectionConfig {
        base: BaseSectionConfig {
            id: id.to_string(),
            health: 10.0,
            ..default()
        },
        kind: SectionKind::Mining(MiningSectionConfig {
            render_mesh: AssetRef::from("gltf/mining_beam_compact.glb#Scene0".to_string()),
            render_mesh_transform: None,
            reach: Meters(f32::NAN),
            pulse_interval_seconds: 1.0,
            carve_radius_cells: 1.5,
        }),
    }))
}

/// A well-formed hull section.
fn hull(id: &str) -> Content {
    Content::Section(Box::new(SectionConfig {
        base: BaseSectionConfig {
            id: id.to_string(),
            health: 100.0,
            ..default()
        },
        kind: SectionKind::Hull(HullSectionConfig::default()),
    }))
}

/// A catalog ship built on a hull and one prototype section.
fn ship_on(id: &str, hull_id: &str, section_id: &str) -> Content {
    let placed = |id: &str, z: f32| SpaceshipSectionConfig {
        id: id.into(),
        position: Vec3::new(0.0, 0.0, z),
        rotation: Quat::IDENTITY,
        source: SectionSource::prototype(id),
    };
    Content::Ship(ShipDesignPrototype {
        id: id.into(),
        name: id.to_string(),
        design: ShipDesign {
            sections: vec![placed(hull_id, 0.0), placed(section_id, -1.0)],
            ..default()
        },
    })
}

/// A scenario that spawns the catalog ship `ship` on start.
fn scenario_spawning(id: &str, ship: &str) -> Content {
    Content::Scenario(ScenarioConfig {
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: true,
            filters: vec![],
            actions: vec![EventActionConfig::SpawnScenarioObject(
                ScenarioObjectConfig {
                    base: BaseScenarioObjectConfig {
                        id: "miner".to_string(),
                        name: "Miner".to_string(),
                        position: Meters3::ZERO,
                        rotation: Quat::IDENTITY,
                    },
                    kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
                        design: ShipDesignSource::Prototype {
                            id: ship.into(),
                            section_patches: default(),
                        },
                        ..default()
                    }),
                },
            )],
        }],
        ..ScenarioConfig::new(
            id.to_string(),
            id.to_string(),
            AssetRef::from("dep://base/textures/cubemap.png".to_string()),
        )
    })
}

fn section_ids(app: &App) -> Vec<String> {
    app.world()
        .resource::<GameSections>()
        .iter()
        .map(|section| section.base.id.clone())
        .collect()
}

fn disabled(app: &App) -> Vec<(String, String)> {
    app.world()
        .resource::<ModQuarantine>()
        .disabled
        .iter()
        .map(|m| (m.id.clone(), m.reason.clone()))
        .collect()
}

#[test]
fn an_invalid_unused_base_section_refuses_the_load() {
    let mut app = headless_app();
    let base = bundle_entry(
        &mut app,
        "base",
        true,
        &[],
        vec![hull("hull"), bad_miner("bad_miner")],
    );
    merge(&mut app, vec![base]);

    let fatal = app
        .world()
        .get_resource::<FatalAssetFailure>()
        .expect("invalid base content refuses the load");
    assert!(
        fatal.detail.contains("bad_miner") && !fatal.boot,
        "the refusal names the section: {fatal:?}"
    );
    assert!(
        app.world().get_resource::<GameSections>().is_none(),
        "a refused base publishes no section at all, the valid hull included"
    );
    assert!(
        disabled(&app).is_empty(),
        "base is refused, never quarantined"
    );
}

#[test]
fn an_invalid_unused_mod_section_quarantines_the_whole_mod() {
    let mut app = headless_app();
    let base = shipped_base(&mut app);
    let badmine = bundle_entry(
        &mut app,
        "badmine",
        false,
        &[],
        vec![hull("badmine_hull"), bad_miner("bad_miner")],
    );
    merge(&mut app, vec![base, badmine]);

    assert!(app.world().get_resource::<FatalAssetFailure>().is_none());
    let sections = section_ids(&app);
    assert!(
        sections.iter().any(|id| id == "mining_beam_section"),
        "base still merges"
    );
    assert!(
        !sections
            .iter()
            .any(|id| id == "bad_miner" || id == "badmine_hull"),
        "no section of the refused mod is published, its valid hull included: {sections:?}"
    );
    let disabled = disabled(&app);
    assert_eq!(disabled.len(), 1, "{disabled:?}");
    assert_eq!(disabled[0].0, "badmine");
    assert!(
        disabled[0].1.contains("section 'bad_miner'") && disabled[0].1.contains("more"),
        "the reason names the section and counts the rest: {}",
        disabled[0].1
    );
    assert!(!app.world().resource::<EnabledMods>().0.contains("badmine"));
}

#[test]
fn an_invalid_mod_overlay_keeps_the_base_section() {
    let mut app = headless_app();
    let base = shipped_base(&mut app);
    // The shipped emitter, StowDoors and StowLift tracks and all, with only
    // its reach broken - so the one finding is the reach.
    let shipped = {
        let bundles = app.world().resource::<Assets<BundleAsset>>();
        let contents = app.world().resource::<Assets<ContentAsset>>();
        let bundle = bundles.get(base.bundle.as_ref().unwrap()).unwrap();
        bundle
            .content
            .iter()
            .filter_map(|handle| contents.get(handle))
            .flat_map(|content| content.0.iter())
            .find_map(|item| match item {
                Content::Section(config) if config.base.id == "mining_beam_section" => {
                    Some(config.as_ref().clone())
                }
                _ => None,
            })
            .expect("base ships mining_beam_section")
    };
    let SectionKind::Mining(shipped_mining) = &shipped.kind else {
        panic!("mining_beam_section is a Mining section");
    };
    let shipped_reach = shipped_mining.reach;
    let mut overlay = shipped.clone();
    let SectionKind::Mining(mining) = &mut overlay.kind else {
        unreachable!()
    };
    mining.reach = Meters(-1.0);
    let rework = bundle_entry(
        &mut app,
        "rework",
        false,
        &[],
        vec![Content::Section(Box::new(overlay))],
    );
    merge(&mut app, vec![base, rework]);

    assert!(app.world().get_resource::<FatalAssetFailure>().is_none());
    let effective = app
        .world()
        .resource::<GameSections>()
        .iter()
        .find(|section| section.base.id == "mining_beam_section")
        .expect("the base section stays");
    let SectionKind::Mining(mining) = &effective.kind else {
        panic!("still a Mining section");
    };
    assert_eq!(
        mining.reach, shipped_reach,
        "the base config is the effective one"
    );
    let disabled = disabled(&app);
    assert_eq!(disabled.len(), 1, "{disabled:?}");
    assert_eq!(disabled[0].0, "rework", "the overlay is charged to the mod");
    assert!(
        disabled[0].1.contains("mining_beam_section"),
        "{}",
        disabled[0].1
    );
}

#[test]
fn a_catalog_ship_on_an_invalid_section_leaves_with_its_mod() {
    let mut app = headless_app();
    let base = shipped_base(&mut app);
    let badmine = bundle_entry(
        &mut app,
        "badmine",
        false,
        &[],
        vec![
            hull("badmine_hull"),
            bad_miner("bad_miner"),
            ship_on("bad_mining_ship", "badmine_hull", "bad_miner"),
            scenario_spawning("bad_mining_run", "bad_mining_ship"),
        ],
    );
    merge(&mut app, vec![base, badmine]);

    assert!(
        !app.world()
            .resource::<GameShipDesigns>()
            .iter()
            .any(|ship| ship.id.as_str() == "bad_mining_ship"),
        "the ship leaves with its mod"
    );
    assert!(
        !app.world()
            .resource::<GameScenarios>()
            .contains_key("bad_mining_run"),
        "no scenario remains to spawn it"
    );
    assert_eq!(disabled(&app).len(), 1);
}

#[test]
fn a_mod_depending_on_a_refused_mod_is_refused_too() {
    let mut app = headless_app();
    let base = shipped_base(&mut app);
    let badmine = bundle_entry(
        &mut app,
        "badmine",
        false,
        &[],
        vec![hull("badmine_hull"), bad_miner("bad_miner")],
    );
    // Valid on its own. Its ship names the refused mod's sections, and
    // without them it would resolve to a partial hull.
    let fleet = bundle_entry(
        &mut app,
        "fleet",
        false,
        &["badmine"],
        vec![
            ship_on("fleet_miner", "badmine_hull", "bad_miner"),
            scenario_spawning("fleet_run", "fleet_miner"),
        ],
    );
    let bystander = bundle_entry(
        &mut app,
        "bystander",
        false,
        &[],
        vec![hull("bystander_hull")],
    );
    merge(&mut app, vec![base, badmine, fleet, bystander]);

    let disabled = disabled(&app);
    assert_eq!(
        disabled
            .iter()
            .map(|(id, _)| id.as_str())
            .collect::<Vec<_>>(),
        vec!["badmine", "fleet"],
        "{disabled:?}"
    );
    assert!(disabled[1].1.contains("'badmine'"), "{}", disabled[1].1);
    let enabled = &app.world().resource::<EnabledMods>().0;
    assert!(!enabled.contains("fleet") && enabled.contains("bystander"));
    assert!(!app
        .world()
        .resource::<GameShipDesigns>()
        .iter()
        .any(|ship| ship.id.as_str() == "fleet_miner"));
    assert!(!app
        .world()
        .resource::<GameScenarios>()
        .contains_key("fleet_run"));
    assert!(
        section_ids(&app).iter().any(|id| id == "bystander_hull"),
        "an unrelated mod still merges"
    );
}

#[test]
fn the_shipped_base_content_passes_the_section_gate() {
    let mut app = headless_app();
    let base = shipped_base(&mut app);
    merge(&mut app, vec![base]);

    assert!(app.world().get_resource::<FatalAssetFailure>().is_none());
    assert!(disabled(&app).is_empty(), "{:?}", disabled(&app));
    assert!(section_ids(&app)
        .iter()
        .any(|id| id == "mining_beam_section"));
}
