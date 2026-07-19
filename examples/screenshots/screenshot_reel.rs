//! screenshot_reel: film the showcase scene into the web site's pure-3D
//! screenshots.
//!
//! The reel scene is EXAMPLE-OWNED data, not a mod: its scenario RON lives at
//! `examples/screenshots/data/reel.content.ron` (never shipped - only `assets/` reaches
//! players/the web build), is embedded via `include_str!`, parsed with the same
//! `Content` type the modding loader uses, and loaded directly with
//! `LoadScenario` once assets are `Loaded`. No catalog entry, no `EnabledMods`;
//! the mod pipeline's live re-merge coverage lives in nova_assets'
//! `toggling_enabled_mods_remerges_live`. Three run modes, all off the same
//! scene:
//!
//! - `BCS_REEL=1`: the [`ScreenshotReelPlugin`] poses the camera per beat and
//!   captures each PNG (staged under `NOVA_SHOT_DIR`), then exits.
//! - `BCS_AUTOPILOT=1`: the smoke path - reach `Playing`, assert the scene loaded,
//!   exit clean.
//! - plain run: boots into the scene under the free-fly WASD camera for framing.
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_SHOT_DIR=target/reel BCS_REEL=1 cargo run --example screenshot_reel --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! BCS_AUTOPILOT=1 cargo run --example screenshot_reel --features debug
//! # look for: `nova harness: reached Playing`,
//! #           `autopilot: cycle complete, no panic`
//! ```

use bevy::prelude::*;
use clap::Parser;
use nova_modding::prelude::Content;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "screenshot_reel")]
#[command(version = "1.0.0")]
#[command(about = "Film the screenshot-reel scene into the web-site screenshots", long_about = None)]
struct Cli;

/// The embedded reel scenario source (example-owned, not shipped in assets/).
const REEL_CONTENT_RON: &str = include_str!("data/reel.content.ron");
/// The scenario id inside the embedded content (the smoke probe keys on it).
#[cfg(feature = "debug")]
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
        // Probe wiring (task 20260719-210443; each plugin is inert without
        // its NOVA_PERF_* env): run timeline + engine-bound invariants +
        // frame-time capture, so `probe run` can measure this example.
        app.add_plugins(nova_probe::nova_timeline());
        app.add_plugins(nova_probe::nova_invariants());
        app.add_plugins(nova_probe::nova_frametime());
        app.add_plugins(nova_autopilot().input(reel_smoke_probe));
        app.add_plugins(ScreenshotReelPlugin::new(reel_beats()));
    }

    app.run();
}

/// Load the embedded reel scenario once assets are `Loaded` (textures the
/// scenario references by path resolve through the normal asset server). Not
/// debug-gated: a plain run boots the scene.
fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_reel_scenario);
}

/// Parse the embedded `reel.content.ron` (a `Vec<Content>`, the same shape the
/// modding loader reads) and return its scenario. This is a dev capture tool, so
/// a broken reel file fails loud at startup rather than booting an empty scene -
/// including a file that grows items this direct path would silently drop (the
/// mod pipeline would have routed a `Section` into `GameSections`; this loads
/// exactly one scenario and nothing else).
fn parse_reel_scenario() -> ScenarioConfig {
    let items: Vec<Content> = ron::de::from_str(REEL_CONTENT_RON)
        .expect("examples/screenshots/data/reel.content.ron must parse as a Vec<Content>");
    let mut scenarios: Vec<ScenarioConfig> = items
        .into_iter()
        .map(|item| match item {
            Content::Scenario(scenario) => scenario,
            other => panic!(
                "examples/screenshots/data/reel.content.ron holds a non-Scenario item this embedded \
                 path would drop: {other:?}"
            ),
        })
        .collect();
    assert_eq!(
        scenarios.len(),
        1,
        "examples/screenshots/data/reel.content.ron must hold exactly one Scenario"
    );
    scenarios.pop().expect("length asserted above")
}

/// Trigger the reel scenario directly - no catalog entry, no `EnabledMods`, no
/// re-merge wait: the scenario config is embedded in this example.
fn load_reel_scenario(mut commands: Commands) {
    let scenario = parse_reel_scenario();
    info!(
        "screenshot-reel: loading embedded scenario '{}'",
        scenario.id
    );
    commands.trigger(LoadScenario(scenario));
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
/// the smoke probe can confirm the embedded-scenario load actually worked.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct ReelSceneLoaded(bool);

#[cfg(feature = "debug")]
fn note_scenario_loaded(loaded: On<ScenarioLoaded>, mut flag: ResMut<ReelSceneLoaded>) {
    if loaded.scenario_id == SCENARIO_ID && loaded.object_count > 0 {
        flag.0 = true;
    }
}

/// Smoke backstop: fail the `BCS_AUTOPILOT` run if the embedded scene never
/// loaded within the window (a silent parse or load regression), instead of
/// passing on `autopilot: cycle complete` alone. Checked late so the
/// `LoadScenario` trigger has time to complete.
#[cfg(feature = "debug")]
fn reel_smoke_probe(world: &mut World, elapsed: f32) {
    if elapsed > nova_protocol::nova_debug::harness::NOVA_AUTOPILOT_SECS - 0.3
        && !world.resource::<ReelSceneLoaded>().0
    {
        panic!(
            "screenshot-reel: scenario '{SCENARIO_ID}' never loaded with objects within the \
             autopilot window (embedded scenario load failed)"
        );
    }
}
