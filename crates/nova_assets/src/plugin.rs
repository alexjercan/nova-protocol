//! The crate's Bevy wiring: the [`GameAssetsStates`] loading state machine and
//! [`GameAssetsPlugin`], which schedules the mod-cache load, the content merge
//! and the asset-collection gates.

/// Glob-import surface: `use nova_assets::plugin::prelude::*` re-exports the
/// public API of this module.
pub mod prelude {
    pub use super::{GameAssetsPlugin, GameAssetsStates};
}

use bevy::prelude::*;
use bevy_asset_loader::prelude::*;
use nova_gameplay::prelude::GameStates;

#[cfg(not(target_arch = "wasm32"))]
use crate::mod_set::load_downloaded_mods;
use crate::{
    collections::{
        fill_ui_font, prepare_cubemap_view, register_sounds, update_nova_hud_assets, BootAssets,
        GameAssets,
    },
    merge::register_bundles,
    mod_set::{
        build_mod_catalog, installed_bundles_changed, installed_set_changed, load_enabled_mods,
        mark_installed_bundles_loaded, save_enabled_mods, seed_enabled_mods, DownloadedMods,
        EnabledMods, ModCatalog,
    },
    portal,
    reload::{
        remerge_on_replaced_content, request_reload_on_key, restart_for_content, ReloadContent,
    },
    safe_mode::{
        clear_quarantine, optional_loads_settled, quarantine_failed_mods, start_optional_loads,
        FatalAssetFailure, ModQuarantine, OptionalBundles,
    },
};
#[cfg(target_arch = "wasm32")]
use crate::{
    mod_cache,
    mod_set::{poll_mod_cache_hydration, start_mod_cache_hydration, ModCacheHydration},
};

/// Game states for the asset loader.
///
/// Two `bevy_asset_loader` loading states chain across this enum: `Boot` loads
/// the tiny [`BootAssets`] collection (just the UI font) so the boot loading
/// screen can render themed text from its first frame, then continues to
/// `Loading`, which loads the full [`GameAssets`] collection behind that
/// screen. bevy_asset_loader keys its internal schedules per state VALUE, so
/// two loading states on one enum chain cleanly (pinned by
/// `boot_then_loading_collections_gate_in_sequence`).
#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
pub enum GameAssetsStates {
    /// The first frame: load the boot collection ([`BootAssets`] - the UI font)
    /// so the loading screen has a themed typeface before the bulk load starts.
    #[default]
    Boot,
    /// Boot assets ready; the full [`GameAssets`] collection is loading behind
    /// the boot loading screen.
    Loading,
    /// Assets loaded; the content merge/registration is running.
    Processing,
    /// Everything is loaded and registered; gameplay can start.
    Loaded,
    /// A declared asset did not resolve, so the run cannot continue.
    ///
    /// Without this the loader simply never finishes: `bevy_asset_loader` waits
    /// on a handle that will never resolve, the loading screen sits there, and
    /// nothing is logged. One renamed `.glb` under `assets/base/` used to hang
    /// a shipped build forever with no way to tell why. Reaching this state is
    /// always a content bug, never a player's problem.
    Failed,
}

/// A plugin that loads game assets and sets up the game.
///
/// Adds the modding and portal-client plugins, inits the mod-set resources
/// ([`EnabledMods`], [`ModCatalog`], [`DownloadedMods`]), drives the
/// [`GameAssetsStates`] loading state machine, and runs the mod-cache load and
/// content-merge/registration systems across `Startup`/`Update`/`OnEnter`.
pub struct GameAssetsPlugin;

impl Plugin for GameAssetsPlugin {
    fn build(&self, app: &mut App) {
        trace!("GameAssetsPlugin: build");

        // The modding plugin registers the `*.content.ron` asset + loader.
        // Add it before the loading state runs so the loader exists when
        // bevy_asset_loader starts loading the content files below.
        app.add_plugins(nova_modding::prelude::NovaModdingPlugin);
        // The portal client (fetch catalog + install/uninstall over the wire) -
        // event/resource API only; the UI binds later.
        app.add_plugins(portal::PortalPlugin);

        // The enabled-mods set drives which cataloged bundles merge. Seeded from
        // the catalog's base entries at Processing; toggled by the mods menu.
        app.init_resource::<EnabledMods>();
        // The menu-facing installed-mods metadata, filled from the catalog at
        // Processing.
        app.init_resource::<ModCatalog>();
        // The downloaded half of the installed set, from the local mod cache.
        app.init_resource::<DownloadedMods>();
        // The OPTIONAL shipped half: cataloged mods the catalog declares but
        // does not load, so one broken mod cannot fail the boot (see
        // `crate::safe_mode`).
        app.init_resource::<OptionalBundles>();
        // What safe mode switched off, and whether the player has been told.
        app.init_resource::<ModQuarantine>();

        // Read the cache index and kick the mods:// bundle loads. Native reads
        // the filesystem cache directly; the web must first hydrate the
        // memory-backed source from IndexedDB, then poll for completion.
        #[cfg(not(target_arch = "wasm32"))]
        app.add_systems(Startup, load_downloaded_mods);
        #[cfg(target_arch = "wasm32")]
        {
            app.add_systems(
                Startup,
                start_mod_cache_hydration.run_if(resource_exists::<mod_cache::ModsSourceDir>),
            );
            app.add_systems(
                Update,
                poll_mod_cache_hydration.run_if(resource_exists::<ModCacheHydration>),
            );
        }
        // A downloaded bundle finishing its async load must re-trigger the
        // DownloadedMods-gated re-runs below.
        app.add_systems(Update, mark_installed_bundles_loaded);

        // Setup the asset loader. Two chained loading states: Boot loads the
        // tiny BootAssets (UI font) so the loading screen can render themed text
        // from its first frame, then Loading loads the full GameAssets behind
        // that screen. bevy_asset_loader keys its schedules per state VALUE, so
        // two loading states on one enum chain cleanly.
        app.init_state::<GameAssetsStates>();
        // Both states carry a failure exit. A collection declares its paths in
        // Rust while the files live under `assets/`, so the two drift the
        // moment one is renamed - and the drift is not a compile error.
        app.add_loading_state(
            LoadingState::new(GameAssetsStates::Boot)
                .continue_to_state(GameAssetsStates::Loading)
                .on_failure_continue_to_state(GameAssetsStates::Failed)
                .load_collection::<BootAssets>(),
        );
        app.add_loading_state(
            LoadingState::new(GameAssetsStates::Loading)
                .continue_to_state(GameAssetsStates::Processing)
                .on_failure_continue_to_state(GameAssetsStates::Failed)
                .load_collection::<GameAssets>(),
        );
        // A boot and a content restart both pass through `Loading`, and each
        // opens a new recovery episode.
        app.add_systems(OnEnter(GameAssetsStates::Loading), clear_quarantine);
        // Publish the preloaded UI font once Boot resolves it. Filled at
        // OnExit(Boot) - which runs BEFORE OnEnter(Loading) in the state
        // transition - so `nova_core`'s loading screen, spawned at
        // OnEnter(Loading), always sees UiFont already present and renders its
        // text in the themed Iosevka face from the first frame.
        app.add_systems(OnExit(GameAssetsStates::Boot), fill_ui_font);

        // Processing in two halves. The first runs the moment the mandatory
        // collection lands: it restores the enabled set and KICKS the optional
        // mods, which the catalog deliberately did not load.
        app.add_systems(
            OnEnter(GameAssetsStates::Processing),
            (
                prepare_cubemap_view,
                load_enabled_mods,
                seed_enabled_mods,
                start_optional_loads,
            )
                .chain(),
        );
        add_merge_systems(app);

        // Rebuild the player-facing rows whenever the installed FILES change, so
        // an install shows up and a bundle that finished loading replaces its
        // id-only fallback row. EnabledMods changes do not alter the rows, so
        // this one watches the two bundle sets rather than the enabled set.
        app.add_systems(
            Update,
            build_mod_catalog
                .run_if(resource_exists::<GameAssets>)
                .run_if(installed_bundles_changed)
                .run_if(not(in_state(GameAssetsStates::Loading))),
        );

        // The reload, the way Wesnoth does it: one key and the content is read
        // off disk again - but as a RESTART, so everything downstream comes up
        // on one version of it. The key is offered in the main menu only; every
        // other way in is a screen being left.
        app.add_systems(
            Update,
            request_reload_on_key.run_if(in_state(GameStates::MainMenu)),
        );
        // The restart's late half: a MOD's content file is not in the collection
        // the boot load waits on, so one that lands after the merge rebuilds the
        // registries on its own.
        app.add_systems(
            Update,
            remerge_on_replaced_content
                .run_if(resource_exists::<GameAssets>)
                .run_if(not(in_state(GameAssetsStates::Loading))),
        );

        // Persist the enabled set whenever it changes (a menu toggle, or the startup
        // seed). Gated the same way as the re-merge so it only fires with the real
        // set present, not during the empty-init on Loading, and never in
        // `Failed`, which persists no mod change.
        app.add_systems(
            Update,
            save_enabled_mods
                .run_if(resource_exists::<GameAssets>)
                .run_if(resource_changed::<EnabledMods>)
                .run_if(not(in_state(GameAssetsStates::Loading)))
                .run_if(not(in_state(GameAssetsStates::Failed))),
        );
    }
}

/// The merge and its refusal: safe mode's verdict, the `Processing` merge, the
/// live re-merge, and the way into `Failed`.
///
/// Split out of [`GameAssetsPlugin::build`] so a headless test drives these
/// exact schedules; the full plugin needs loaders a test app does not have.
fn add_merge_systems(app: &mut App) {
    // Safe mode's verdict, every frame for the life of the app: a
    // downloaded bundle lands long after boot, and an install done from the
    // Mods screen fails at the moment it is tried. Not in `Failed`, which
    // changes no mod.
    app.add_systems(
        Update,
        quarantine_failed_mods.run_if(not(in_state(GameAssetsStates::Failed))),
    );
    // The second half WAITS for those loads to settle, so the merge sees an
    // enabled optional mod's content and the player is told about a broken
    // one before the menu opens - and a mod that never answers is bounded
    // rather than fatal (`optional_loads_settled`).
    app.add_systems(
        Update,
        (
            build_mod_catalog,
            register_bundles,
            register_sounds,
            update_nova_hud_assets,
            finish_processing.run_if(not(resource_exists::<FatalAssetFailure>)),
        )
            .chain()
            .after(quarantine_failed_mods)
            .run_if(in_state(GameAssetsStates::Processing))
            .run_if(optional_loads_settled),
    );

    // Re-merge live when the installed set changes in either half, once the
    // catalog is loaded. The condition also fires on the initial inserts,
    // which is harmless (idempotent re-merge); it is skipped while still
    // loading because the catalog is not yet present (register_bundles logs
    // + no-ops). A restart's base content can land here after `Processing`,
    // so a refusal on this path ends the run too, as does an invalid or
    // failed mod during a scenario. After safe mode, so a refusal it raised
    // this frame is seen: the merge must not publish content without that
    // mod before the run ends. `Failed` merges nothing.
    app.add_systems(
        Update,
        (
            register_bundles
                .run_if(resource_exists::<GameAssets>)
                .run_if(installed_set_changed)
                .run_if(not(resource_exists::<FatalAssetFailure>))
                .run_if(not(in_state(GameAssetsStates::Loading)))
                .run_if(not(in_state(GameAssetsStates::Failed))),
            fail_refused_content
                .run_if(resource_exists::<FatalAssetFailure>)
                .run_if(not(in_state(GameAssetsStates::Failed))),
        )
            .chain()
            .after(quarantine_failed_mods),
    );
    app.add_systems(OnEnter(GameAssetsStates::Failed), report_failed_assets);

    // The content restart. Not from `Failed`: that run is over until the game
    // is launched again, and leaving a scenario on the way into it still sends
    // a `ReloadContent`.
    app.add_message::<ReloadContent>();
    app.add_systems(
        Update,
        restart_for_content
            .run_if(resource_exists::<GameAssets>)
            .run_if(not(in_state(GameAssetsStates::Loading)))
            .run_if(not(in_state(GameAssetsStates::Failed))),
    );
}

/// End the run when the merge refused the base content, from `Processing` or
/// from a live re-merge, or when the merge or safe mode refused a mod during a
/// scenario.
///
/// Both states, as a boot failure leaves them: `GameStates::Loading` tears
/// down the menu or the scenario, so nothing plays on the last good catalog.
fn fail_refused_content(
    mut assets_state: ResMut<NextState<GameAssetsStates>>,
    mut game_state: ResMut<NextState<GameStates>>,
) {
    assets_state.set(GameAssetsStates::Failed);
    game_state.set(GameStates::Loading);
}

/// Say, once and loudly, that the run is over because an asset is missing.
///
/// `bevy_asset_loader` reports WHICH handle failed through its own log, so this
/// does not try to name the file. What it adds is the verdict: the state is
/// terminal, nothing downstream will ever run, and a loading screen that never
/// finishes is this and not a slow disk.
fn report_failed_assets(
    mut commands: Commands,
    boot: Option<Res<BootAssets>>,
    refused: Option<Res<FatalAssetFailure>>,
) {
    // The merge refused invalid base content and already said why; its detail
    // is the one the screen must show.
    if refused.is_some() {
        return;
    }
    error!(
        "asset loading FAILED - a path declared in a collection \
         (crates/nova_assets/src/collections.rs) does not resolve under assets/. \
         The bevy_asset_loader errors above name the handle. Nothing past this \
         point runs; the loading screen will not advance."
    );
    // Which of the two collections failed is the one thing the state itself
    // does not say, and it decides what the screen can DRAW: without the boot
    // collection there is no UI font, so the report renders in the engine's
    // default face.
    let boot_failed = boot.is_none();
    commands.insert_resource(FatalAssetFailure {
        detail: if boot_failed {
            "The boot assets did not load. The game's own interface font is missing \
             or unreadable."
                .to_string()
        } else {
            "A file the game needs did not load. The installation is incomplete or \
             damaged."
                .to_string()
        },
        boot: boot_failed,
    });
}

/// Leave `Processing` for `Loaded`, once the merge has run.
fn finish_processing(mut state: ResMut<NextState<GameAssetsStates>>) {
    state.set(GameAssetsStates::Loaded);
}

#[cfg(test)]
mod tests {
    use bevy::{asset::AssetPlugin, state::app::StatesPlugin};
    use nova_events::units::prelude::Meters;
    use nova_gameplay::prelude::AssetRef;
    use nova_modding::prelude::{
        BundleAsset, CatalogEntry, Content, ContentAsset, InstalledCatalog, ModEntry, ModMeta,
        NovaModdingPlugin,
    };
    use nova_ship::prelude::*;

    use super::*;

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
                pulse_sound: AssetRef::from("sounds/mining_pulse.wav".to_string()),
                refusal_sound: AssetRef::from("sounds/radar_deny.wav".to_string()),
                door_open_sound: AssetRef::from("sounds/mining_door_open.wav".to_string()),
                door_close_sound: AssetRef::from("sounds/mining_door_close.wav".to_string()),
                reach: Meters(f32::NAN),
                pulse_interval_seconds: 1.0,
                carve_radius_cells: 1.5,
            }),
        }))
    }

    /// A headless app on the production merge schedules, with an in-memory
    /// `base` and one optional mod `extra`, both enabled. Returns the two
    /// content handles, so a test can replace what a restart re-reads.
    fn merge_app(
        base: Vec<Content>,
        extra: Vec<Content>,
    ) -> (App, Handle<ContentAsset>, Handle<ContentAsset>) {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            StatesPlugin,
            AssetPlugin::default(),
            NovaModdingPlugin,
        ));
        // What the sound and HUD steps of the chain read.
        app.init_asset::<Image>();
        app.init_asset::<AudioSource>();
        app.init_resource::<DownloadedMods>();
        app.init_resource::<OptionalBundles>();
        app.init_resource::<ModQuarantine>();
        app.init_resource::<ModCatalog>();
        app.init_state::<GameAssetsStates>();
        app.init_state::<GameStates>();
        add_merge_systems(&mut app);
        // Stands in for `nova_menu`, which restarts the content whenever a
        // scenario is left.
        app.init_resource::<ReloadsSent>();
        app.init_resource::<FailedLeft>();
        app.add_systems(
            OnExit(GameAssetsStates::Failed),
            |mut left: ResMut<FailedLeft>| left.0 += 1,
        );
        app.add_systems(
            OnExit(GameStates::Playing),
            |mut reload: MessageWriter<ReloadContent>, mut sent: ResMut<ReloadsSent>| {
                reload.write(ReloadContent);
                sent.0 += 1;
            },
        );

        let mut entry = |id: &str, base: bool, items: Vec<Content>| {
            let content = app
                .world_mut()
                .resource_mut::<Assets<ContentAsset>>()
                .add(ContentAsset(items));
            let bundle = app
                .world_mut()
                .resource_mut::<Assets<BundleAsset>>()
                .add(BundleAsset {
                    content: vec![content.clone()],
                    meta: ModMeta::default(),
                    new_game_scenario: None,
                    resources: vec![],
                    resource_base: format!("mods/{id}"),
                });
            let entry = CatalogEntry {
                decl: ModEntry {
                    id: id.to_string(),
                    bundle: format!("mods/{id}/{id}.bundle.ron"),
                    base,
                },
                bundle: Some(bundle),
            };
            (entry, content)
        };
        let (base_entry, base_content) = entry("base", true, base);
        let (extra_entry, extra_content) = entry("extra", false, extra);
        let catalog = app
            .world_mut()
            .resource_mut::<Assets<InstalledCatalog>>()
            .add(InstalledCatalog {
                entries: vec![base_entry, extra_entry],
            });
        app.insert_resource(GameAssets {
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
        app.insert_resource(EnabledMods(
            ["base".to_string(), "extra".to_string()].into(),
        ));
        app.world_mut()
            .resource_mut::<NextState<GameAssetsStates>>()
            .set(GameAssetsStates::Processing);
        (app, base_content, extra_content)
    }

    #[derive(Resource, Default)]
    struct ReloadsSent(u32);

    /// How often the run left `Failed`, which is terminal.
    #[derive(Resource, Default)]
    struct FailedLeft(u32);

    fn run_frames(app: &mut App) {
        for _ in 0..4 {
            app.update();
        }
    }

    fn state(app: &App) -> GameAssetsStates {
        app.world()
            .resource::<State<GameAssetsStates>>()
            .get()
            .clone()
    }

    fn section_ids(app: &App) -> Vec<String> {
        app.world()
            .resource::<GameSections>()
            .iter()
            .map(|section| section.base.id.clone())
            .collect()
    }

    /// What a restart does to a content file: new bytes under the same handle,
    /// and the set change `remerge_on_replaced_content` raises for it.
    fn replace(app: &mut App, handle: &Handle<ContentAsset>, items: Vec<Content>) {
        app.world_mut()
            .resource_mut::<Assets<ContentAsset>>()
            .insert(handle.id(), ContentAsset(items))
            .expect("the handle is live");
        app.world_mut()
            .resource_mut::<DownloadedMods>()
            .set_changed();
    }

    #[test]
    fn valid_base_content_reaches_loaded_and_publishes() {
        let (mut app, _, _) = merge_app(vec![hull("hull")], vec![hull("extra_hull")]);
        run_frames(&mut app);

        assert_eq!(state(&app), GameAssetsStates::Loaded);
        assert!(app.world().get_resource::<FatalAssetFailure>().is_none());
        assert_eq!(section_ids(&app), vec!["hull", "extra_hull"]);
    }

    /// The fatal screen reads the merge's own detail, not the missing-file
    /// message `report_failed_assets` writes for a collection that did not
    /// resolve.
    #[test]
    fn invalid_base_content_fails_processing_with_its_own_detail() {
        let (mut app, _, _) = merge_app(vec![hull("hull"), bad_miner("bad_miner")], vec![]);
        run_frames(&mut app);

        assert_eq!(state(&app), GameAssetsStates::Failed);
        let failure = app.world().resource::<FatalAssetFailure>();
        assert!(
            failure.detail.contains("section 'bad_miner'") && !failure.boot,
            "{failure:?}"
        );
        assert!(app.world().get_resource::<GameSections>().is_none());
    }

    /// A restart's base content can land after `Processing`, on the live
    /// re-merge, with the player in the menu or already back in a scenario.
    /// Its refusal ends the run as the boot's does: both states, its own
    /// detail, no mod changed - and `Failed` then neither merges nor restarts.
    fn refuse_live_base_from(game_state: GameStates) -> App {
        let (mut app, base, extra) = merge_app(vec![hull("hull")], vec![hull("extra_hull")]);
        run_frames(&mut app);
        assert_eq!(state(&app), GameAssetsStates::Loaded);
        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(game_state);
        run_frames(&mut app);

        // Both files come back broken in one restart.
        replace(&mut app, &base, vec![hull("hull"), bad_miner("bad_miner")]);
        replace(&mut app, &extra, vec![bad_miner("extra_miner")]);
        run_frames(&mut app);

        assert_eq!(state(&app), GameAssetsStates::Failed);
        assert_eq!(
            *app.world().resource::<State<GameStates>>().get(),
            GameStates::Loading,
            "the menu or scenario is torn down"
        );
        let failure = app.world().resource::<FatalAssetFailure>();
        assert!(
            failure.detail.contains("section 'bad_miner'"),
            "{failure:?}"
        );
        assert!(
            app.world().resource::<ModQuarantine>().disabled.is_empty(),
            "a refused base run quarantines no mod"
        );
        assert!(
            app.world().resource::<EnabledMods>().0.contains("extra"),
            "a refused base run removes no mod from the saved set"
        );

        // A downloaded mod that fails to load now: safe mode leaves it alone.
        let missing = app
            .world()
            .resource::<AssetServer>()
            .load::<BundleAsset>("missing/missing.bundle.ron");
        app.world_mut()
            .resource_mut::<DownloadedMods>()
            .0
            .push(crate::mod_set::DownloadedMod {
                record: crate::mod_cache::InstalledModRecord {
                    id: "late".to_string(),
                    version: "1".to_string(),
                    bundle: "missing.bundle.ron".to_string(),
                },
                bundle: missing.clone(),
            });
        app.world_mut()
            .resource_mut::<EnabledMods>()
            .0
            .insert("late".to_string());
        while !app
            .world()
            .resource::<AssetServer>()
            .load_state(&missing)
            .is_failed()
        {
            app.update();
        }
        run_frames(&mut app);
        assert!(
            app.world().resource::<EnabledMods>().0.contains("late")
                && app.world().resource::<ModQuarantine>().disabled.is_empty(),
            "Failed changes no mod"
        );

        // Fixed files and another trigger: `Failed` does not merge them.
        replace(&mut app, &base, vec![hull("hull"), hull("hull_two")]);
        replace(&mut app, &extra, vec![hull("extra_hull")]);
        run_frames(&mut app);

        assert_eq!(app.world().resource::<FailedLeft>().0, 0, "Failed was left");
        assert!(
            !section_ids(&app).contains(&"hull_two".to_string()),
            "Failed re-merged: {:?}",
            section_ids(&app)
        );
        app
    }

    /// A mod that turns invalid on the live re-merge, with the player in the
    /// menu or in a scenario.
    fn refuse_live_mod_from(game_state: GameStates) -> App {
        let (mut app, _, extra) = merge_app(vec![hull("hull")], vec![hull("extra_hull")]);
        run_frames(&mut app);
        assert_eq!(state(&app), GameAssetsStates::Loaded);
        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(game_state);
        run_frames(&mut app);

        replace(&mut app, &extra, vec![bad_miner("extra_miner")]);
        run_frames(&mut app);
        app
    }

    /// Safe mode cannot take a mod away from a running scenario: the run
    /// ends with the mod named, and nothing about the mods or the published
    /// catalog changes.
    #[test]
    fn an_invalid_mod_in_a_scenario_fails_the_run_and_changes_no_mod() {
        let app = refuse_live_mod_from(GameStates::Playing);

        assert_eq!(state(&app), GameAssetsStates::Failed);
        assert_eq!(
            *app.world().resource::<State<GameStates>>().get(),
            GameStates::Loading,
            "the scenario is torn down"
        );
        let failure = app.world().resource::<FatalAssetFailure>();
        assert!(
            failure.detail.contains("mod 'extra'")
                && failure.detail.contains("section 'extra_miner'")
                && !failure.boot,
            "{failure:?}"
        );
        assert!(app.world().resource::<ModQuarantine>().disabled.is_empty());
        assert!(app.world().resource::<EnabledMods>().0.contains("extra"));
        assert_eq!(section_ids(&app), vec!["hull", "extra_hull"]);
    }

    #[test]
    fn an_invalid_mod_in_the_menu_is_quarantined_and_the_run_continues() {
        let app = refuse_live_mod_from(GameStates::MainMenu);

        assert_eq!(state(&app), GameAssetsStates::Loaded);
        assert!(app.world().get_resource::<FatalAssetFailure>().is_none());
        let quarantine = app.world().resource::<ModQuarantine>();
        assert!(
            quarantine.disabled.iter().any(|m| m.id == "extra"),
            "{:?}",
            quarantine.disabled
        );
        assert!(!app.world().resource::<EnabledMods>().0.contains("extra"));
        assert_eq!(section_ids(&app), vec!["hull"]);
    }

    /// A downloaded mod `late` whose bundle fails to load after `Loaded`,
    /// with `extra` depending on it, in the menu or in a scenario.
    fn fail_live_load_from(game_state: GameStates) -> App {
        let (mut app, _, _) = merge_app(vec![hull("hull")], vec![hull("extra_hull")]);
        run_frames(&mut app);
        assert_eq!(state(&app), GameAssetsStates::Loaded);
        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(game_state);
        run_frames(&mut app);

        let mut bundles = app.world_mut().resource_mut::<Assets<BundleAsset>>();
        let (_, extra) = bundles
            .iter_mut()
            .find(|(_, bundle)| bundle.resource_base == "mods/extra")
            .expect("extra is loaded");
        extra.meta.dependencies = vec!["late".to_string()];
        let missing = app
            .world()
            .resource::<AssetServer>()
            .load::<BundleAsset>("missing/missing.bundle.ron");
        app.world_mut()
            .resource_mut::<DownloadedMods>()
            .0
            .push(crate::mod_set::DownloadedMod {
                record: crate::mod_cache::InstalledModRecord {
                    id: "late".to_string(),
                    version: "1".to_string(),
                    bundle: "missing.bundle.ron".to_string(),
                },
                bundle: missing.clone(),
            });
        app.world_mut()
            .resource_mut::<EnabledMods>()
            .0
            .insert("late".to_string());
        while !app
            .world()
            .resource::<AssetServer>()
            .load_state(&missing)
            .is_failed()
        {
            app.update();
        }
        run_frames(&mut app);
        app
    }

    /// Neither the failed mod nor the mod that depends on it is taken away
    /// from a running scenario: the run ends with the failed mod named.
    #[test]
    fn a_mod_load_failure_in_a_scenario_fails_the_run_and_changes_no_mod() {
        let app = fail_live_load_from(GameStates::Playing);

        assert_eq!(state(&app), GameAssetsStates::Failed);
        assert_eq!(
            *app.world().resource::<State<GameStates>>().get(),
            GameStates::Loading,
            "the scenario is torn down"
        );
        let failure = app.world().resource::<FatalAssetFailure>();
        assert!(
            failure.detail.contains("mod 'late'") && !failure.boot,
            "{failure:?}"
        );
        assert!(app.world().resource::<ModQuarantine>().disabled.is_empty());
        let enabled = &app.world().resource::<EnabledMods>().0;
        assert!(
            enabled.contains("late") && enabled.contains("extra"),
            "{enabled:?}"
        );
        assert_eq!(section_ids(&app), vec!["hull", "extra_hull"]);
    }

    #[test]
    fn a_mod_load_failure_in_the_menu_quarantines_it_and_its_dependent() {
        let app = fail_live_load_from(GameStates::MainMenu);

        assert_eq!(state(&app), GameAssetsStates::Loaded);
        assert!(app.world().get_resource::<FatalAssetFailure>().is_none());
        let disabled: Vec<&str> = app
            .world()
            .resource::<ModQuarantine>()
            .disabled
            .iter()
            .map(|m| m.id.as_str())
            .collect();
        assert_eq!(disabled, vec!["late", "extra"]);
        let enabled = &app.world().resource::<EnabledMods>().0;
        assert!(
            !enabled.contains("late") && !enabled.contains("extra"),
            "{enabled:?}"
        );
        assert_eq!(section_ids(&app), vec!["hull"]);
    }

    /// A download that lands during a scenario and depends on a mod safe
    /// mode disabled earlier this episode is refused by ending the run, not by
    /// taking it off.
    #[test]
    fn a_late_dependent_of_a_disabled_mod_in_a_scenario_fails_the_run() {
        let (mut app, _, _) = merge_app(vec![hull("hull")], vec![bad_miner("extra_miner")]);
        run_frames(&mut app);
        assert_eq!(state(&app), GameAssetsStates::Loaded);
        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::Playing);
        run_frames(&mut app);

        let content = app
            .world_mut()
            .resource_mut::<Assets<ContentAsset>>()
            .add(ContentAsset(vec![hull("late_hull")]));
        let bundle = app
            .world_mut()
            .resource_mut::<Assets<BundleAsset>>()
            .add(BundleAsset {
                content: vec![content],
                meta: ModMeta {
                    dependencies: vec!["extra".to_string()],
                    ..default()
                },
                new_game_scenario: None,
                resources: vec![],
                resource_base: "mods/late".to_string(),
            });
        app.world_mut()
            .resource_mut::<DownloadedMods>()
            .0
            .push(crate::mod_set::DownloadedMod {
                record: crate::mod_cache::InstalledModRecord {
                    id: "late".to_string(),
                    version: "1".to_string(),
                    bundle: "late.bundle.ron".to_string(),
                },
                bundle,
            });
        app.world_mut()
            .resource_mut::<EnabledMods>()
            .0
            .insert("late".to_string());
        run_frames(&mut app);

        assert_eq!(state(&app), GameAssetsStates::Failed);
        assert_eq!(
            *app.world().resource::<State<GameStates>>().get(),
            GameStates::Loading,
            "the scenario is torn down"
        );
        let failure = app.world().resource::<FatalAssetFailure>();
        assert!(
            failure.detail.contains("mod 'late'") && failure.detail.contains("mod 'extra'"),
            "{failure:?}"
        );
        let disabled: Vec<&str> = app
            .world()
            .resource::<ModQuarantine>()
            .disabled
            .iter()
            .map(|m| m.id.as_str())
            .collect();
        assert_eq!(disabled, vec!["extra"], "only the boot's own verdict");
        assert!(app.world().resource::<EnabledMods>().0.contains("late"));
        assert_eq!(section_ids(&app), vec!["hull"]);
    }

    #[test]
    fn a_live_base_refusal_in_the_menu_fails_the_run() {
        let app = refuse_live_base_from(GameStates::MainMenu);
        assert_eq!(app.world().resource::<ReloadsSent>().0, 0);
    }

    /// Leaving the scenario sends a `ReloadContent` on the way into `Failed`,
    /// and it must not restart the load.
    #[test]
    fn a_live_base_refusal_in_a_scenario_fails_the_run_without_a_restart() {
        let mut app = refuse_live_base_from(GameStates::Playing);
        assert_eq!(
            app.world().resource::<ReloadsSent>().0,
            1,
            "the scenario exit asked for a restart"
        );
        app.world_mut().write_message(ReloadContent);
        run_frames(&mut app);
        assert_eq!(state(&app), GameAssetsStates::Failed);
        assert_eq!(
            app.world().resource::<FailedLeft>().0,
            0,
            "a restart left Failed"
        );
    }
}
