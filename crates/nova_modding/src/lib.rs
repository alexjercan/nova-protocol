//! RON scenario/mod format for Nova Protocol.
//!
//! This crate is the authoring surface of the modding language. It loads a
//! single uniform content format:
//! - `*.content.ron` -> [`ContentAsset`], a RON `Vec<`[`Content`]`>` where each
//!   item carries its KIND as a data flag (`Section((..))` / `Scenario((..))`).
//!
//! A [`Content`] item is one of:
//! - [`Content::Section`] - a [`SectionConfig`] prototype,
//! - [`Content::Scenario`] - a [`ScenarioConfig`], and
//! - [`Content::Campaign`] - a [`CampaignConfig`], the ordered scenario-id
//!   mapping the Scenarios picker groups and launches by,
//! - [`Content::Style`] - a [`ShipStyleConfig`], the look a ship's derived skin
//!   wears: materials per surface role plus the decoration it scatters, and
//! - [`Content::Ship`] - a [`ShipConfig`], a whole hull a scenario spawns by id.
//!
//! The kind lives IN the RON structure (an externally-tagged enum), so ONE
//! loader reads any content file and a downstream router (`nova_assets`'s
//! `register_bundles`) dispatches each item into its id-keyed registry
//! (`GameSections` / `GameScenarios` / `GameCampaigns`). A single file may mix
//! kinds.
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
#![warn(missing_docs)]

use bevy::{
    asset::{
        io::Reader, Asset, AssetLoader, AssetPath, LoadContext, UntypedAssetId,
        VisitAssetDependencies,
    },
    prelude::*,
    reflect::TypePath,
};
// The pure serde format types live in the engine-free `nova_mod_format`
// crate so the portal generator builds without bevy; re-exported here so game
// code keeps importing them from nova_modding.
pub use nova_mod_format::{BundleManifest, CatalogManifest, ModEntry, ModMeta, BASE_MOD_ID};
use nova_scenario::prelude::{CampaignConfig, ScenarioConfig, ShipConfig};
use nova_ship::prelude::{SectionConfig, ShipStyleConfig};
use serde::{Deserialize, Serialize};

/// Glob-import surface: `use nova_modding::prelude::*` brings the content/bundle
/// asset types, their RON loaders, and [`NovaModdingPlugin`] into scope.
pub mod prelude {
    pub use super::{
        parse_content, pretty_config, serialize_content, serialize_manifest, BundleAsset,
        BundleAssetLoader, BundleManifest, CatalogEntry, CatalogLoader, CatalogManifest, Content,
        ContentAsset, ContentAssetLoader, InstalledCatalog, ModEntry, ModMeta, ModdingLoaderError,
        NovaModdingPlugin, BASE_MOD_ID,
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
    /// registers into `GameSections`. Boxed so the enum is not sized by its
    /// largest payload (`Box<T>` serializes exactly like `T`, so the RON
    /// wire shape is unchanged - the content parity tests pin that).
    Section(Box<SectionConfig>),
    /// A [`ScenarioConfig`] - registers into `GameScenarios` keyed by its id.
    Scenario(ScenarioConfig),
    /// A [`CampaignConfig`] - registers into `GameCampaigns` keyed by its id.
    /// Carries the campaign's ordered member scenario ids (hidden ones
    /// included) so the picker can group and launch them as a unit.
    Campaign(CampaignConfig),
    /// A [`ShipStyleConfig`] - registers into `GameStyles` keyed by its id. A
    /// ship names one and its derived skin wears it, so a mod can ship a look
    /// and a scenario can put it on the enemies without any code changing.
    Style(ShipStyleConfig),
    /// A [`ShipConfig`] - registers into `GameShips` keyed by its id. A whole
    /// hull (its section list, its skin and style, its collapse threshold),
    /// authored once and spawned by id, so a scenario names a corvette instead
    /// of carrying a copy of one.
    Ship(ShipConfig),
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
    /// The manifest's New Game declaration, carried through verbatim; the
    /// merge honors it only from the base bundle (see `BundleManifest`).
    pub new_game_scenario: Option<String>,
    /// The binary resource files the manifest declared (bundle-dir-relative),
    /// carried verbatim. The merge validates every `self://` content ref against
    /// this list before rewriting it (see `BundleManifest::resources`).
    pub resources: Vec<String>,
    /// The bundle's own folder as an asset-path prefix, computed from this
    /// bundle's load path with the source scheme preserved: `mods/<id>` for a
    /// shipped bundle (default source), `mods://<id>` for a downloaded one. A
    /// `self://X` content ref rewrites to `<resource_base>/X` at merge time, so
    /// it resolves against the mod's own folder on native and web alike.
    pub resource_base: String,
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

/// The deterministic pretty-printer every writer of a content file shares.
///
/// Struct names omitted, indented, tuple members on their own lines - the
/// style the hand-authored mods are written in
/// (`assets/mods/example/example.content.ron`), so a generated file and an
/// authored one read the same and a regenerated file diffs against itself
/// rather than against its own formatting.
pub fn pretty_config() -> ron::ser::PrettyConfig {
    ron::ser::PrettyConfig::default()
        .struct_names(false)
        .separate_tuple_members(true)
        .enumerate_arrays(false)
}

/// One content `Vec` as a `*.content.ron` file body: the pretty config above
/// plus a trailing newline (POSIX-clean).
///
/// Here rather than in the offline authoring tool because the editor writes
/// content files too, and two writers with two formatters would produce two
/// dialects of one format.
pub fn serialize_content(content: &[Content]) -> Result<String, ron::Error> {
    let body = ron::ser::to_string_pretty(&content, pretty_config())?;
    Ok(format!("{body}\n"))
}

/// One bundle manifest as a `*.bundle.ron` file body, in the same style.
pub fn serialize_manifest(manifest: &BundleManifest) -> Result<String, ron::Error> {
    let body = ron::ser::to_string_pretty(manifest, pretty_config())?;
    Ok(format!("{body}\n"))
}

/// Decode a `*.content.ron` file body - the same read [`ContentAssetLoader`]
/// does, for callers that hold the bytes rather than an asset path.
pub fn parse_content(bytes: &[u8]) -> Result<Vec<Content>, ron::error::SpannedError> {
    ron::de::from_bytes(bytes)
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
/// `base.bundle.ron` yields `bundle.ron` and matches.
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

        // Content paths resolve against the bundle file's DIRECTORY, which
        // is what makes a bundle folder self-contained and relocatable.
        let base = load_context
            .path()
            .parent()
            .unwrap_or_else(|| AssetPath::from(""));

        let content = manifest
            .content
            .iter()
            .map(|rel| {
                // The owned `to_string` is load-bearing - an
                // `AssetPath::from(&str)` would borrow `manifest.content`, which
                // does not outlive the resolved path.
                let resolved = base.resolve(&AssetPath::from(rel.to_string()));
                load_context.load::<ContentAsset>(resolved)
            })
            .collect();

        // `base` keeps its source scheme, so this is `mods/<id>` for a
        // shipped bundle and `mods://<id>` for a downloaded one - exactly the
        // prefix a `self://` content ref rewrites against at merge time. Must
        // stay after the content map, which borrows `base`.
        let resource_base = base.to_string();

        Ok(BundleAsset {
            content,
            meta: manifest.meta,
            new_game_scenario: manifest.new_game_scenario,
            resources: manifest.resources,
            resource_base,
        })
    }

    fn extensions(&self) -> &[&str] {
        &["bundle.ron"]
    }
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
/// `catalog.ron` and matches.
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

        // Bundle paths are asset-root-relative (the catalog lives at the
        // root), so they load as-is - no dir resolution, unlike a bundle's own
        // content paths.
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
            Campaign((
                id: "demo_campaign",
                name: "Demo Campaign",
                scenarios: ["demo", "demo_two"],
            )),
        ]"#;

        let items: Vec<Content> =
            ron::de::from_bytes(ron.as_bytes()).expect("content RON should decode");
        assert_eq!(items.len(), 3);
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
        match &items[2] {
            Content::Campaign(campaign) => {
                assert_eq!(campaign.id, "demo_campaign");
                assert_eq!(campaign.name, "Demo Campaign");
                assert_eq!(campaign.scenarios, vec!["demo", "demo_two"]);
            }
            other => panic!("expected a Campaign, got {other:?}"),
        }
    }

    /// What [`serialize_content`] writes is what [`parse_content`] reads, and
    /// it writes the same bytes twice. The editor saves through this pair, so a
    /// file it cannot read back is a lost document and a file that differs from
    /// itself is a save that diffs on every write.
    #[test]
    fn a_written_content_file_reads_back_and_writes_the_same_twice() {
        let written = vec![Content::Scenario(ScenarioConfig::new(
            "demo".to_string(),
            "Demo".to_string(),
            "scenarios/space.cube.png".into(),
        ))];

        let body = serialize_content(&written).expect("serialize");
        let read = parse_content(body.as_bytes()).expect("parse back");

        assert_eq!(read.len(), 1);
        match &read[0] {
            Content::Scenario(scenario) => assert_eq!(scenario.id, "demo"),
            other => panic!("expected a Scenario, got {other:?}"),
        }
        assert_eq!(serialize_content(&read).expect("re-serialize"), body);
        assert!(body.ends_with('\n'), "content files end in a newline");
    }
}
