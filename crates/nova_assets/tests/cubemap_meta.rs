//! Loads the real `textures/cubemap.png` through a headless asset server and
//! asserts its `.meta` loader settings reinterpret the stacked image into a
//! 6 layer array at load time.
//!
//! This guards the skybox upload race: the renderer eagerly uploads every
//! loaded image, and the raw stacked form (24576 px tall) exceeds the 16384
//! texture limit of smaller GPUs, turning into a fatal render validation
//! error. Reinterpreting in the loader means the oversized 2D form never
//! exists. If someone deletes or breaks `cubemap.png.meta`, this test fails.
//!
//! It proves the meta FILE, not the app: this rig uses `AssetPlugin`'s
//! default `meta_check` (Always), while the shipped app reads metas per-path
//! (`nova_core::assets_plugin`). The test that guards the real app's config
//! is nova_core/tests/cubemap_meta_app_config.rs - the gap between the two
//! is exactly how the meta shipped ignored (task 20260713-175416).

use std::time::{Duration, Instant};

use bevy::{
    asset::{AssetPlugin, LoadState},
    image::{CompressedImageFormats, ImageLoader},
    prelude::*,
};

#[test]
fn cubemap_meta_loads_six_layer_array() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin {
            // Tests run with the crate root as cwd; the asset folder lives at
            // the workspace root.
            file_path: "../../assets".to_string(),
            ..default()
        },
    ));
    app.init_asset::<Image>();
    app.register_asset_loader(ImageLoader::new(CompressedImageFormats::NONE));

    let asset_server = app.world().resource::<AssetServer>().clone();
    let handle: Handle<Image> = asset_server.load("base/textures/cubemap.png");

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
        "cubemap.png.meta should reinterpret the stacked image into 6 layers"
    );
    assert_eq!(
        image.height(),
        image.width(),
        "each cubemap face should be square after reinterpretation"
    );
}
