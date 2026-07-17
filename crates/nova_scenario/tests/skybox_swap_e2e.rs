//! End-to-end proof of the `SetSkybox` modding action against a REAL cubemap on a
//! headless asset server (task 20260715-140049, follow-up to 20260525-133017 R1.1).
//!
//! The applier's deferred install is already unit-tested in `actions.rs`
//! (`skybox_swap_waits_for_load_then_installs`), but that rig uses a synthetic
//! `Image::default()` and stops at the `SkyboxConfig` insert. It never reaches the
//! LAST bridge: bevy_common_systems' `SkyboxPlugin` runs an `On<Insert, SkyboxConfig>`
//! observer that reads the loaded image, reinterprets the stacked cubemap, and
//! attaches a bevy `Skybox` to the camera. This test drives the whole chain on the
//! real `textures/cubemap_alt.png` file:
//!
//!   SetSkybox action -> NovaEventWorld command flush -> PendingSkyboxSwap
//!   -> real asset load -> apply_pending_skybox_swaps installs SkyboxConfig
//!   -> SkyboxPlugin observer -> the scenario camera's `Skybox.image` swaps.
//!
//! Modeled on `nova_assets/tests/example_scenario.rs` (real headless asset IO). Asset IO
//! reads the real workspace `assets/` (tests run with the crate root as cwd).

use std::time::{Duration, Instant};

use bevy::{
    asset::{AssetPlugin, LoadState},
    core_pipeline::Skybox,
    image::{CompressedImageFormats, ImageLoader},
    prelude::*,
    render::render_resource::TextureViewDimension,
};
use bevy_common_systems::prelude::{
    EventAction, GameEventInfo, GameEventsPlugin, GameObjectives, SkyboxConfig, SkyboxPlugin,
};
use nova_scenario::prelude::*;

/// A headless app that loads real PNGs and runs the full skybox-swap chain: the bcs
/// `SkyboxPlugin` observer, the real event flush (`GameEventsPlugin::<NovaEventWorld>`
/// runs `state_to_world_system` in `PostUpdate`), and `apply_pending_skybox_swaps`.
///
/// The applier is registered ungated here; in the shipped app it runs
/// `.run_if(scenario_is_live)` (`loader.rs`), which is an orthogonal scheduling
/// concern - this test exercises the swap behavior, not the run condition.
fn headless_skybox_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin {
            // Tests run with the crate root as cwd; assets live at the workspace root.
            file_path: "../../assets".to_string(),
            ..default()
        },
    ));
    app.init_asset::<Image>();
    app.register_asset_loader(ImageLoader::new(CompressedImageFormats::NONE));
    app.add_plugins(SkyboxPlugin);
    app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
    // `NovaEventWorld::state_to_world_system` reads this resource every flush.
    app.init_resource::<GameObjectives>();
    app.add_systems(Update, apply_pending_skybox_swaps);
    app.finish();
    app
}

/// Pump updates until `handle` reaches `Loaded`, panicking on failure or timeout. The
/// 4096x24576 PNGs take a few seconds to decode in dev builds; the deadline only
/// bounds a hang.
fn wait_loaded(app: &mut App, asset_server: &AssetServer, handle: &Handle<Image>, what: &str) {
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        app.update();
        match asset_server.load_state(handle) {
            LoadState::Loaded => break,
            LoadState::Failed(err) => panic!("{what} failed to load: {err}"),
            _ => {}
        }
        assert!(Instant::now() < deadline, "timed out loading {what}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// The full modding hook: firing `SetSkybox` with a real cubemap path swaps the
/// scenario camera's live `Skybox` to the new image, inheriting brightness.
#[test]
fn set_skybox_swaps_a_real_cubemap_on_the_scenario_camera() {
    let mut app = headless_skybox_app();
    let asset_server = app.world().resource::<AssetServer>().clone();

    // A scenario camera already showing the shipped cubemap at brightness 700.
    let initial: Handle<Image> = asset_server.load("base/textures/cubemap.png");
    wait_loaded(&mut app, &asset_server, &initial, "the initial cubemap");

    let camera = app
        .world_mut()
        .spawn((
            Camera3d::default(),
            ScenarioCameraMarker,
            SkyboxConfig {
                cubemap: initial.clone(),
                brightness: 700.0,
            },
        ))
        .id();

    // The bcs observer attaches a `Skybox` off the initial `SkyboxConfig`.
    app.update();
    assert_eq!(
        app.world()
            .get::<Skybox>(camera)
            .expect("the initial SkyboxConfig must produce a Skybox")
            .image
            .as_ref(),
        Some(&initial),
        "the camera starts on the initial cubemap"
    );

    // The swap target is a DIFFERENT real asset (distinct path -> distinct id).
    let swapped: Handle<Image> = asset_server.load("base/textures/cubemap_alt.png");
    assert_ne!(
        swapped, initial,
        "the swap must target a different cubemap than the initial one"
    );

    // Fire the real SetSkybox action through the real NovaEventWorld. Mutating the
    // resource trips `resource_changed`, so the PostUpdate event chain flushes the
    // queued command (-> PendingSkyboxSwap) exactly as it does in game.
    {
        let mut event_world = app.world_mut().resource_mut::<NovaEventWorld>();
        SetSkyboxActionConfig::new("base/textures/cubemap_alt.png")
            .action(&mut event_world, &GameEventInfo::default());
    }

    // Tick until the new cubemap has loaded and the swap has propagated all the way
    // to the camera's `Skybox.image`.
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        app.update();
        let landed = app
            .world()
            .get::<Skybox>(camera)
            .and_then(|sky| sky.image.clone())
            .is_some_and(|image| image == swapped);
        if landed {
            break;
        }
        if asset_server.load_state(&swapped).is_failed() {
            panic!("the swapped cubemap failed to load");
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for the skybox swap to reach the camera"
        );
        std::thread::sleep(Duration::from_millis(10));
    }

    // The swap fully landed: the pending tag is consumed, the config points at the
    // new cubemap, and brightness was inherited (the action set none).
    assert!(
        app.world().get::<PendingSkyboxSwap>(camera).is_none(),
        "the pending swap must be consumed once the cubemap is installed"
    );
    let config = app
        .world()
        .get::<SkyboxConfig>(camera)
        .expect("SkyboxConfig present after the swap");
    assert_eq!(
        config.cubemap, swapped,
        "SkyboxConfig points at the new cubemap"
    );
    assert_eq!(
        config.brightness, 700.0,
        "brightness is inherited when the action does not override it"
    );
    let skybox = app.world().get::<Skybox>(camera).expect("Skybox present");
    assert_eq!(
        skybox.image.as_ref(),
        Some(&swapped),
        "the live Skybox.image handle swapped to the new cubemap"
    );
    assert_ne!(
        skybox.image.as_ref(),
        Some(&initial),
        "the live Skybox no longer shows the initial cubemap"
    );
    assert_eq!(
        skybox.brightness, 700.0,
        "the installed Skybox carries the inherited brightness"
    );

    // And the swapped image is actually renderable as a skybox: 6 array layers
    // (here from the `.meta` array_layout - this rig's default meta_check reads
    // every meta) AND a Cube texture view. The view is the applier's job: an
    // already-arrayed image skips the bcs observer's fallback branch that used
    // to be the only place the view was set, and bevy's skybox sanity check
    // refuses a non-Cube view (warn_once) and silently skips rendering the
    // sky (task 20260717-013440).
    let images = app.world().resource::<Assets<Image>>();
    let image = images.get(&swapped).expect("swapped cubemap is in Assets");
    assert_eq!(
        image.texture_descriptor.array_layer_count(),
        6,
        "the swapped cubemap must be a 6 layer array"
    );
    assert_eq!(
        image
            .texture_view_descriptor
            .as_ref()
            .and_then(|descriptor| descriptor.dimension),
        Some(TextureViewDimension::Cube),
        "the swapped cubemap must carry a Cube texture view"
    );
}
