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

#[cfg(not(target_arch = "wasm32"))]
use crate::mod_set::load_downloaded_mods;
use crate::{
    collections::{
        fill_ui_font, prepare_cubemap_view, register_sounds, update_nova_hud_assets, BootAssets,
        GameAssets,
    },
    merge::register_bundles,
    mod_set::{
        build_mod_catalog, installed_set_changed, load_enabled_mods,
        mark_downloaded_bundles_loaded, save_enabled_mods, seed_enabled_mods, DownloadedMods,
        EnabledMods, ModCatalog,
    },
    portal,
    reload::{
        raise_reload_cover, reload_content, remerge_on_replaced_content, request_reload_on_key,
        settle_reload, ReloadContent,
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
        app.add_systems(Update, mark_downloaded_bundles_loaded);

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
        app.add_systems(OnEnter(GameAssetsStates::Failed), report_failed_assets);
        // Publish the preloaded UI font once Boot resolves it. Filled at
        // OnExit(Boot) - which runs BEFORE OnEnter(Loading) in the state
        // transition - so `nova_core`'s loading screen, spawned at
        // OnEnter(Loading), always sees UiFont already present and renders its
        // text in the themed Iosevka face from the first frame.
        app.add_systems(OnExit(GameAssetsStates::Boot), fill_ui_font);

        app.add_systems(
            OnEnter(GameAssetsStates::Processing),
            (
                prepare_cubemap_view,
                build_mod_catalog,
                load_enabled_mods,
                seed_enabled_mods,
                register_bundles,
                register_sounds,
                update_nova_hud_assets,
                |mut state: ResMut<NextState<GameAssetsStates>>| {
                    state.set(GameAssetsStates::Loaded);
                },
            )
                .chain(),
        );

        // Re-merge live when the installed set changes in either half, once the
        // catalog is loaded. The condition also fires on the initial inserts,
        // which is harmless (idempotent re-merge); it is skipped while still
        // loading because the catalog is not yet present (register_bundles logs
        // + no-ops).
        app.add_systems(
            Update,
            register_bundles
                .run_if(resource_exists::<GameAssets>)
                .run_if(installed_set_changed)
                .run_if(not(in_state(GameAssetsStates::Loading))),
        );

        // Rebuild the player-facing rows on the same downloaded-set changes, so
        // an install shows up and a loaded bundle's meta replaces its id-only
        // fallback row. EnabledMods changes do not alter the rows, so this one
        // watches only DownloadedMods.
        app.add_systems(
            Update,
            build_mod_catalog
                .run_if(resource_exists::<GameAssets>)
                .run_if(resource_changed::<DownloadedMods>)
                .run_if(not(in_state(GameAssetsStates::Loading))),
        );

        // The reload, the way Wesnoth does it: one key, anywhere, and the
        // content is read off disk again. Chained so a press raises the cover
        // in the frame it happened - the read itself deliberately waits for the
        // frame after, with the panel already on screen.
        app.add_message::<ReloadContent>();
        app.add_systems(
            Update,
            (
                request_reload_on_key,
                raise_reload_cover,
                reload_content,
                settle_reload,
            )
                .chain()
                .run_if(resource_exists::<GameAssets>)
                .run_if(not(in_state(GameAssetsStates::Loading))),
        );
        // The reload's late half: a file that comes back CHANGED rebuilds the
        // registries. Ungated by the installed set, which a re-saved mod does
        // not change.
        app.add_systems(
            Update,
            remerge_on_replaced_content
                .run_if(resource_exists::<GameAssets>)
                .run_if(not(in_state(GameAssetsStates::Loading))),
        );

        // Persist the enabled set whenever it changes (a menu toggle, or the startup
        // seed). Gated the same way as the re-merge so it only fires with the real
        // set present, not during the empty-init on Loading.
        app.add_systems(
            Update,
            save_enabled_mods
                .run_if(resource_exists::<GameAssets>)
                .run_if(resource_changed::<EnabledMods>)
                .run_if(not(in_state(GameAssetsStates::Loading))),
        );
    }
}

/// Say, once and loudly, that the run is over because an asset is missing.
///
/// `bevy_asset_loader` reports WHICH handle failed through its own log, so this
/// does not try to name the file. What it adds is the verdict: the state is
/// terminal, nothing downstream will ever run, and a loading screen that never
/// finishes is this and not a slow disk.
fn report_failed_assets() {
    error!(
        "asset loading FAILED - a path declared in a collection \
         (crates/nova_assets/src/collections.rs) does not resolve under assets/. \
         The bevy_asset_loader errors above name the handle. Nothing past this \
         point runs; the loading screen will not advance."
    );
}
