//! `nova_assets` loads the game's assets and assembles its content. It sets up
//! `bevy_asset_loader` (glTF, textures, shaders, sounds), registers the
//! built-in sections and scenarios, and performs the MOD MERGE - resolving the
//! installed-mods catalog and `EnabledMods` into the active section/scenario
//! set. The base content it registers is GENERATED from Rust builders (edit the
//! builder, not the committed `*.content.ron`), and the crate also backs the
//! `content` CLI (`gen`/`lint`; the balance audit and input-overlap check are
//! folded into `lint`) that authors and validates that content.
//!
//! Static assets PRELOAD through `bevy_asset_loader` collections: the UI font in
//! the `Boot` [`BootAssets`] collection (published as [`nova_ui::font::UiFont`]),
//! and textures, meshes, the shared HUD art and the UI SFX in the `Loading`
//! [`GameAssets`] collection - so everything is load-gated before gameplay
//! starts rather than fetched lazily at first use. Scenario-authored `AssetRef`s
//! and downloaded `mods://` bundles are the DYNAMIC exceptions: their paths vary
//! at runtime, so they cannot sit behind a one-shot collection and stay on the
//! asset server by design.
#![warn(missing_docs)]

mod collections;
mod merge;
mod mod_set;
mod plugin;
mod scenario;
mod sections;

pub mod content_report;
pub mod mod_cache;
pub mod mod_prefs;
pub mod mod_refs;
pub mod portal;

/// The balance audit over shipped scenario content: derives fairness metrics
/// from the authored data and grades them. Shared by the `content` CLI's `lint`
/// subcommand and the CI gate test; not part of the game's public API.
#[doc(hidden)]
pub mod balance;

/// The RON generation surface for the built-in scenarios - the source the
/// `content gen` CLI writes and the `content_ron_parity` test asserts. Not part
/// of the game's public API.
#[doc(hidden)]
pub mod scenario_generation;

pub mod lint_walk;

pub use collections::{BootAssets, GameAssets};
pub use merge::{merge_bundles, register_bundles, MergeOutcome};
#[cfg(not(target_arch = "wasm32"))]
pub use mod_set::load_downloaded_mods;
pub use mod_set::{
    build_mod_catalog, installed_set_changed, load_enabled_mods, mark_downloaded_bundles_loaded,
    save_enabled_mods, seed_enabled_mods, DownloadedMod, DownloadedMods, EnabledMods, ModCatalog,
    ModInfo,
};
#[cfg(target_arch = "wasm32")]
pub use mod_set::{poll_mod_cache_hydration, start_mod_cache_hydration, ModCacheHydration};
pub use plugin::{GameAssetsPlugin, GameAssetsStates};

/// Glob-import surface: `use nova_assets::prelude::*` brings the loaded-asset
/// resources, the mod-set/portal types, and [`GameAssetsPlugin`] into scope.
pub mod prelude {
    pub use nova_mod_format::{PortalCatalog, PortalEntry};
    pub use nova_modding::prelude::ModMeta;

    pub use super::{
        portal::{
            FetchPortalCatalog, InstallJobs, InstallPortalMod, InstallStatus, PendingRemovals,
            PortalConfig, PortalFetchTimeout, RemoteCatalog, RemoteCatalogState,
            UninstallPortalMod,
        },
        DownloadedMod, DownloadedMods, EnabledMods, GameAssets, GameAssetsPlugin, GameAssetsStates,
        ModCatalog, ModInfo,
    };
}

/// The production `register_bundles` system, re-exported for the crate's
/// integration tests (which drive the RON modding pipeline end to end: load the
/// base bundle + its content files and route their items into `GameSections` /
/// `GameScenarios`). Not part of the public API.
#[doc(hidden)]
pub use crate::register_bundles as register_bundles_for_test;
