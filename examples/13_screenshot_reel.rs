//! 13_screenshot_reel: film the `screenshot-reel` mod's showcase scene into the
//! web site's pure-3D screenshots.
//!
//! Boot: enable the `screenshot-reel` mod once assets are `Loaded` (the live
//! re-merge folds its scenario into `GameScenarios`, robust against a saved
//! mod-prefs file that `load_enabled_mods` would otherwise restore over a startup
//! insert), then load `screenshot_reel` as soon as it registers, and reach
//! `Playing`. Three run modes, all off the same scene:
//!
//! - `BCS_REEL=1`: the [`ScreenshotReelPlugin`] poses the camera per beat and
//!   captures each PNG (staged under `NOVA_SHOT_DIR`), then exits.
//! - `BCS_AUTOPILOT=1`: the smoke path - reach `Playing`, assert the scene loaded,
//!   exit clean.
//! - plain run: boots into the scene under the free-fly WASD camera for framing.
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_SHOT_DIR=target/reel BCS_REEL=1 cargo run --example 13_screenshot_reel --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! BCS_AUTOPILOT=1 cargo run --example 13_screenshot_reel --features debug
//! # look for: `nova harness: reached Playing`,
//! #           `autopilot: cycle complete, no panic`
//! ```

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "13_screenshot_reel")]
#[command(version = "1.0.0")]
#[command(about = "Film the screenshot-reel scene into the web-site screenshots", long_about = None)]
struct Cli;

/// The mod whose scenario supplies the capture scene (catalog id).
const MOD_ID: &str = "screenshot-reel";
/// The scenario the mod registers.
const SCENARIO_ID: &str = "screenshot_reel";

fn main() {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    // Headless harness (inert without its env var): the autopilot smoke path,
    // and the reel that captures the shots under BCS_REEL.
    #[cfg(feature = "debug")]
    {
        app.init_resource::<ReelSceneLoaded>();
        app.add_observer(note_scenario_loaded);
        app.add_plugins(nova_autopilot().input(reel_smoke_probe));
        app.add_plugins(ScreenshotReelPlugin::new(reel_beats()));
    }

    app.run();
}

/// Enable the reel mod when assets are ready, and load its scenario once the
/// live re-merge has registered it. Not debug-gated: a plain run boots the scene.
fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), enable_reel_mod);
    app.add_systems(
        Update,
        load_reel_scenario
            .run_if(resource_exists::<GameScenarios>)
            .run_if(not(resource_exists::<ReelLoaded>)),
    );
}

/// Turn on the reel mod. Done at `OnEnter(Loaded)` - after the whole
/// `OnEnter(Processing)` registration chain (including `load_enabled_mods`,
/// which would clobber an earlier insert with a saved set) - so the
/// `resource_changed::<EnabledMods>` re-merge folds the mod in next.
fn enable_reel_mod(mut enabled: ResMut<EnabledMods>) {
    enabled.0.insert(MOD_ID.to_string());
    info!("screenshot-reel: enabled mod '{MOD_ID}'");
}

/// Marker: the reel scenario has been requested, so the loader system stops.
#[derive(Resource)]
struct ReelLoaded;

/// Once assets are `Loaded` and the re-merge has put `screenshot_reel` into
/// `GameScenarios`, load it. Polls (returns early until ready) so it never races
/// the live `register_bundles` re-merge.
fn load_reel_scenario(
    mut commands: Commands,
    scenarios: Res<GameScenarios>,
    assets_state: Res<State<GameAssetsStates>>,
) {
    if *assets_state.get() != GameAssetsStates::Loaded {
        return;
    }
    let Some(scenario) = scenarios.get(SCENARIO_ID) else {
        return; // the mod's re-merge has not landed yet; retry next frame
    };
    commands.trigger(LoadScenario(scenario.clone()));
    commands.insert_resource(ReelLoaded);
    info!("screenshot-reel: loading scenario '{SCENARIO_ID}'");
}

/// The reel: an ordered list of framed beats. Each poses the scenario camera and
/// captures a PNG (staged under `NOVA_SHOT_DIR`). Framings are a first pass -
/// eyeball and tune against the real render. Only the shots a pure-3D scene can
/// make live here; HUD/menu/editor/combat shots come in a later phase.
#[cfg(feature = "debug")]
fn reel_beats() -> Vec<ReelBeat> {
    vec![
        // Ship in the foreground with the whole planetoid behind it (the gravity
        // beat): looking between the ship and the planetoid so both read.
        // Frame the SHIP (look at the origin); the planetoid reads as a large
        // body off to the side, not a centered rock face.
        ReelBeat::new(
            ReelCamera::new(Vec3::new(-6.0, 5.0, 15.0), Vec3::ZERO),
            "feature-gravity.png",
        ),
        // The planetoid as the gravity-well subject, backed off so it is a whole
        // body with the field behind, not a surface closeup.
        ReelBeat::new(
            ReelCamera::new(Vec3::new(-2.0, 8.0, 22.0), Vec3::new(24.0, 0.0, -6.0)),
            "wiki-gravity.png",
        ),
        // A ship beauty pass, backed off so the whole hull + sections read.
        ReelBeat::new(
            ReelCamera::new(Vec3::new(8.0, 4.5, 12.0), Vec3::ZERO),
            "wiki-sections.png",
        ),
    ]
}

/// Set once `ScenarioLoaded` fires for the reel scenario with real content, so
/// the smoke probe can confirm the mod-driven load actually worked.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct ReelSceneLoaded(bool);

#[cfg(feature = "debug")]
fn note_scenario_loaded(loaded: On<ScenarioLoaded>, mut flag: ResMut<ReelSceneLoaded>) {
    if loaded.scenario_id == SCENARIO_ID && loaded.object_count > 0 {
        flag.0 = true;
    }
}

/// Smoke backstop: fail the `BCS_AUTOPILOT` run if the mod-driven scene never
/// loaded within the window (a silent mod-enable or registration regression),
/// instead of passing on `autopilot: cycle complete` alone. Checked late so the
/// enable -> re-merge -> load chain has time to complete.
#[cfg(feature = "debug")]
fn reel_smoke_probe(world: &mut World, elapsed: f32) {
    if elapsed > nova_protocol::nova_debug::harness::NOVA_AUTOPILOT_SECS - 0.3
        && !world.resource::<ReelSceneLoaded>().0
    {
        panic!(
            "screenshot-reel: scenario '{SCENARIO_ID}' never loaded with objects within the \
             autopilot window (mod enable or registration failed)"
        );
    }
}
