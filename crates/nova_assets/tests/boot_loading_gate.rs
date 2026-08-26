//! Pins the two-loading-state chain that the boot loading screen depends on.
//! `nova_assets` registers TWO `bevy_asset_loader` loading states on the SINGLE
//! `GameAssetsStates` enum: `Boot` loads the boot collection then continues to
//! `Loading`, which loads the main collection then continues to `Processing`.
//! This walk works only because bevy_asset_loader keys its internal schedules
//! per state VALUE, so two loading states on one enum chain rather than
//! clobbering each other.
//!
//! This test drives that exact registration shape on the real `GameAssetsStates`
//! enum with real async image loads (one small PNG per collection, from the
//! workspace `assets/`), and asserts the machine walks
//! `Boot -> Loading -> Processing -> Loaded` with BOTH collections resolved. It
//! uses lightweight probe collections rather than the shipped `BootAssets` /
//! `GameAssets` because the full `GameAssets` (glTF `WorldAsset`s, audio, the
//! recursive mod catalog) has no headless loader set in the test suite - that
//! full walk is exercised by the render-based screenshot/probe harness and this
//! task's native-run verification. What is NEW and unit-pinnable here is the
//! chaining mechanism, which is independent of which assets the collections
//! hold.
//!
//! Asset IO reads the real workspace `assets/` (tests run with the crate root as
//! cwd), matching `tests/example_scenario.rs` and `skybox_swap_e2e.rs`.

use std::time::{Duration, Instant};

use bevy::{
    asset::AssetPlugin,
    image::{CompressedImageFormats, ImageLoader},
    prelude::*,
    state::app::StatesPlugin,
};
use bevy_asset_loader::prelude::*;
use nova_assets::prelude::GameAssetsStates;

/// Stands in for `BootAssets`: the first (boot) collection, loaded in `Boot`.
#[derive(AssetCollection, Resource)]
struct BootProbe {
    #[asset(path = "icons/fps.png")]
    #[expect(
        dead_code,
        reason = "the handle exists to make the collection load, not to be read"
    )]
    icon: Handle<Image>,
}

/// Stands in for `GameAssets`: the second (main) collection, loaded in `Loading`.
#[derive(AssetCollection, Resource)]
struct MainProbe {
    #[asset(path = "icons/target.png")]
    #[expect(
        dead_code,
        reason = "the handle exists to make the collection load, not to be read"
    )]
    sprite: Handle<Image>,
}

#[test]
fn boot_then_loading_collections_gate_in_sequence() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        StatesPlugin,
        AssetPlugin {
            file_path: "../../assets".to_string(),
            ..default()
        },
    ));
    app.init_asset::<Image>();
    app.register_asset_loader(ImageLoader::new(CompressedImageFormats::NONE));
    app.init_state::<GameAssetsStates>();

    // The exact registration shape nova_assets uses: two loading states on the
    // one enum, Boot -> Loading -> (Processing).
    app.add_loading_state(
        LoadingState::new(GameAssetsStates::Boot)
            .continue_to_state(GameAssetsStates::Loading)
            .load_collection::<BootProbe>(),
    );
    app.add_loading_state(
        LoadingState::new(GameAssetsStates::Loading)
            .continue_to_state(GameAssetsStates::Processing)
            .load_collection::<MainProbe>(),
    );
    // Mirror production's one-shot Processing -> Loaded handoff.
    app.add_systems(
        OnEnter(GameAssetsStates::Processing),
        |mut next: ResMut<NextState<GameAssetsStates>>| next.set(GameAssetsStates::Loaded),
    );
    app.finish();

    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        app.update();
        let state = app
            .world()
            .resource::<State<GameAssetsStates>>()
            .get()
            .clone();
        if state == GameAssetsStates::Loaded {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "state machine never reached Loaded (two loading states failed to \
             chain); stuck at {state:?}"
        );
        std::thread::sleep(Duration::from_millis(5));
    }

    assert!(
        app.world().contains_resource::<BootProbe>(),
        "the boot loading state must resolve its collection before continuing"
    );
    assert!(
        app.world().contains_resource::<MainProbe>(),
        "the loading state must resolve its collection after the boot chain"
    );
}
