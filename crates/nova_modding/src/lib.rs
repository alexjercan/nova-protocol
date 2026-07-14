//! RON scenario/mod format for Nova Protocol.
//!
//! This crate is the authoring surface of the modding language: it loads
//! `*.scenario.ron` files into a [`ScenarioAsset`] wrapping a
//! [`ScenarioConfig`](nova_scenario::prelude::ScenarioConfig). The scenario
//! config tree is `serde` under nova_scenario's `serde` feature (which this
//! crate enables), so the loader is a pure RON decode.
//!
//! Asset references inside a scenario (section render meshes, particle effects,
//! the skybox cubemap, asteroid textures) are authored as paths and stay as
//! paths in the loaded config - each is an
//! [`AssetRef`](nova_gameplay::prelude::AssetRef) that resolves to a live
//! `Handle` lazily at spawn time through the `AssetServer`. So the loader does
//! not itself touch the `AssetServer`/`LoadContext`; it just deserializes.
//!
//! Downstream (`nova_assets`) drives the actual load of `assets/scenarios/*.ron`
//! into the `GameScenarios` resource.

use bevy::{
    asset::{io::Reader, Asset, AssetLoader, LoadContext, UntypedAssetId, VisitAssetDependencies},
    prelude::*,
    reflect::TypePath,
};
use nova_scenario::prelude::ScenarioConfig;

pub mod prelude {
    pub use super::{NovaModdingPlugin, ScenarioAsset, ScenarioAssetLoader, ScenarioLoaderError};
}

/// A scenario loaded from a `*.scenario.ron` file: a thin [`Asset`] wrapper
/// around the runtime [`ScenarioConfig`].
///
/// [`Asset`] is implemented by hand rather than derived: the derive would try to
/// walk the wrapped `ScenarioConfig` for `Handle` dependencies, but scenario
/// asset references are [`AssetRef`](nova_gameplay::prelude::AssetRef) paths that
/// resolve lazily at spawn, so this asset declares no dependencies of its own.
#[derive(TypePath, Clone, Debug)]
pub struct ScenarioAsset(pub ScenarioConfig);

impl VisitAssetDependencies for ScenarioAsset {
    fn visit_dependencies(&self, _visit: &mut impl FnMut(UntypedAssetId)) {}
}

impl Asset for ScenarioAsset {}

/// Errors produced while loading a `*.scenario.ron` file.
#[derive(Debug)]
pub enum ScenarioLoaderError {
    /// The file could not be read.
    Io(std::io::Error),
    /// The bytes were not valid scenario RON.
    Ron(ron::error::SpannedError),
}

impl std::fmt::Display for ScenarioLoaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScenarioLoaderError::Io(err) => write!(f, "failed to read scenario file: {err}"),
            ScenarioLoaderError::Ron(err) => write!(f, "failed to parse scenario RON: {err}"),
        }
    }
}

impl std::error::Error for ScenarioLoaderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ScenarioLoaderError::Io(err) => Some(err),
            ScenarioLoaderError::Ron(err) => Some(err),
        }
    }
}

impl From<std::io::Error> for ScenarioLoaderError {
    fn from(err: std::io::Error) -> Self {
        ScenarioLoaderError::Io(err)
    }
}

impl From<ron::error::SpannedError> for ScenarioLoaderError {
    fn from(err: ron::error::SpannedError) -> Self {
        ScenarioLoaderError::Ron(err)
    }
}

/// Bevy [`AssetLoader`] for `*.scenario.ron` files.
#[derive(Default, TypePath)]
pub struct ScenarioAssetLoader;

impl AssetLoader for ScenarioAssetLoader {
    type Asset = ScenarioAsset;
    type Settings = ();
    type Error = ScenarioLoaderError;

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

/// Registers the [`ScenarioAsset`] type and its `*.scenario.ron` loader.
pub struct NovaModdingPlugin;

impl Plugin for NovaModdingPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<ScenarioAsset>()
            .init_asset_loader::<ScenarioAssetLoader>();
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
}
