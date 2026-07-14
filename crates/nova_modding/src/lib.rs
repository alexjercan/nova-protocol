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
//! `register_content`) dispatches each item into its id-keyed registry
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
    asset::{io::Reader, Asset, AssetLoader, LoadContext, UntypedAssetId, VisitAssetDependencies},
    prelude::*,
    reflect::TypePath,
};
use nova_gameplay::prelude::SectionConfig;
use nova_scenario::prelude::ScenarioConfig;
use serde::{Deserialize, Serialize};

pub mod prelude {
    pub use super::{
        Content, ContentAsset, ContentAssetLoader, ModdingLoaderError, NovaModdingPlugin,
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

/// Registers the modding asset type and its RON loader.
pub struct NovaModdingPlugin;

impl Plugin for NovaModdingPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<ContentAsset>()
            .init_asset_loader::<ContentAssetLoader>();
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
}
