//! The PRELOADED asset collections ([`BootAssets`], [`GameAssets`]) and the
//! `Processing`-state systems that publish their handles to the consumers that
//! read them (the UI font, the skybox cube view, the UI sound bank, the HUD art).

use bevy::{
    prelude::*,
    render::render_resource::{TextureViewDescriptor, TextureViewDimension},
};
use bevy_asset_loader::prelude::*;
use nova_gameplay::prelude::*;
use nova_modding::prelude::InstalledCatalog;
use nova_ui::font::UiFont;

/// The boot collection: the handful of assets the boot loading screen needs to
/// render itself before the bulk [`GameAssets`] load starts. Loaded in the
/// `Boot` state; today just the shared UI font, so the phosphor loading screen
/// has its themed typeface from the first frame.
#[derive(AssetCollection, Resource, Clone)]
pub struct BootAssets {
    /// The shared UI typeface, published as [`UiFont`] at `OnExit(Boot)`.
    #[asset(path = "fonts/SGr-IosevkaTerm-Medium.ttf")]
    pub ui_font: Handle<Font>,
}

/// Publish the preloaded UI font as the shared [`UiFont`] resource once the
/// [`BootAssets`] collection has resolved (run at `OnExit(Boot)`).
pub(crate) fn fill_ui_font(mut commands: Commands, boot: Res<BootAssets>) {
    commands.insert_resource(UiFont(boot.ui_font.clone()));
}

/// The canonical UI-SFX asset paths held by the `GameAssets::ui_sfx` mapped
/// collection - the single distinct file per entry in
/// [`nova_gameplay::audio::UI_SFX_FILES`] (which has two keys sharing
/// `ui_toggle`). The collection's `#[asset(paths(...))]` list MUST mirror this
/// (kept adjacent so they move together); `ui_sfx_collection_matches_ui_sfx_files`
/// pins that this set covers exactly the files `UI_SFX_FILES` references, and
/// that each file exists on disk.
#[cfg(test)]
const UI_SFX_COLLECTION_PATHS: [&str; 14] = [
    "sounds/objective_new.wav",
    "sounds/objective_complete.wav",
    "sounds/menu_select.wav",
    "sounds/ui_toggle.wav",
    "sounds/nova_key.wav",
    "sounds/nova_back.wav",
    "sounds/nova_enter.wav",
    "sounds/nova_ok.wav",
    "sounds/nova_error.wav",
    "sounds/nova_tick.wav",
    "sounds/nova_coil.wav",
    "sounds/nova_powerup.wav",
    "sounds/nova_powerdown.wav",
    "sounds/nova_bed.wav",
];

/// The keycap paths held by the `GameAssets::key_glyphs` mapped collection -
/// a verbatim mirror of that `#[asset(paths(...))]` attribute (kept adjacent so
/// they move together). `key_glyph_collection_matches_mapping_table` pins this
/// against `nova_gameplay::hud::key_glyphs`'s mapping table, which OWNS the set.
#[cfg(test)]
const KEY_GLYPH_COLLECTION_PATHS: [&str; 13] = [
    "input-prompts/keyboard/Alt/T_Brackets_L_Key_Alt.png",
    "input-prompts/keyboard/Alt/T_Brackets_R_Key_Alt.png",
    "input-prompts/keyboard/Alt/T_Crtl_Key_Alt.png",
    "input-prompts/keyboard/Alt/T_G_Key_Alt.png",
    "input-prompts/keyboard/Alt/T_Mouse_Scroll_Key_Dark_Key_Alt.png",
    "input-prompts/keyboard/Alt/T_O_Key_Alt.png",
    "input-prompts/keyboard/Alt/T_Shift_Key_Alt.png",
    "input-prompts/keyboard/Alt/T_Space_Key_Alt.png",
    "input-prompts/keyboard/Alt/T_Tab_Key_Alt.png",
    "input-prompts/keyboard/Alt/T_Tilde_Key_Alt.png",
    "input-prompts/keyboard/Alt/T_W_Key_Alt.png",
    "input-prompts/keyboard/Alt/T_X_Key_Alt.png",
    "input-prompts/keyboard/Alt/T_Z_Key_Alt.png",
];

/// The loaded base-game asset handles, collected by `bevy_asset_loader` and
/// inserted as a [`Resource`] once every listed asset (and the installed-mods
/// catalog) has loaded. Systems read these handles to build meshes/materials.
#[derive(AssetCollection, Resource, Clone)]
pub struct GameAssets {
    /// The skybox cubemap texture.
    #[asset(path = "base/textures/cubemap.png")]
    pub cubemap: Handle<Image>,
    /// The asteroid surface texture.
    #[asset(path = "base/textures/asteroid.png")]
    pub asteroid_texture: Handle<Image>,
    /// The base hull section mesh.
    #[asset(path = "base/gltf/hull-01.glb#Scene0")]
    pub hull_01: Handle<WorldAsset>,
    /// The turret yaw-ring section mesh.
    #[asset(path = "base/gltf/turret-yaw-01.glb#Scene0")]
    pub turret_yaw_01: Handle<WorldAsset>,
    /// The turret pitch-mount section mesh.
    #[asset(path = "base/gltf/turret-pitch-01.glb#Scene0")]
    pub turret_pitch_01: Handle<WorldAsset>,
    /// The turret barrel section mesh.
    #[asset(path = "base/gltf/turret-barrel-01.glb#Scene0")]
    pub turret_barrel_01: Handle<WorldAsset>,
    /// The torpedo-bay section mesh.
    #[asset(path = "base/gltf/torpedo-bay-01.glb#Scene0")]
    pub torpedo_bay_01: Handle<WorldAsset>,
    /// The FPS status-bar icon.
    #[asset(path = "icons/fps.png")]
    pub fps_icon: Handle<Image>,
    /// The lock-on target sprite.
    #[asset(path = "icons/target.png")]
    pub target_sprite: Handle<Image>,
    /// The NOVA CRT brand mark, drawn by the NOVA OS drawer plate. Preloaded
    /// here so that site reads the handle through [`NovaHudAssets`] instead of
    /// a lazy `asset_server.load`.
    #[asset(path = "icons/nova_crt_mark.png")]
    pub nova_crt_mark: Handle<Image>,
    /// The UI sound-effect handles, keyed by file stem. Loaded and load-gated as
    /// part of this collection so the WAVs are ready before `Processing`; the
    /// `SoundBank<UiSfx>` built in `register_sounds` references these same
    /// already-loaded assets (the AssetServer dedups by path). The explicit
    /// `paths(...)` list is kept in sync with `nova_gameplay::audio::UI_SFX_FILES`
    /// by `ui_sfx_collection_matches_ui_sfx_files`; a folder collection is not
    /// used because folder collections do not work on wasm.
    #[asset(
        paths(
            "sounds/objective_new.wav",
            "sounds/objective_complete.wav",
            "sounds/menu_select.wav",
            "sounds/ui_toggle.wav",
            "sounds/nova_key.wav",
            "sounds/nova_back.wav",
            "sounds/nova_enter.wav",
            "sounds/nova_ok.wav",
            "sounds/nova_error.wav",
            "sounds/nova_tick.wav",
            "sounds/nova_coil.wav",
            "sounds/nova_powerup.wav",
            "sounds/nova_powerdown.wav",
            "sounds/nova_bed.wav",
        ),
        collection(mapped, typed)
    )]
    pub ui_sfx: bevy::platform::collections::HashMap<AssetFileStem, Handle<AudioSource>>,
    /// The installed-mods catalog (`assets/mods.catalog.ron`): every installed mod
    /// (base first, then mods) with metadata + a `BundleAsset` handle. The
    /// `InstalledCatalog` asset visits EVERY entry's bundle as a dependency, so
    /// bevy_asset_loader gates the collection on the whole tree's RECURSIVE load
    /// state - every installed bundle + its content is loaded before
    /// `register_bundles` runs at `OnEnter(Processing)`, regardless of which mods
    /// are enabled. `EnabledMods` then selects which cataloged bundles actually
    /// merge (base enabled by default; the mods menu toggles the rest).
    ///
    /// The `<name>.catalog.ron` STEM is load-bearing: bevy_asset_loader kicks off
    /// each collection field with an UNTYPED `load_untyped`, which resolves the
    /// loader by extension only. Bevy's full extension is everything after the FIRST
    /// dot, so a bare `catalog.ron` resolves to `ron` (no loader) and fails;
    /// `mods.catalog.ron` resolves to `catalog.ron` and matches `CatalogLoader`.
    #[asset(path = "mods.catalog.ron")]
    pub catalog: Handle<InstalledCatalog>,
    /// The keycap glyphs the HUD draws for bound keys, keyed by file stem. The
    /// explicit `paths(...)` list mirrors
    /// [`nova_gameplay::hud::key_glyphs::key_glyph_asset_paths`] (the mapping
    /// table owns it); `key_glyph_collection_matches_mapping_table` pins them
    /// together. Preloaded rather than lazily `server.load`ed per chip so the
    /// keycaps load-gate like the UI font and the CRT mark; a folder collection
    /// is not used because folder collections do not work on wasm.
    #[asset(
        paths(
            "input-prompts/keyboard/Alt/T_Brackets_L_Key_Alt.png",
            "input-prompts/keyboard/Alt/T_Brackets_R_Key_Alt.png",
            "input-prompts/keyboard/Alt/T_Crtl_Key_Alt.png",
            "input-prompts/keyboard/Alt/T_G_Key_Alt.png",
            "input-prompts/keyboard/Alt/T_Mouse_Scroll_Key_Dark_Key_Alt.png",
            "input-prompts/keyboard/Alt/T_O_Key_Alt.png",
            "input-prompts/keyboard/Alt/T_Shift_Key_Alt.png",
            "input-prompts/keyboard/Alt/T_Space_Key_Alt.png",
            "input-prompts/keyboard/Alt/T_Tab_Key_Alt.png",
            "input-prompts/keyboard/Alt/T_Tilde_Key_Alt.png",
            "input-prompts/keyboard/Alt/T_W_Key_Alt.png",
            "input-prompts/keyboard/Alt/T_X_Key_Alt.png",
            "input-prompts/keyboard/Alt/T_Z_Key_Alt.png",
        ),
        collection(mapped, typed)
    )]
    pub key_glyphs: bevy::platform::collections::HashMap<AssetFileStem, Handle<Image>>,
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
pub(crate) fn prepare_cubemap_view(
    mut images: ResMut<Assets<Image>>,
    game_assets: Res<GameAssets>,
) {
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

/// Load the game's UI sound effects into the [`SoundBank<UiSfx>`] the interface
/// consumers read - engine chrome from root `assets/sounds/` via
/// `SoundBank::load`'s convention (like `icons/`, outside every mod). World
/// sounds have NO bank: every one is an authored
/// `AssetRef<AudioSource>` on its owning content config, resolved by its cue. A
/// bank rather than `GameAssets` because the bank has no public "build from
/// existing handles" constructor; loading here kicks the (tiny) WAVs off well
/// before the first sound plays.
pub(crate) fn register_sounds(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(SoundBank::load(&assets, UI_SFX_FILES));
}

// TODO(20260525-133028): Probably need to refactor this somehow
pub(crate) fn update_nova_hud_assets(
    mut nova_hud_assets: ResMut<NovaHudAssets>,
    game_assets: Res<GameAssets>,
    images: Res<Assets<Image>>,
) {
    nova_hud_assets.target_sprite = game_assets.target_sprite.clone();
    nova_hud_assets.nova_crt_mark = game_assets.nova_crt_mark.clone();
    // Fan the stem-keyed collection out to the label-keyed lookup the HUD
    // consumes (several labels share one keycap, e.g. both Control keys).
    let mut key_glyphs = KeyGlyphs::from_stems(|stem| game_assets.key_glyphs.get(stem).cloned());
    // Then measure each cap inside its (square) canvas, so the HUD can size
    // from the ART: the wide caps are drawn wide and short, and a square box
    // threw ~40% of their height away. This runs in Processing, where the whole
    // collection is loaded, and reads pixels the image loader keeps in the main
    // world (`RenderAssetUsages::default`).
    let measured = key_glyphs.measure_caps(&images);
    if measured < key_glyphs.len() {
        warn!(
            "update_nova_hud_assets: only {measured}/{} keycap LABELS resolved a \
             cap rect (several labels share one image); the rest fall back to a \
             square box",
            key_glyphs.len()
        );
    }
    nova_hud_assets.key_glyphs = key_glyphs;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// DoD 2, half two: every keycap the HUD mapping table can resolve must
    /// appear in `GameAssets::key_glyphs`'s explicit `paths(...)` list, or that
    /// glyph would load lazily and ungated again - the exact regression closed
    /// for the UI SFX. The mapping table owns the list; this pins the attribute
    /// to it.
    #[test]
    fn key_glyph_collection_matches_mapping_table() {
        use std::collections::BTreeSet;

        let mapped: BTreeSet<String> = key_glyph_asset_paths().into_iter().collect();
        let collection: BTreeSet<String> = KEY_GLYPH_COLLECTION_PATHS
            .iter()
            .map(|p| (*p).to_string())
            .collect();
        assert_eq!(
            collection, mapped,
            "GameAssets::key_glyphs must preload exactly the keycaps the HUD \
             mapping table resolves"
        );

        for path in KEY_GLYPH_COLLECTION_PATHS {
            let full = std::path::Path::new("../../assets").join(path);
            assert!(
                full.exists(),
                "keycap glyph missing on disk: {}",
                full.display()
            );
        }
    }

    /// The `GameAssets::ui_sfx` mapped collection must load-gate exactly the
    /// files the UI `SoundBank<UiSfx>` plays: its path list
    /// ([`UI_SFX_COLLECTION_PATHS`], mirrored into the `#[asset(paths(...))]`
    /// attribute) must equal the distinct files
    /// [`nova_gameplay::audio::UI_SFX_FILES`] references, and each must exist on
    /// disk. A sound added to `UI_SFX_FILES` without a matching collection path
    /// would load lazily (ungated) again; this pins them together.
    #[test]
    fn ui_sfx_collection_matches_ui_sfx_files() {
        use std::collections::BTreeSet;

        let bank_paths: BTreeSet<String> = UI_SFX_FILES
            .iter()
            .map(|(_, name)| format!("sounds/{name}.wav"))
            .collect();
        let collection_paths: BTreeSet<String> = UI_SFX_COLLECTION_PATHS
            .iter()
            .map(|p| (*p).to_string())
            .collect();
        assert_eq!(
            collection_paths, bank_paths,
            "GameAssets::ui_sfx collection paths must cover exactly the distinct \
             files UI_SFX_FILES references"
        );

        // Every path resolves to a real file (tests run with the crate root as
        // cwd; assets live at the workspace root, like the tests/ integration rigs).
        for path in UI_SFX_COLLECTION_PATHS {
            let full = std::path::Path::new("../../assets").join(path);
            assert!(
                full.exists(),
                "UI SFX file missing on disk: {}",
                full.display()
            );
        }
    }

    /// The editor (and any other DIRECT `SkyboxConfig` insert on the preloaded
    /// `GameAssets` cubemap - `nova_editor::setup_editor_scene`) relies on
    /// `prepare_cubemap_view` having already set the Cube texture view at
    /// startup. It runs in `OnEnter(GameAssetsStates::Processing)`, before any
    /// camera spawns, so the bcs `SkyboxPlugin` observer - which sets the view
    /// only on its single-layer fallback branch - sees a ready 6-layer + Cube
    /// image and just attaches `Skybox`. Task suspected the editor was missing
    /// this view; the investigation found this system already covers it. This
    /// pins that coverage: an arrayed cubemap gets its Cube view, a
    /// single-layer one is left for the fallback. If someone drops or breaks
    /// `prepare_cubemap_view`, this fails before the editor sky silently
    /// disappears on a 16384-limit GPU.
    #[test]
    fn prepare_cubemap_view_sets_cube_view_on_the_game_assets_cubemap() {
        use bevy::{
            asset::RenderAssetUsages,
            render::render_resource::{Extent3d, TextureDimension, TextureFormat},
        };

        fn game_assets_with(cubemap: Handle<Image>) -> GameAssets {
            GameAssets {
                cubemap,
                asteroid_texture: default(),
                hull_01: default(),
                turret_yaw_01: default(),
                turret_pitch_01: default(),
                turret_barrel_01: default(),
                torpedo_bay_01: default(),
                fps_icon: default(),
                target_sprite: default(),
                nova_crt_mark: default(),
                ui_sfx: default(),
                catalog: default(),
                key_glyphs: default(),
            }
        }

        fn stacked_image() -> Image {
            Image::new_fill(
                Extent3d {
                    width: 1,
                    height: 6,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                &[0, 0, 0, 255],
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::all(),
            )
        }

        fn run_prepare_on(image: Image) -> Option<TextureViewDimension> {
            let mut app = App::new();
            app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()));
            app.init_asset::<Image>();
            app.add_systems(Update, prepare_cubemap_view);
            app.finish();
            let cubemap = app.world_mut().resource_mut::<Assets<Image>>().add(image);
            app.insert_resource(game_assets_with(cubemap.clone()));
            app.update();
            app.world()
                .resource::<Assets<Image>>()
                .get(&cubemap)
                .expect("cubemap is in Assets")
                .texture_view_descriptor
                .as_ref()
                .and_then(|descriptor| descriptor.dimension)
        }

        // A meta'd cubemap arrives 6-layer (no view yet) -> gets its Cube view.
        let mut arrayed = stacked_image();
        let _ = arrayed.reinterpret_stacked_2d_as_array(6);
        assert!(
            arrayed.texture_view_descriptor.is_none(),
            "rig sanity: a freshly-arrayed cubemap has no view yet"
        );
        assert_eq!(
            run_prepare_on(arrayed),
            Some(TextureViewDimension::Cube),
            "prepare_cubemap_view must give the arrayed cubemap its Cube view \
             (the editor's direct SkyboxConfig insert depends on this)"
        );

        // A single-layer arrival (meta not applied) is left untouched so the
        // SkyboxPlugin fallback reinterpret still runs.
        assert_eq!(
            run_prepare_on(stacked_image()),
            None,
            "a single-layer cubemap is left alone for the SkyboxPlugin fallback"
        );
    }
}
