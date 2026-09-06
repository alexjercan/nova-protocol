//! A `Grammar` is content, so the authoring rule holds at LOAD as well as at
//! lint: an id it names that no enabled bundle holds is an Error, recorded
//! where the Mods menu reads it.
//!
//! `content lint` walks `assets/` offline and never sees an installed mod, so
//! the offline half alone cannot decide a cross-mod reference. This is the
//! runtime half - the same check, over the MERGED catalog, run by
//! `register_bundles`.

use bevy::{asset::AssetPlugin, ecs::system::RunSystemOnce, prelude::*};
use nova_assets::prelude::*;
use nova_modding::prelude::{
    BundleAsset, CatalogEntry, Content, ContentAsset, InstalledCatalog, ModEntry, ModMeta,
    NovaModdingPlugin,
};
use nova_scenario::prelude::ContentIssues;
use nova_ship::prelude::{
    GameGrammars, GrammarGrid, GrammarKeel, GrammarPart, GrammarVacuum, ShipGrammarConfig,
};

/// A headless app with the modding loaders. Nothing is read off disk: every
/// bundle in these tests is built in memory, and the asset root is only there
/// because the loaders are registered against one.
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
    app.init_resource::<DownloadedMods>();
    app
}

/// A `GameAssets` whose raw handles are all default - `register_bundles` keeps
/// `AssetRef`s as paths and never resolves them.
fn game_assets_with_catalog(catalog: Handle<InstalledCatalog>) -> GameAssets {
    GameAssets {
        cubemap: Handle::default(),
        asteroid_texture: Handle::default(),
        portrait_meridian_control: Handle::default(),
        portrait_deck_chief: Handle::default(),
        portrait_copilot: Handle::default(),
        portrait_engineer: Handle::default(),
        portrait_player: Handle::default(),
        portrait_automated_beacon: Handle::default(),
        portrait_unknown_channel: Handle::default(),
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
    }
}

/// One grammar over unit-cube roles, on the shipped hull's grid.
fn grammar(id: &str, stern_drive: &str) -> ShipGrammarConfig {
    ShipGrammarConfig {
        id: id.to_string(),
        name: id.to_string(),
        grid: GrammarGrid {
            half_width: 4,
            height: 5,
            length: 11,
        },
        vacuum: GrammarVacuum {
            base: 1.0,
            taper: 0.5,
            stern: 2.0,
            bow_taper: 1.0,
        },
        keel: GrammarKeel {
            hull: "cell".to_string(),
            bridge: "cell".to_string(),
            stern_deck: "cell".to_string(),
            stern_drive: stern_drive.to_string(),
            bow_gun: None,
        },
        parts: vec![GrammarPart {
            prototype: "cell".to_string(),
            weight: 1.0,
            aim: None,
            zone: None,
        }],
    }
}

/// One unit-cube hull prototype, the only section these bundles ship.
fn cell_section() -> nova_ship::prelude::SectionConfig {
    nova_ship::prelude::SectionConfig {
        base: nova_ship::prelude::BaseSectionConfig {
            id: "cell".to_string(),
            name: "Cell".to_string(),
            link_points: nova_ship::prelude::unit_cube_link_points(),
            ..default()
        },
        kind: nova_ship::prelude::SectionKind::Hull(default()),
    }
}

/// Merge the given bundles, in order, through the real `register_bundles`.
fn merge(app: &mut App, bundles: Vec<(&str, Vec<Content>)>) {
    let entries: Vec<CatalogEntry> = bundles
        .into_iter()
        .enumerate()
        .map(|(index, (id, content))| {
            let content = app
                .world_mut()
                .resource_mut::<Assets<ContentAsset>>()
                .add(ContentAsset(content));
            let bundle = app
                .world_mut()
                .resource_mut::<Assets<BundleAsset>>()
                .add(BundleAsset {
                    content: vec![content],
                    meta: ModMeta::default(),
                    new_game_scenario: None,
                    resources: vec![],
                    resource_base: format!("mods/{id}"),
                });
            CatalogEntry {
                decl: ModEntry {
                    id: id.to_string(),
                    bundle: format!("{id}/{id}.bundle.ron"),
                    base: index == 0,
                    hidden: false,
                },
                bundle,
            }
        })
        .collect();
    let enabled: std::collections::HashSet<String> =
        entries.iter().map(|entry| entry.decl.id.clone()).collect();
    let catalog = app
        .world_mut()
        .resource_mut::<Assets<InstalledCatalog>>()
        .add(InstalledCatalog { entries });
    app.world_mut()
        .insert_resource(game_assets_with_catalog(catalog));
    app.world_mut().insert_resource(EnabledMods(enabled));
    app.world_mut()
        .run_system_once(nova_assets::register_bundles_for_test)
        .expect("register bundles");
}

/// The reference class this gate exists for, and the one `content lint` cannot
/// decide on its own: mod `parts` ships the drive, mod `hulls` ships a grammar
/// that seeds it. Together they are clean. With `parts` switched off, the
/// grammar names a prototype nothing holds - and that is an Error keyed on the
/// grammar id, not a line the player first sees when a generator is run.
#[test]
fn a_grammar_naming_a_section_no_enabled_bundle_holds_is_an_error_at_load() {
    let together = {
        let mut app = headless_app();
        merge(
            &mut app,
            vec![
                ("base", vec![Content::Section(Box::new(cell_section()))]),
                (
                    "parts",
                    vec![Content::Section(Box::new(
                        nova_ship::prelude::SectionConfig {
                            base: nova_ship::prelude::BaseSectionConfig {
                                id: "borrowed_drive".to_string(),
                                name: "Borrowed Drive".to_string(),
                                link_points: nova_ship::prelude::unit_cube_link_points(),
                                ..default()
                            },
                            ..cell_section()
                        },
                    ))],
                ),
                (
                    "hulls",
                    vec![Content::Grammar(grammar("borrowed_hull", "borrowed_drive"))],
                ),
            ],
        );
        app
    };
    assert!(
        together
            .world()
            .resource::<ContentIssues>()
            .errors("borrowed_hull")
            .is_empty(),
        "with both mods on, the grammar's every id resolves: {:?}",
        together.world().resource::<ContentIssues>().0
    );

    let mut alone = headless_app();
    merge(
        &mut alone,
        vec![
            ("base", vec![Content::Section(Box::new(cell_section()))]),
            (
                "hulls",
                vec![Content::Grammar(grammar("borrowed_hull", "borrowed_drive"))],
            ),
        ],
    );
    let issues = alone.world().resource::<ContentIssues>();
    let errors = issues.errors("borrowed_hull");
    assert_eq!(
        errors.len(),
        1,
        "the grammar is registered against a catalog that no longer holds its \
         drive, so exactly one id fails to resolve: {:?}",
        issues.0
    );
    assert!(
        errors[0].message.contains("borrowed_drive"),
        "the error names the id to fix: {}",
        errors[0].message
    );

    // The grammar is still REGISTERED. Dropping it would silently restore
    // whatever it overlaid, and a hidden fallback is the thing the authoring
    // rule exists to refuse - the finding is the answer, not a substitution.
    assert!(
        alone
            .world()
            .resource::<GameGrammars>()
            .get_grammar("borrowed_hull")
            .is_some(),
        "a refused grammar is reported, not quietly replaced"
    );
}
