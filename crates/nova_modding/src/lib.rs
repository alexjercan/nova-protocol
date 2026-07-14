//! RON scenario/mod format for Nova Protocol.
//!
//! This crate is the authoring surface of the modding language. It loads:
//! - `*.scenario.ron` -> [`ScenarioAsset`] (a
//!   [`ScenarioConfig`](nova_scenario::prelude::ScenarioConfig)), and
//! - `*.sections.ron` -> [`SectionCatalogAsset`] (a `Vec` of
//!   [`SectionConfig`](nova_gameplay::prelude::SectionConfig) prototypes).
//!
//! The config trees are `serde` under nova_scenario's / nova_gameplay's `serde`
//! features (which this crate enables), so the loaders are pure RON decodes.
//!
//! Asset references inside the configs (section render meshes, particle effects,
//! the skybox cubemap, asteroid textures) are authored as paths and stay as paths
//! in the loaded config - each is an
//! [`AssetRef`](nova_gameplay::prelude::AssetRef) that resolves to a live `Handle`
//! lazily at spawn time through the `AssetServer`. So the loaders do not touch the
//! `AssetServer`/`LoadContext`; they just deserialize.
//!
//! Downstream (`nova_assets`) drives the actual load of `assets/scenarios/*.ron`
//! into `GameScenarios` and `assets/sections/*.ron` into `GameSections`.

use bevy::{
    asset::{io::Reader, Asset, AssetLoader, LoadContext, UntypedAssetId, VisitAssetDependencies},
    prelude::*,
    reflect::TypePath,
};
use nova_gameplay::prelude::SectionConfig;
use nova_scenario::prelude::ScenarioConfig;

pub mod prelude {
    pub use super::{
        ModdingLoaderError, NovaModdingPlugin, ScenarioAsset, ScenarioAssetLoader,
        SectionCatalogAsset, SectionCatalogAssetLoader,
    };
}

/// A scenario loaded from a `*.scenario.ron` file: a thin [`Asset`] wrapper
/// around the runtime [`ScenarioConfig`].
///
/// [`Asset`] is implemented by hand rather than derived: the derive would try to
/// walk the wrapped config for `Handle` dependencies, but asset references are
/// [`AssetRef`](nova_gameplay::prelude::AssetRef) paths that resolve lazily at
/// spawn, so this asset declares no dependencies of its own.
#[derive(TypePath, Clone, Debug)]
pub struct ScenarioAsset(pub ScenarioConfig);

impl VisitAssetDependencies for ScenarioAsset {
    fn visit_dependencies(&self, _visit: &mut impl FnMut(UntypedAssetId)) {}
}

impl Asset for ScenarioAsset {}

/// A section-prototype catalog loaded from a `*.sections.ron` file: the named
/// [`SectionConfig`]s a scenario's ships reference by id. Same lazy-`AssetRef`
/// contract as [`ScenarioAsset`], so it declares no `Handle` dependencies.
#[derive(TypePath, Clone, Debug)]
pub struct SectionCatalogAsset(pub Vec<SectionConfig>);

impl VisitAssetDependencies for SectionCatalogAsset {
    fn visit_dependencies(&self, _visit: &mut impl FnMut(UntypedAssetId)) {}
}

impl Asset for SectionCatalogAsset {}

/// Errors produced while loading a modding RON asset (`*.scenario.ron` or
/// `*.sections.ron`).
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

/// Bevy [`AssetLoader`] for `*.scenario.ron` files.
#[derive(Default, TypePath)]
pub struct ScenarioAssetLoader;

impl AssetLoader for ScenarioAssetLoader {
    type Asset = ScenarioAsset;
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
        let config: ScenarioConfig = ron::de::from_bytes(&bytes)?;
        Ok(ScenarioAsset(config))
    }

    fn extensions(&self) -> &[&str] {
        &["scenario.ron"]
    }
}

/// Bevy [`AssetLoader`] for `*.sections.ron` catalog files (a RON `Vec` of
/// [`SectionConfig`]).
#[derive(Default, TypePath)]
pub struct SectionCatalogAssetLoader;

impl AssetLoader for SectionCatalogAssetLoader {
    type Asset = SectionCatalogAsset;
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
        let sections: Vec<SectionConfig> = ron::de::from_bytes(&bytes)?;
        Ok(SectionCatalogAsset(sections))
    }

    fn extensions(&self) -> &[&str] {
        &["sections.ron"]
    }
}

/// Registers the modding asset types and their RON loaders.
pub struct NovaModdingPlugin;

impl Plugin for NovaModdingPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<ScenarioAsset>()
            .init_asset_loader::<ScenarioAssetLoader>()
            .init_asset::<SectionCatalogAsset>()
            .init_asset_loader::<SectionCatalogAssetLoader>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal `*.scenario.ron` body decodes into a `ScenarioConfig` the same
    /// way the loader will, with a path-authored cubemap. (The full nested-tree
    /// decode - objects, actions, ship bindings - is covered by nova_scenario's
    /// own RON round-trip test; here we only pin the loader's decode path and the
    /// `cubemap: "<path>"` -> `AssetRef` mapping.)
    #[test]
    fn minimal_scenario_ron_decodes() {
        let ron = r#"(
            id: "demo",
            name: "Demo",
            description: "a tiny scenario",
            cubemap: "scenarios/space.cube.png",
            events: [],
        )"#;

        let config: ScenarioConfig =
            ron::de::from_bytes(ron.as_bytes()).expect("scenario RON should decode");
        assert_eq!(config.id, "demo");
        assert!(config.events.is_empty());
        assert_eq!(config.cubemap.path(), Some("scenarios/space.cube.png"));
    }

    /// A minimal `*.sections.ron` catalog (a RON `Vec<SectionConfig>`) decodes the
    /// way the loader will, including the base fields and a path-authored render
    /// mesh via `AssetRef`.
    #[test]
    fn minimal_sections_ron_decodes() {
        let ron = r#"[
            (
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
            ),
        ]"#;

        let sections: Vec<SectionConfig> =
            ron::de::from_bytes(ron.as_bytes()).expect("sections RON should decode");
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].base.id, "basic_hull_section");
    }
}
