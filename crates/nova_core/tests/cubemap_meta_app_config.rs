//! Loads the real `textures/cubemap.png` through the app's actual asset
//! configuration (`nova_core::assets_plugin`) and asserts its `.meta` loader
//! settings reinterpret the stacked image into a 6 layer array at load time.
//!
//! The sibling test in nova_assets (tests/cubemap_meta.rs) proves the meta
//! file itself works, but it builds its own `AssetPlugin` with default
//! settings - which is how the original fix shipped verified while the app's
//! `AssetMetaCheck::Never` ignored every meta on every platform (the v0.5.0
//! web build logged the single-layer canary warning). This test pins the app
//! config: if `assets_plugin()` stops reading the cubemap's meta, it fails.

use std::time::{Duration, Instant};

use bevy::{
    asset::{AssetPlugin, LoadState},
    image::{CompressedImageFormats, ImageLoader},
    prelude::*,
};

#[test]
fn app_asset_config_loads_cubemap_as_six_layer_array() {
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
    let handle: Handle<Image> = asset_server.load("textures/cubemap.png");

    // The PNG decode of a 4096x24576 image takes a few seconds in dev builds;
    // the deadline only bounds a hang.
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        app.update();
        match asset_server.load_state(&handle) {
            LoadState::Loaded => break,
            LoadState::Failed(err) => panic!("cubemap failed to load: {err}"),
            _ => {}
        }
        assert!(Instant::now() < deadline, "timed out loading the cubemap");
        std::thread::sleep(Duration::from_millis(10));
    }

    let images = app.world().resource::<Assets<Image>>();
    let image = images.get(&handle).expect("loaded image is in Assets");
    assert_eq!(
        image.texture_descriptor.array_layer_count(),
        6,
        "the app's meta_check must apply cubemap.png.meta's array_layout"
    );
    assert_eq!(
        image.height(),
        image.width(),
        "each cubemap face should be square after reinterpretation"
    );
}
