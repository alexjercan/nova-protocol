//! A Bevy plugin for loading game assets and initializing asset resources.

use bevy::{
    prelude::*,
    render::render_resource::{TextureViewDescriptor, TextureViewDimension},
};
use bevy_asset_loader::prelude::*;
use nova_gameplay::prelude::*;
use nova_modding::prelude::{Content, ContentAsset};
use nova_scenario::prelude::GameScenarios;

mod scenario;
mod sections;

pub mod prelude {
    pub use super::{GameAssets, GameAssetsPlugin, GameAssetsStates};
}

/// The RON generation surface for the built-in scenarios (task 20260525-133028
/// follow-up). The scenario builders are the single definition of each
/// built-in; production loads their serialized RON, and this module lets the
/// generator/parity test rebuild them with PATH-based asset refs and serialize
/// them deterministically. Not part of the game's public API - it exists for
/// the `content_ron_parity` integration test.
///
/// The `ScenarioConfig` serde derives are already present in this crate's
/// build: `nova_modding` (a dependency) turns on `nova_scenario/serde`, and
/// Cargo feature unification carries it here.
#[doc(hidden)]
pub mod scenario_generation {
    use nova_gameplay::prelude::{AssetRef, SectionConfig};
    use nova_modding::prelude::Content;
    use nova_scenario::prelude::ScenarioConfig;

    use crate::sections::{build_sections, SectionMeshRefs};

    /// The skybox cubemap asset path (matches `GameAssets::cubemap`).
    const CUBEMAP_PATH: &str = "textures/cubemap.png";
    /// The asteroid texture asset path (matches `GameAssets::asteroid_texture`).
    const ASTEROID_TEXTURE_PATH: &str = "textures/asteroid.png";

    /// The section-prototype catalog built from PATH-based mesh refs - the source
    /// the content parity test wraps as `Content::Section` items and serializes
    /// into `assets/sections/base.content.ron` (production loads that file and
    /// routes its items into `GameSections` via `register_content`).
    pub fn build_section_catalog() -> Vec<SectionConfig> {
        build_sections(&SectionMeshRefs::from_paths())
    }

    /// Build all four built-in configs with path-based asset refs, in a stable
    /// order. This is the source the parity test serializes and compares. The
    /// ships now reference the section catalog by prototype id, so the scenario
    /// generators no longer need the resolved `GameSections`.
    pub fn build_scenarios() -> Vec<ScenarioConfig> {
        let cubemap = || AssetRef::from(CUBEMAP_PATH.to_string());
        let texture = || AssetRef::from(ASTEROID_TEXTURE_PATH.to_string());

        vec![
            crate::scenario::asteroid_next(cubemap()),
            crate::scenario::asteroid_field(cubemap(), texture()),
            crate::scenario::menu_ambience(cubemap(), texture()),
            crate::scenario::shakedown::shakedown_run(cubemap(), texture()),
        ]
    }

    /// The section catalog wrapped as one `Vec<Content>` of `Content::Section`
    /// items - the shape the committed `assets/sections/base.content.ron` file
    /// carries. The parity test serializes this.
    pub fn build_section_content() -> Vec<Content> {
        build_section_catalog()
            .into_iter()
            .map(Content::Section)
            .collect()
    }

    /// The four built-in scenarios, each wrapped as its own single-item
    /// `Vec<Content>` (`[Content::Scenario(..)]`) keyed by scenario id - the
    /// shape each committed `assets/scenarios/<id>.content.ron` file carries. The
    /// parity test serializes each.
    pub fn build_scenario_contents() -> Vec<(String, Vec<Content>)> {
        build_scenarios()
            .into_iter()
            .map(|scenario| (scenario.id.clone(), vec![Content::Scenario(scenario)]))
            .collect()
    }

    /// The deterministic pretty-printer for the built-in content RON. Matches
    /// the hand-committed `demo.content.ron` style: struct names omitted,
    /// indented, so the data files stay diff-friendly and reviewable.
    pub fn pretty_config() -> ron::ser::PrettyConfig {
        ron::ser::PrettyConfig::default()
            .struct_names(false)
            .separate_tuple_members(true)
            .enumerate_arrays(false)
    }
}

/// The production `register_content` system, re-exported for the crate's
/// integration tests (which drive the RON modding pipeline end to end: load the
/// `.content.ron` files and route their items into `GameSections` /
/// `GameScenarios`). Not part of the public API.
#[doc(hidden)]
pub use crate::register_content as register_content_for_test;

/// Route every loaded [`ContentAsset`] item into its id-keyed registry.
///
/// This is the generic replacement for the old per-kind `register_sections` +
/// `register_scenario` systems: it reads each content file from
/// [`Assets<ContentAsset>`] through the [`GameAssets`] handles and dispatches
/// each [`Content`] item by variant - `Section` collects into the
/// [`GameSections`] `Vec`, `Scenario` inserts into [`GameScenarios`] keyed by
/// its id. Both resources are always inserted (empty if nothing loaded).
///
/// The content files are part of the `GameAssets` collection, so
/// bevy_asset_loader guarantees they finished loading before this runs
/// `OnEnter(Processing)`; a handle whose asset is somehow not loaded is logged
/// and skipped (never a panic) - the remaining files still register.
pub fn register_content(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    contents: Res<Assets<ContentAsset>>,
) {
    let mut sections: Vec<SectionConfig> = Vec::new();
    let mut scenarios = GameScenarios::default();

    // Every content-file handle in the collection. Order is stable so overlay
    // (later id wins) is deterministic; base sections first, then scenarios.
    let handles = [
        &game_assets.section_content,
        &game_assets.demo_content,
        &game_assets.asteroid_field_content,
        &game_assets.asteroid_next_content,
        &game_assets.menu_ambience_content,
        &game_assets.shakedown_content,
    ];

    for handle in handles {
        let Some(content) = contents.get(handle) else {
            error!(
                "register_content: a content asset was not loaded; skipping it \
                 (the other content still registers)"
            );
            continue;
        };
        for item in &content.0 {
            match item {
                Content::Section(cfg) => sections.push(cfg.clone()),
                Content::Scenario(cfg) => {
                    scenarios.insert(cfg.id.clone(), cfg.clone());
                }
            }
        }
    }

    commands.insert_resource(GameSections(sections));
    commands.insert_resource(scenarios);
}

/// Game states for the asset loader.
#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
pub enum GameAssetsStates {
    #[default]
    Loading,
    Processing,
    Loaded,
}

/// A plugin that loads game assets and sets up the game.
pub struct GameAssetsPlugin;

impl Plugin for GameAssetsPlugin {
    fn build(&self, app: &mut App) {
        debug!("GameAssetsPlugin: build");

        // The modding plugin registers the `*.content.ron` asset + loader.
        // Add it before the loading state runs so the loader exists when
        // bevy_asset_loader starts loading the content files below.
        app.add_plugins(nova_modding::prelude::NovaModdingPlugin);

        // Setup the asset loader to load assets during the loading state.
        app.init_state::<GameAssetsStates>();
        app.add_loading_state(
            LoadingState::new(GameAssetsStates::Loading)
                .continue_to_state(GameAssetsStates::Processing)
                .load_collection::<GameAssets>(),
        );

        app.add_systems(
            OnEnter(GameAssetsStates::Processing),
            (
                prepare_cubemap_view,
                register_content,
                register_sounds,
                update_nova_hud_assets,
                |mut state: ResMut<NextState<GameAssetsStates>>| {
                    state.set(GameAssetsStates::Loaded);
                },
            )
                .chain(),
        );
    }
}

#[derive(AssetCollection, Resource, Clone)]
pub struct GameAssets {
    #[asset(path = "textures/cubemap.png")]
    pub cubemap: Handle<Image>,
    #[asset(path = "textures/asteroid.png")]
    pub asteroid_texture: Handle<Image>,
    #[asset(path = "gltf/hull-01.glb#Scene0")]
    pub hull_01: Handle<WorldAsset>,
    #[asset(path = "gltf/turret-yaw-01.glb#Scene0")]
    pub turret_yaw_01: Handle<WorldAsset>,
    #[asset(path = "gltf/turret-pitch-01.glb#Scene0")]
    pub turret_pitch_01: Handle<WorldAsset>,
    #[asset(path = "gltf/turret-barrel-01.glb#Scene0")]
    pub turret_barrel_01: Handle<WorldAsset>,
    #[asset(path = "gltf/torpedo-bay-01.glb#Scene0")]
    pub torpedo_bay_01: Handle<WorldAsset>,
    #[asset(path = "icons/fps.png")]
    pub fps_icon: Handle<Image>,
    #[asset(path = "icons/target.png")]
    pub target_sprite: Handle<Image>,
    #[asset(path = "sections/base.content.ron")]
    pub section_content: Handle<ContentAsset>,
    #[asset(path = "scenarios/demo.content.ron")]
    pub demo_content: Handle<ContentAsset>,
    #[asset(path = "scenarios/asteroid_field.content.ron")]
    pub asteroid_field_content: Handle<ContentAsset>,
    #[asset(path = "scenarios/asteroid_next.content.ron")]
    pub asteroid_next_content: Handle<ContentAsset>,
    #[asset(path = "scenarios/menu_ambience.content.ron")]
    pub menu_ambience_content: Handle<ContentAsset>,
    #[asset(path = "scenarios/shakedown_run.content.ron")]
    pub shakedown_content: Handle<ContentAsset>,
}

/// Give the skybox cubemap its cube texture view.
///
/// The stacked `textures/cubemap.png` is reinterpreted into a 6 layer array
/// at load time by its `.meta` loader settings (`array_layout: RowCount`).
/// Doing it at load time matters: the renderer eagerly uploads every loaded
/// image, and the raw stacked form is 24576 px tall - over the 16384 texture
/// limit of smaller GPUs (e.g. CI's llvmpipe), where the upload becomes a
/// fatal validation error. Whether the old on-insert reinterpret in
/// `SkyboxPlugin` beat that upload depended on which frame the asset
/// finished loading, so the failure was flaky.
///
/// The loader settings cannot express a texture view, so the cube view is
/// set here, in the Processing state - after the collection is loaded and
/// before anything spawns a camera. `SkyboxPlugin` sees the layers and view
/// already prepared and just attaches the `Skybox` component.
///
/// If the meta was not applied (the image still has a single layer), leave
/// the image alone so the `SkyboxPlugin` fallback reinterpret still works.
fn prepare_cubemap_view(mut images: ResMut<Assets<Image>>, game_assets: Res<GameAssets>) {
    let Some(mut image) = images.get_mut(&game_assets.cubemap) else {
        error!("prepare_cubemap_view: cubemap image not loaded");
        return;
    };
    if image.texture_descriptor.array_layer_count() > 1 {
        image.texture_view_descriptor = Some(TextureViewDescriptor {
            dimension: Some(TextureViewDimension::Cube),
            ..default()
        });
    } else {
        warn!(
            "prepare_cubemap_view: cubemap loaded as a single layer image; \
             was the `cubemap.png.meta` array_layout applied?"
        );
    }
}

/// Load the Nova sound effects into a keyed [`SoundBank`] the audio module reads.
///
/// Uses `SoundBank::load` (the bcs registry) rather than the `GameAssets`
/// collection because the bank has no public "build from existing handles"
/// constructor; loading here kicks the (tiny) WAVs off well before the first
/// gameplay sound plays. The `sounds/<name>.wav` convention is applied by the
/// bank, and `NOVA_SFX_FILES` is the single source of truth for the key->file map.
fn register_sounds(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(SoundBank::load(&assets, NOVA_SFX_FILES));
}

// TODO(20260525-133028): Probably need to refactor this somehow
fn update_nova_hud_assets(
    mut nova_hud_assets: ResMut<NovaHudAssets>,
    game_assets: Res<GameAssets>,
) {
    nova_hud_assets.target_sprite = game_assets.target_sprite.clone();
}
