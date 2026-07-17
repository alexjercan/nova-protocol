//! Loads the real cubemap PNGs through the app's actual asset configuration
//! (`nova_core::assets_plugin`) and asserts their `.meta` loader settings
//! reinterpret the stacked image into a 6 layer array at load time.
//!
//! The sibling test in nova_assets (tests/cubemap_meta.rs) proves the meta
//! file itself works, but it builds its own `AssetPlugin` with default
//! settings - which is how the original fix shipped verified while the app's
//! `AssetMetaCheck::Never` ignored every meta on every platform (the v0.5.0
//! web build logged the single-layer canary warning). These tests pin the app
//! config: if `assets_plugin()` stops reading a shipped cubemap's meta, they
//! fail.
//!
//! `cubemap_alt.png` (broadside's sky, also dep://'d by the gauntlet and
//! ledger webmods) is pinned alongside `cubemap.png`: its path was once missing
//! from the old `meta_check` Paths set, so it loaded as a single-layer
//! 4096x24576 image (task 20260717-013440). The bcs SkyboxPlugin fallback
//! reinterpret hid that in the normal path, but a scenario teardown during the
//! PNG decode leaves the raw stacked image to be uploaded as-is - over the 16384
//! texture limit of llvmpipe/WebGL2-class GPUs, a fatal wgpu validation error.
//!
//! `mods/example/textures/nebula.png` is the example mod's OWN skybox, pinned
//! here to prove the config honors a MOD-shipped sidecar too. A per-path Paths
//! set could never list a dynamic `mods://`/`self://` path, so mod cubemaps kept
//! riding the same teardown race (task 20260717-111558); the config now uses
//! `AssetMetaCheck::Always`, which reads every asset's sidecar regardless of
//! source. This case FAILS under the old `Paths` config and passes under
//! `Always` - it is the regression pin for that switch.

use std::time::{Duration, Instant};

use bevy::{
    asset::{AssetPlugin, LoadState},
    image::{CompressedImageFormats, ImageLoader},
    prelude::*,
};

/// Loads `path` through the exact asset config the game ships and asserts the
/// image comes out of the LOADER as a 6 layer array of square faces.
fn assert_app_config_loads_as_six_layer_array(path: &str) {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin {
            // Tests run with the crate root as cwd; the asset folder lives at
            // the workspace root.
            file_path: "../../assets".to_string(),
            ..nova_core::assets_plugin()
        },
    ));
    app.init_asset::<Image>();
    app.register_asset_loader(ImageLoader::new(CompressedImageFormats::NONE));

    let asset_server = app.world().resource::<AssetServer>().clone();
    let handle: Handle<Image> = asset_server.load(path.to_string());

    // The PNG decode of a 4096x24576 image takes a few seconds in dev builds;
    // the deadline only bounds a hang.
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        app.update();
        match asset_server.load_state(&handle) {
            LoadState::Loaded => break,
            LoadState::Failed(err) => panic!("{path} failed to load: {err}"),
            _ => {}
        }
        assert!(Instant::now() < deadline, "timed out loading {path}");
        std::thread::sleep(Duration::from_millis(10));
    }

    let images = app.world().resource::<Assets<Image>>();
    let image = images.get(&handle).expect("loaded image is in Assets");
    assert_eq!(
        image.texture_descriptor.array_layer_count(),
        6,
        "the app's meta_check must apply {path}.meta's array_layout"
    );
    assert_eq!(
        image.height(),
        image.width(),
        "each cubemap face should be square after reinterpretation"
    );
}

#[test]
fn app_asset_config_loads_cubemap_as_six_layer_array() {
    assert_app_config_loads_as_six_layer_array("base/textures/cubemap.png");
}

#[test]
fn app_asset_config_loads_cubemap_alt_as_six_layer_array() {
    assert_app_config_loads_as_six_layer_array("base/textures/cubemap_alt.png");
}

/// A MOD-shipped cubemap (the example mod's own `nebula.png`, a dynamic
/// `self://`/`mods://` path no static Paths set could enumerate) must also get
/// its sidecar honored. This is the pin for the `Paths` -> `Always` switch
/// (task 20260717-111558): it fails if the config ever stops reading a
/// non-base asset's meta.
///
/// This loads the SHIPPED path (default file source, `assets/mods/example/...`).
/// The DOWNLOADED path (a `mods://` source reading a cached sidecar) is not
/// exercised here; its coverage rests on `nova_portal_gen` packaging every mod
/// file verbatim (the `.meta` included) so the sidecar lands in the cache -
/// asserted by `nova_assets/tests/mod_binary_resources.rs`.
#[test]
fn app_asset_config_loads_mod_cubemap_as_six_layer_array() {
    assert_app_config_loads_as_six_layer_array("mods/example/textures/nebula.png");
}
