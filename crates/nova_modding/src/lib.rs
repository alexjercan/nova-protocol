//! RON scenario/mod format for Nova Protocol.
//!
//! This crate is the authoring surface of the modding language. It loads a
//! single uniform content format:
//! - `*.content.ron` -> [`ContentAsset`], a RON `Vec<`[`Content`]`>` where each
//!   item carries its KIND as a data flag (`Section((..))` / `Scenario((..))`).
//!
//! A [`Content`] item is one of:
//! - [`Content::Section`] - a [`SectionConfig`](nova_gameplay::prelude::SectionConfig)
//!   prototype (previously the `*.sections.ron` catalog), and
//! - [`Content::Scenario`] - a [`ScenarioConfig`](nova_scenario::prelude::ScenarioConfig)
//!   (previously the `*.scenario.ron` file).
//!
//! The kind lives IN the RON structure (an externally-tagged enum), so ONE
//! loader reads any content file and a downstream router (`nova_assets`'s
//! `register_bundles`) dispatches each item into its id-keyed registry
//! (`GameSections` / `GameScenarios`). A single file may mix kinds.
//!
//! The config trees are `serde` under nova_scenario's / nova_gameplay's `serde`
//! features (which this crate enables), so the loader is a pure RON decode.
//!
//! Asset references inside the configs (section render meshes, particle effects,
//! the skybox cubemap, asteroid textures) are authored as paths and stay as paths
//! in the loaded config - each is an
//! [`AssetRef`](nova_gameplay::prelude::AssetRef) that resolves to a live `Handle`
//! lazily at spawn time through the `AssetServer`. So the loader does not touch the
//! `AssetServer`/`LoadContext`; it just deserializes.
//!
//! Downstream (`nova_assets`) drives the actual load of `assets/**/*.content.ron`
//! and routes each item into `GameScenarios` / `GameSections`.

use bevy::{
    asset::{
        io::Reader, Asset, AssetLoader, AssetPath, LoadContext, UntypedAssetId,
        VisitAssetDependencies,
    },
    prelude::*,
    reflect::TypePath,
};
use nova_gameplay::prelude::SectionConfig;
use nova_scenario::prelude::ScenarioConfig;
use serde::{Deserialize, Serialize};

pub mod prelude {
    pub use super::{
        BundleAsset, BundleAssetLoader, BundleManifest, CatalogEntry, CatalogLoader,
        CatalogManifest, Content, ContentAsset, ContentAssetLoader, InstalledCatalog, ModEntry,
        ModMeta, ModdingLoaderError, NovaModdingPlugin,
    };
}

/// A single piece of authored content, with its KIND as a data flag.
///
/// A content file is a RON `Vec<Content>`; this externally-tagged enum makes the
/// kind explicit in the data (`Section((..))` / `Scenario((..))`) so one loader
/// reads any file and a router dispatches each item into its registry. Adding a
/// kind is one variant here plus one router arm downstream - no new loader or
/// asset type.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Content {
    /// A section-prototype ([`SectionConfig`]) the ships reference by id -
    /// registers into `GameSections`.
    Section(SectionConfig),
    /// A [`ScenarioConfig`] - registers into `GameScenarios` keyed by its id.
    Scenario(ScenarioConfig),
}

/// The content of one `*.content.ron` file: a thin [`Asset`] wrapper around a
/// `Vec` of [`Content`] items.
///
/// [`Asset`] is implemented by hand rather than derived: the derive would try to
/// walk the wrapped configs for `Handle` dependencies, but asset references are
/// [`AssetRef`](nova_gameplay::prelude::AssetRef) paths that resolve lazily at
/// spawn, so this asset declares no dependencies of its own.
#[derive(TypePath, Clone, Debug)]
pub struct ContentAsset(pub Vec<Content>);

impl VisitAssetDependencies for ContentAsset {
    fn visit_dependencies(&self, _visit: &mut impl FnMut(UntypedAssetId)) {}
}

impl Asset for ContentAsset {}

/// A mod's SELF-DESCRIPTION, authored in its `*.bundle.ron` manifest - the
/// Factorio `info.json` analog and the single source of truth for mod metadata
/// (the menu list, the details panel, and the mod portal all read it).
///
/// Every field is optional (serde-defaulted) so a bare `(content: [...])`
/// manifest stays valid. Conventions: `version` is an opaque semver-ish string
/// (empty = unversioned; the base game leaves it empty, the GAME version is
/// authoritative there); `dependencies` lists mod ids - `base` is an IMPLICIT
/// dependency and is not declared (resolution is task 20260715-142931);
/// `icon`/`screenshots` are paths relative to the bundle's directory, reserved
/// for the mod portal and the details panel.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ModMeta {
    /// Display name for the mods list (falls back to the catalog id when empty).
    #[serde(default)]
    pub name: String,
    /// One-line description for the mods list / details panel.
    #[serde(default)]
    pub description: String,
    /// Author credit, shown in the details panel.
    #[serde(default)]
    pub author: String,
    /// Opaque version string (semver-ish); empty = unversioned.
    #[serde(default)]
    pub version: String,
    /// Ids of mods this one needs (schema-only for now; `base` is implicit).
    #[serde(default)]
    pub dependencies: Vec<String>,
    /// Icon image path, relative to the bundle directory (reserved).
    #[serde(default)]
    pub icon: Option<String>,
    /// Screenshot image paths, relative to the bundle directory (reserved).
    #[serde(default)]
    pub screenshots: Vec<String>,
}

/// The on-disk `*.bundle.ron` manifest: the list of content files a bundle
/// folder packages, as paths RELATIVE to the manifest file's own directory,
/// plus the mod's [`ModMeta`] self-description.
///
/// A bundle is a DIRECTORY plus this manifest - the manifest, not directory
/// enumeration, is what makes bundles wasm-safe (`load_folder` is broken on the
/// web target, so a bundle can never rely on listing its directory).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BundleManifest {
    /// Content-file paths, relative to the `bundle.ron` file's directory (e.g.
    /// `"sections/base.content.ron"`, `"scenarios/demo.content.ron"`).
    pub content: Vec<String>,
    /// The mod's self-description; defaulted so meta-less manifests stay valid.
    #[serde(default)]
    pub meta: ModMeta,
}

/// A loaded bundle: the [`ContentAsset`] handles for every content file its
/// [`BundleManifest`] listed, in manifest order, plus the manifest's [`ModMeta`].
///
/// Unlike [`ContentAsset`] (a leaf with no dependencies), a bundle HAS
/// dependencies - its content files. [`Asset`] and [`VisitAssetDependencies`]
/// are implemented by hand so that `visit_dependencies` reports every content
/// handle: this is what tells bevy to load the content along with the bundle and
/// to only report the bundle's RECURSIVE load state as `Loaded` once all its
/// content has loaded.
#[derive(TypePath, Clone, Debug)]
pub struct BundleAsset {
    /// One handle per content file the manifest listed, in manifest order.
    pub content: Vec<Handle<ContentAsset>>,
    /// The mod's self-description, carried from the manifest.
    pub meta: ModMeta,
}

impl VisitAssetDependencies for BundleAsset {
    fn visit_dependencies(&self, visit: &mut impl FnMut(UntypedAssetId)) {
        for handle in &self.content {
            visit(handle.id().untyped());
        }
    }
}

impl Asset for BundleAsset {}

/// Errors produced while loading a modding RON asset (`*.content.ron`).
#[derive(Debug)]
pub enum ModdingLoaderError {
    /// The file could not be read.
    Io(std::io::Error),
    /// The bytes were not valid RON for the expected type.
    Ron(ron::error::SpannedError),
}

impl std::fmt::Display for ModdingLoaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModdingLoaderError::Io(err) => write!(f, "failed to read modding file: {err}"),
            ModdingLoaderError::Ron(err) => write!(f, "failed to parse modding RON: {err}"),
        }
    }
}

impl std::error::Error for ModdingLoaderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ModdingLoaderError::Io(err) => Some(err),
            ModdingLoaderError::Ron(err) => Some(err),
        }
    }
}

impl From<std::io::Error> for ModdingLoaderError {
    fn from(err: std::io::Error) -> Self {
        ModdingLoaderError::Io(err)
    }
}

impl From<ron::error::SpannedError> for ModdingLoaderError {
    fn from(err: ron::error::SpannedError) -> Self {
        ModdingLoaderError::Ron(err)
    }
}

/// Bevy [`AssetLoader`] for `*.content.ron` files (a RON `Vec<`[`Content`]`>`).
#[derive(Default, TypePath)]
pub struct ContentAssetLoader;

impl AssetLoader for ContentAssetLoader {
    type Asset = ContentAsset;
    type Settings = ();
    type Error = ModdingLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let items: Vec<Content> = ron::de::from_bytes(&bytes)?;
        Ok(ContentAsset(items))
    }

    fn extensions(&self) -> &[&str] {
        &["content.ron"]
    }
}

/// Bevy [`AssetLoader`] for `*.bundle.ron` files (a RON [`BundleManifest`]).
///
/// Decodes the manifest, then for each listed content path issues a
/// `load_context.load::<ContentAsset>` and collects the handles into a
/// [`BundleAsset`]. The manifest paths are resolved RELATIVE to the bundle
/// file's own directory (via [`AssetPath::resolve`] against the bundle path's
/// parent), so a bundle folder is self-contained and relocatable.
///
/// NAMING: a bundle manifest MUST be named `<pack>.bundle.ron` (e.g.
/// `base.bundle.ron`), never a bare `bundle.ron`. bevy resolves an UNTYPED load
/// (which is how `bevy_asset_loader` kicks off collection fields) by the file's
/// FULL extension - everything after the FIRST dot. `bundle.ron` yields the bare
/// `ron` extension (this loader is registered for `bundle.ron`, so it would not
/// match, and the load fails with "Could not find an asset loader"); a stemmed
/// `base.bundle.ron` yields `bundle.ron` and matches. See task 20260714-163342.
#[derive(Default, TypePath)]
pub struct BundleAssetLoader;

impl AssetLoader for BundleAssetLoader {
    type Asset = BundleAsset;
    type Settings = ();
    type Error = ModdingLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let manifest: BundleManifest = ron::de::from_bytes(&bytes)?;

        // Resolve each content path against the bundle file's DIRECTORY so the
        // manifest paths are bundle-relative (self-contained folder). `path()`
        // is the bundle file itself (e.g. `base/base.bundle.ron`); its parent is the
        // bundle dir (e.g. `base`), and `resolve` joins the relative content
        // path onto it.
        let base = load_context
            .path()
            .parent()
            .unwrap_or_else(|| AssetPath::from(""));

        let content = manifest
            .content
            .iter()
            .map(|rel| {
                // `to_string` (owned) is load-bearing here, not a smell: an
                // `AssetPath::from(&str)` would borrow `manifest.content`, which does
                // not outlive the resolved path.
                let resolved = base.resolve(&AssetPath::from(rel.to_string()));
                load_context.load::<ContentAsset>(resolved)
            })
            .collect();

        Ok(BundleAsset {
            content,
            meta: manifest.meta,
        })
    }

    fn extensions(&self) -> &[&str] {
        &["bundle.ron"]
    }
}

/// One INSTALLED mod's catalog DECLARATION: identity + where to find it +
/// deployment flags. A thin pointer, not metadata - the mod's self-description
/// (name, author, version, ...) lives in its own bundle's [`ModMeta`].
///
/// `id` is what `register_bundles` keys enable/disable on and the merge-overlay
/// namespace. `bundle` is the mod's `*.bundle.ron` manifest path, RELATIVE to the
/// asset root (the catalog lives at the root). `base` marks the base game's own
/// entry - enabled by default and (in the UI) locked on.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModEntry {
    /// Stable id - the enable/disable key and the merge-overlay namespace.
    pub id: String,
    /// The mod's `*.bundle.ron` manifest path, asset-root-relative.
    pub bundle: String,
    /// True for the base game's entry: enabled by default, locked on in the UI.
    #[serde(default)]
    pub base: bool,
    /// True for dev/tooling mods (e.g. the screenshot-reel capture set): omitted
    /// from the player-facing mods list, but still installed - the bundle loads
    /// and the mod is enableable by id (examples do so via `EnabledMods`).
    #[serde(default)]
    pub hidden: bool,
}

/// The on-disk `mods.catalog.ron`: every INSTALLED mod, in load order (base first).
///
/// This is the wasm-safe source of truth for what is installed - a manifest, never
/// directory enumeration (`load_folder` is broken on the web target). Which entries
/// are ENABLED is a separate runtime concern (`nova_assets::EnabledMods`), not baked
/// into this read-only asset.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CatalogManifest {
    /// Installed mods, in load order. `base` should come first so it merges first.
    pub mods: Vec<ModEntry>,
}

/// One loaded catalog entry: a mod's [`ModEntry`] declaration paired with the
/// loaded handle for its [`BundleAsset`].
#[derive(Clone, Debug)]
pub struct CatalogEntry {
    /// The catalog declaration (id, bundle path, base/hidden flags).
    pub decl: ModEntry,
    /// The loaded handle for this mod's bundle.
    pub bundle: Handle<BundleAsset>,
}

/// A loaded installed-mods catalog: every installed mod's declaration + bundle
/// handle, in catalog (load) order.
///
/// Like [`BundleAsset`] one level up, an `InstalledCatalog` HAS dependencies - the
/// bundle of EVERY installed mod - so [`Asset`] and [`VisitAssetDependencies`] are
/// hand-implemented to visit each entry's bundle handle. That makes bevy load every
/// installed bundle (and, through each, its content) along with the catalog, and
/// report the catalog's RECURSIVE load state as `Loaded` only once all of it has
/// loaded - so the merge sees fully-loaded bundles regardless of which are enabled.
#[derive(TypePath, Clone, Debug)]
pub struct InstalledCatalog {
    /// One entry per installed mod, in catalog order.
    pub entries: Vec<CatalogEntry>,
}

impl VisitAssetDependencies for InstalledCatalog {
    fn visit_dependencies(&self, visit: &mut impl FnMut(UntypedAssetId)) {
        for entry in &self.entries {
            visit(entry.bundle.id().untyped());
        }
    }
}

impl Asset for InstalledCatalog {}

/// Bevy [`AssetLoader`] for `mods.catalog.ron` files (a RON [`CatalogManifest`]).
///
/// Decodes the manifest, then for each installed mod issues a
/// `load_context.load::<BundleAsset>` (the paths are asset-root-relative) and pairs
/// each handle with its declaration into an [`InstalledCatalog`]. Mirrors
/// [`BundleAssetLoader`] one level up (a catalog of bundles instead of a bundle of
/// content).
///
/// NAMING: same rule as bundles - the catalog MUST be named `<name>.catalog.ron`
/// (e.g. `mods.catalog.ron`), never a bare `catalog.ron`. bevy_asset_loader loads it
/// UNTYPED (as a `GameAssets` field), which resolves the loader by the file's full
/// extension - everything after the FIRST dot. A single-dot name yields the bare
/// `ron` extension (no loader, fails in-game); `mods.catalog.ron` yields
/// `catalog.ron` and matches. See task 20260714-163342.
#[derive(Default, TypePath)]
pub struct CatalogLoader;

impl AssetLoader for CatalogLoader {
    type Asset = InstalledCatalog;
    type Settings = ();
    type Error = ModdingLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let manifest: CatalogManifest = ron::de::from_bytes(&bytes)?;

        // Bundle paths are asset-root-relative (the catalog lives at the root), so
        // they load as-is - no dir resolution, unlike a bundle's content paths which
        // are bundle-relative.
        let entries = manifest
            .mods
            .into_iter()
            .map(|decl| {
                let bundle = load_context.load::<BundleAsset>(AssetPath::from(decl.bundle.clone()));
                CatalogEntry { decl, bundle }
            })
            .collect();

        Ok(InstalledCatalog { entries })
    }

    fn extensions(&self) -> &[&str] {
        &["catalog.ron"]
    }
}

/// Registers the modding asset types and their RON loaders.
pub struct NovaModdingPlugin;

impl Plugin for NovaModdingPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<ContentAsset>()
            .init_asset_loader::<ContentAssetLoader>()
            .init_asset::<BundleAsset>()
            .init_asset_loader::<BundleAssetLoader>()
            .init_asset::<InstalledCatalog>()
            .init_asset_loader::<CatalogLoader>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `*.content.ron` body mixing a `Section((..))` and a `Scenario((..))`
    /// decodes into a `Vec<Content>` of length 2, with the kind flag driving
    /// which variant each item becomes. (The full nested-tree decode of each
    /// config is covered by nova_scenario's / nova_gameplay's own RON round-trip
    /// tests; here we only pin the loader's `Vec<Content>` decode path and the
    /// externally-tagged `Section`/`Scenario` mapping.)
    #[test]
    fn mixed_content_ron_decodes() {
        let ron = r#"[
            Section((
                base: (
                    id: "basic_hull_section",
                    name: "Basic Hull",
                    description: "a hull",
                    mass: 1.0,
                    health: 100.0,
                ),
                kind: Hull((
                    render_mesh: Some("gltf/hull-01.glb#Scene0"),
                )),
            )),
            Scenario((
                id: "demo",
                name: "Demo",
                description: "a tiny scenario",
                cubemap: "scenarios/space.cube.png",
                events: [],
            )),
        ]"#;

        let items: Vec<Content> =
            ron::de::from_bytes(ron.as_bytes()).expect("content RON should decode");
        assert_eq!(items.len(), 2);
        match &items[0] {
            Content::Section(section) => assert_eq!(section.base.id, "basic_hull_section"),
            other => panic!("expected a Section, got {other:?}"),
        }
        match &items[1] {
            Content::Scenario(scenario) => {
                assert_eq!(scenario.id, "demo");
                assert!(scenario.events.is_empty());
                assert_eq!(scenario.cubemap.path(), Some("scenarios/space.cube.png"));
            }
            other => panic!("expected a Scenario, got {other:?}"),
        }
    }

    /// A `bundle.ron` body decodes into a [`BundleManifest`] carrying the listed
    /// content-file paths in order. (The actual load of each content file into a
    /// `BundleAsset` is exercised by the `nova_assets` `demo_scenario`
    /// integration test on the real asset server.)
    /// A meta-less `(content: [...])` manifest still decodes (back-compat pin:
    /// every field of [`ModMeta`] is serde-defaulted), and a manifest WITH a meta
    /// block decodes every field.
    #[test]
    fn bundle_manifest_ron_decodes() {
        // Meta-less body (the pre-142849 format) -> ModMeta::default().
        let ron = r#"(content: [
            "sections/base.content.ron",
            "scenarios/demo.content.ron",
        ])"#;

        let manifest: BundleManifest =
            ron::de::from_bytes(ron.as_bytes()).expect("bundle manifest should decode");
        assert_eq!(
            manifest.content,
            vec![
                "sections/base.content.ron".to_string(),
                "scenarios/demo.content.ron".to_string(),
            ]
        );
        assert!(
            manifest.meta.name.is_empty() && manifest.meta.dependencies.is_empty(),
            "a meta-less manifest defaults to an empty ModMeta"
        );

        // Full meta block -> every field decodes.
        let ron = r#"(
            content: ["mod.content.ron"],
            meta: (
                name: "Demo Mod",
                description: "a demo",
                author: "someone",
                version: "1.2.3",
                dependencies: ["other-mod"],
                icon: Some("icon.png"),
                screenshots: ["shots/a.png"],
            ),
        )"#;
        let manifest: BundleManifest =
            ron::de::from_bytes(ron.as_bytes()).expect("meta manifest should decode");
        assert_eq!(manifest.meta.name, "Demo Mod");
        assert_eq!(manifest.meta.description, "a demo");
        assert_eq!(manifest.meta.author, "someone");
        assert_eq!(manifest.meta.version, "1.2.3");
        assert_eq!(manifest.meta.dependencies, vec!["other-mod".to_string()]);
        assert_eq!(manifest.meta.icon.as_deref(), Some("icon.png"));
        assert_eq!(manifest.meta.screenshots, vec!["shots/a.png".to_string()]);
    }

    /// A `mods.catalog.ron` body decodes into a [`CatalogManifest`] carrying the
    /// installed mods' thin declarations in order, with `base` and `hidden`
    /// defaulting to false when omitted. (The actual load of each bundle into an
    /// `InstalledCatalog` is exercised by the `nova_assets` integration test on
    /// the real asset server.)
    #[test]
    fn catalog_manifest_ron_decodes() {
        let ron = r#"(mods: [
            (id: "base", bundle: "base/base.bundle.ron", base: true),
            (id: "demo", bundle: "mods/demo/demo.bundle.ron"),
            (id: "reel", bundle: "mods/reel/reel.bundle.ron", hidden: true),
        ])"#;
        let manifest: CatalogManifest =
            ron::de::from_bytes(ron.as_bytes()).expect("catalog should decode");
        assert_eq!(manifest.mods.len(), 3);
        assert_eq!(manifest.mods[0].id, "base");
        assert!(manifest.mods[0].base, "base flag decodes");
        assert!(!manifest.mods[0].hidden, "hidden defaults to false");
        assert_eq!(manifest.mods[1].id, "demo");
        assert!(
            !manifest.mods[1].base,
            "base defaults to false when omitted"
        );
        assert_eq!(manifest.mods[1].bundle, "mods/demo/demo.bundle.ron");
        assert!(manifest.mods[2].hidden, "hidden flag decodes");
        assert!(!manifest.mods[2].base);
    }
}
