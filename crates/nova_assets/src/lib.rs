//! A Bevy plugin for loading game assets and initializing asset resources.

use std::collections::HashSet;

use bevy::{
    prelude::*,
    render::render_resource::{TextureViewDescriptor, TextureViewDimension},
};
use bevy_asset_loader::prelude::*;
use nova_gameplay::prelude::*;
use nova_modding::prelude::{BundleAsset, Content, ContentAsset, ModList};
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
    /// into `assets/base/sections/base.content.ron` (production loads that file
    /// via the base bundle and routes its items into `GameSections` via
    /// `register_bundles`).
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
    /// items - the shape the committed `assets/base/sections/base.content.ron` file
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

/// The production `register_bundles` system, re-exported for the crate's
/// integration tests (which drive the RON modding pipeline end to end: load the
/// base bundle + its content files and route their items into `GameSections` /
/// `GameScenarios`). Not part of the public API.
#[doc(hidden)]
pub use crate::register_bundles as register_bundles_for_test;

/// Route every loaded bundle's content into the id-keyed game registries, with
/// load-order overlay.
///
/// It builds the ORDERED bundle list - the base bundle first, then every enabled
/// mod bundle from the [`ModList`] enable-list in enable order - flattens each
/// bundle's content (in manifest order, across its content files), and hands the
/// whole ordered list to [`merge_bundles`]. Because a later bundle's items are
/// applied after earlier ones, a LATER (mod) bundle wins on an id collision with
/// the base (load-order overlay); a duplicate id WITHIN one bundle is a conflict,
/// logged and skipped. Both resources are always inserted (empty if nothing
/// loaded).
///
/// The base bundle and the enable-list are both part of the `GameAssets`
/// collection, so bevy_asset_loader gates the collection on their RECURSIVE load
/// state and guarantees every mod bundle + content file finished loading before
/// this runs `OnEnter(Processing)`. A handle whose asset is somehow not loaded is
/// logged and skipped (never a panic) - the remaining content still registers.
pub fn register_bundles(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    mod_lists: Res<Assets<ModList>>,
    bundles: Res<Assets<BundleAsset>>,
    contents: Res<Assets<ContentAsset>>,
) {
    // Ordered bundle handles: base first, then enabled mods in enable order.
    let mut bundle_handles: Vec<&Handle<BundleAsset>> = vec![&game_assets.base_bundle];
    match mod_lists.get(&game_assets.mod_list) {
        Some(mod_list) => bundle_handles.extend(mod_list.bundles.iter()),
        None => error!(
            "register_bundles: the mod enable-list was not loaded; registering the base only"
        ),
    }

    // Flatten each bundle into its ordered `&Content` items (missing content is
    // logged and skipped). Kept as one Vec per bundle so `merge_bundles` can tell
    // intra-bundle duplicates from cross-bundle overlay.
    let mut bundle_items: Vec<Vec<&Content>> = Vec::new();
    for bundle_handle in bundle_handles {
        let Some(bundle) = bundles.get(bundle_handle) else {
            error!(
                "register_bundles: a bundle asset was not loaded; skipping it \
                 (the other bundles still register)"
            );
            continue;
        };
        let mut items: Vec<&Content> = Vec::new();
        for content_handle in &bundle.content {
            let Some(content) = contents.get(content_handle) else {
                error!(
                    "register_bundles: a content asset was not loaded; skipping it \
                     (the other content still registers)"
                );
                continue;
            };
            items.extend(content.0.iter());
        }
        bundle_items.push(items);
    }

    let outcome = merge_bundles(bundle_items.iter().map(|items| items.iter().copied()));
    for conflict in &outcome.conflicts {
        error!("register_bundles: {conflict}");
    }

    commands.insert_resource(GameSections(outcome.sections));
    commands.insert_resource(outcome.scenarios);
}

/// The result of merging an ordered list of bundles: the id-keyed registries plus
/// any intra-bundle id conflicts that were detected (and skipped).
pub struct MergeOutcome {
    /// Sections in registration order (base then mods), overlaid last-wins by id.
    pub sections: Vec<SectionConfig>,
    /// Scenarios keyed by id, overlaid last-wins.
    pub scenarios: GameScenarios,
    /// Human-readable messages, one per intra-bundle duplicate id that was
    /// skipped. Empty on clean data.
    pub conflicts: Vec<String>,
}

/// Merge an ORDERED list of bundles into the id-keyed registries. Each bundle is
/// an ordered list of its `&Content` items (already flattened across the bundle's
/// content files).
///
/// Two overlay rules, mirroring Wesnoth's base+addons model:
/// - CROSS-bundle (a later bundle vs an earlier one): last-wins overlay by id -
///   a mod's `Content` with the same id as the base REPLACES it. This is the
///   whole point of mods.
/// - INTRA-bundle (the same id twice in ONE bundle - including the BASE bundle,
///   whose content files flatten into one bundle): a conflict. The first item is
///   kept, the duplicate is skipped, and a message is recorded. This is an
///   authoring error in any pack, surfaced loudly (the caller logs it) rather than
///   silently last-wins-overlaid like the cross-bundle case - but NOT a panic, so
///   bad mod (or base) data cannot crash the app.
pub fn merge_bundles<'a, B, I>(bundles: B) -> MergeOutcome
where
    B: IntoIterator<Item = I>,
    I: IntoIterator<Item = &'a Content>,
{
    let mut sections: Vec<SectionConfig> = Vec::new();
    let mut scenarios = GameScenarios::default();
    let mut conflicts: Vec<String> = Vec::new();

    for bundle in bundles {
        // Ids seen in THIS bundle, per kind - reset each bundle so a later bundle
        // may overlay an earlier one, while a repeat within one bundle conflicts.
        let mut seen_sections: HashSet<&str> = HashSet::new();
        let mut seen_scenarios: HashSet<&str> = HashSet::new();

        for item in bundle {
            match item {
                Content::Section(cfg) => {
                    if !seen_sections.insert(cfg.base.id.as_str()) {
                        conflicts.push(format!(
                            "section id '{}' appears more than once in one bundle; \
                             keeping the first, skipping the duplicate",
                            cfg.base.id
                        ));
                        continue;
                    }
                    merge_content_item(item, &mut sections, &mut scenarios);
                }
                Content::Scenario(cfg) => {
                    if !seen_scenarios.insert(cfg.id.as_str()) {
                        conflicts.push(format!(
                            "scenario id '{}' appears more than once in one bundle; \
                             keeping the first, skipping the duplicate",
                            cfg.id
                        ));
                        continue;
                    }
                    merge_content_item(item, &mut sections, &mut scenarios);
                }
            }
        }
    }

    MergeOutcome {
        sections,
        scenarios,
        conflicts,
    }
}

/// Route one content item into the accumulating registries with last-wins
/// overlay by id. Both kinds overlay identically: a later item (from a later
/// bundle) with the same id replaces the earlier one rather than appending a
/// shadowed duplicate. Sections keep a Vec (order matters for the editor palette)
/// so overlay is a linear replace-in-place; scenarios are a map so overlay is a
/// plain `insert`. Called by [`merge_bundles`] once per accepted item.
fn merge_content_item(
    item: &Content,
    sections: &mut Vec<SectionConfig>,
    scenarios: &mut GameScenarios,
) {
    match item {
        Content::Section(cfg) => match sections.iter_mut().find(|s| s.base.id == cfg.base.id) {
            Some(existing) => *existing = cfg.clone(),
            None => sections.push(cfg.clone()),
        },
        Content::Scenario(cfg) => {
            scenarios.insert(cfg.id.clone(), cfg.clone());
        }
    }
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
                register_bundles,
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
    /// The base game, packaged as a folder bundle: `assets/base/base.bundle.ron`
    /// lists the base content files, and its `BundleAsset` handle carries a
    /// `ContentAsset` handle for each. bevy_asset_loader gates the collection
    /// on this handle's RECURSIVE load state, so the bundle's content is fully
    /// loaded before `register_bundles` runs at `OnEnter(Processing)`.
    ///
    /// The `<pack>.bundle.ron` STEM is load-bearing: bevy_asset_loader kicks off
    /// each collection field with an UNTYPED `load_untyped`, which resolves the
    /// loader by extension only. Bevy's full extension is everything after the
    /// FIRST dot, so a bare `bundle.ron` resolves to `ron` (no loader) and fails;
    /// `base.bundle.ron` resolves to `bundle.ron` and matches `BundleAssetLoader`.
    #[asset(path = "base/base.bundle.ron")]
    pub base_bundle: Handle<BundleAsset>,
    /// The enabled mods, as a `ModList` enable-list (`assets/enabled.mods.ron`).
    /// Each entry is a mod `BundleAsset` handle; `register_bundles` merges them
    /// AFTER the base, in enable order, so a mod overlays the base by id. Ships
    /// EMPTY (`(mods: [])`) by default - a pristine base game. Like the base
    /// bundle, the collection gates on this handle's RECURSIVE load state, so
    /// every enabled mod bundle + its content is loaded before the merge runs.
    ///
    /// The `<name>.mods.ron` STEM is load-bearing for the same reason as the
    /// bundle (untyped `load_untyped` -> extension-only resolution): a bare
    /// `mods.ron` resolves to `ron` (no loader); `enabled.mods.ron` -> `mods.ron`,
    /// which `ModListLoader` registers.
    #[asset(path = "enabled.mods.ron")]
    pub mod_list: Handle<ModList>,
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

#[cfg(test)]
mod tests {
    use nova_gameplay::prelude::{BaseSectionConfig, HullSectionConfig, SectionKind};

    use super::*;

    fn section(id: &str, health: f32) -> SectionConfig {
        SectionConfig {
            base: BaseSectionConfig {
                id: id.to_string(),
                health,
                ..Default::default()
            },
            kind: SectionKind::Hull(HullSectionConfig::default()),
        }
    }

    /// A later content item with the same section id overlays the earlier one
    /// (last-wins) instead of appending a shadowed duplicate, and does so
    /// in-place so the palette order is preserved. This is the seam mods
    /// (20260714-134127) rely on, mirroring the scenario map's insert-overlay.
    #[test]
    fn later_section_overlays_earlier_by_id_in_place() {
        let mut sections: Vec<SectionConfig> = Vec::new();
        let mut scenarios = GameScenarios::default();

        // Base bundle: two sections in palette order.
        merge_content_item(
            &Content::Section(section("hull", 100.0)),
            &mut sections,
            &mut scenarios,
        );
        merge_content_item(
            &Content::Section(section("thruster", 50.0)),
            &mut sections,
            &mut scenarios,
        );

        // Mod bundle: overlays "hull" with a new health, leaves "thruster".
        merge_content_item(
            &Content::Section(section("hull", 999.0)),
            &mut sections,
            &mut scenarios,
        );

        // No duplicate appended: still two sections, original order kept.
        assert_eq!(sections.len(), 2, "overlay must replace, not append");
        assert_eq!(sections[0].base.id, "hull", "palette order preserved");
        assert_eq!(sections[1].base.id, "thruster");
        // Last-wins: the overlaid value took effect.
        assert_eq!(sections[0].base.health, 999.0, "later section must win");
    }

    /// A later scenario with the same id overlays the earlier one, same as
    /// sections - the two kinds must behave identically under overlay.
    #[test]
    fn later_scenario_overlays_earlier_by_id() {
        let mut sections: Vec<SectionConfig> = Vec::new();
        let mut scenarios = GameScenarios::default();

        // Reuse a real built scenario (no Default on ScenarioConfig) and overlay
        // a second config sharing its id but with a different name.
        let mut base = scenario_generation::build_scenarios()
            .into_iter()
            .next()
            .expect("build_scenarios yields at least one scenario");
        let id = base.id.clone();
        base.name = "base".to_string();
        let mut modded = base.clone();
        modded.name = "modded".to_string();

        merge_content_item(&Content::Scenario(base), &mut sections, &mut scenarios);
        merge_content_item(&Content::Scenario(modded), &mut sections, &mut scenarios);

        assert_eq!(scenarios.len(), 1, "overlay must replace, not add");
        assert_eq!(
            scenarios.get(&id).unwrap().name,
            "modded",
            "later scenario must win"
        );
    }

    /// A later bundle (a mod) overlays an earlier bundle (the base) by id:
    /// last-wins across bundles, with a fresh section left added. No conflicts -
    /// same id in DIFFERENT bundles is the intended overlay, not an error.
    #[test]
    fn merge_bundles_overlays_later_bundle_by_id() {
        let base = vec![
            Content::Section(section("hull", 100.0)),
            Content::Section(section("thruster", 50.0)),
        ];
        let modded = vec![
            // Overrides the base hull by id.
            Content::Section(section("hull", 999.0)),
            // Adds a brand-new section.
            Content::Section(section("shield", 25.0)),
        ];

        let outcome = merge_bundles([base.iter(), modded.iter()]);

        assert!(
            outcome.conflicts.is_empty(),
            "same id across bundles is overlay, not a conflict: {:?}",
            outcome.conflicts
        );
        // hull overlaid in place (order preserved), thruster kept, shield appended.
        assert_eq!(
            outcome
                .sections
                .iter()
                .map(|s| s.base.id.as_str())
                .collect::<Vec<_>>(),
            vec!["hull", "thruster", "shield"]
        );
        assert_eq!(
            outcome.sections[0].base.health, 999.0,
            "the mod's hull must win over the base's"
        );
    }

    /// The SAME id twice within ONE bundle is a conflict: the first is kept, the
    /// duplicate is skipped and recorded. This is the "intra-bundle duplicate is
    /// an error" rule (surfaced loudly by the caller), distinct from cross-bundle
    /// overlay.
    #[test]
    fn merge_bundles_intra_bundle_duplicate_is_a_conflict() {
        let bundle = vec![
            Content::Section(section("hull", 100.0)),
            // Duplicate id in the SAME bundle - a conflict, not an overlay.
            Content::Section(section("hull", 999.0)),
        ];

        let outcome = merge_bundles([bundle.iter()]);

        assert_eq!(outcome.sections.len(), 1, "the duplicate must be skipped");
        assert_eq!(
            outcome.sections[0].base.health, 100.0,
            "the FIRST occurrence is kept within a bundle"
        );
        assert_eq!(outcome.conflicts.len(), 1, "the conflict is recorded");
        assert!(
            outcome.conflicts[0].contains("hull"),
            "the conflict names the offending id: {}",
            outcome.conflicts[0]
        );
    }
}
