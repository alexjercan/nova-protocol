//! render_scale_shot: boot a shipped scenario at a chosen graphics preset and
//! capture the primary window to a PNG - the end-to-end proof that the
//! render-scale lever (task 20260718-004723) renders a correct frame, not a
//! black one.
//!
//! On `Low` the scenario view is drawn into a reduced-resolution offscreen
//! target and upscaled to the window by a blit camera (see
//! `nova_scenario::render_scale`); `Medium`/`High` render straight to the window.
//! Because the capture reads the primary window, a `Low` shot is the real
//! upscaled frame the player sees - world AND HUD - so a misconfigured camera
//! stack (a black or empty window) shows up immediately as a bad PNG, where
//! frame-time capture alone could not tell "fewer pixels" from "nothing drawn".
//!
//! Capture High vs Low for the same scene (windowed, real GPU):
//! ```text
//! NOVA_SHOT_PATH=rs-high.png NOVA_PERF_QUALITY=high NOVA_SHOT_DIR=target/shots \
//!   NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 cargo run --example render_scale_shot --features debug
//! NOVA_SHOT_PATH=rs-low.png  NOVA_PERF_QUALITY=low  NOVA_SHOT_DIR=target/shots \
//!   NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 cargo run --example render_scale_shot --features debug
//! ```
//!
//! Two run modes, the fleet's one capture idiom (`shoot` inside an autopilot
//! step), so the same walk drives both:
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - load the scene, settle, exit
//!   clean, capturing nothing. This is the path `probe run` takes.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write the PNG (`NOVA_SHOT_PATH`,
//!   staged under `NOVA_SHOT_DIR`).
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example render_scale_shot --features debug
//! # look for: `nova harness: reached Playing`, `autopilot: cycle complete, no panic`
//! ```
//!
//! `--scenario <id>` or `NOVA_PERF_SCENARIO` picks the scene (default
//! `asteroid_field`). The walk waits on the PLAYER SHIP, so a scenario without
//! one error-exits naming the step rather than shooting an empty frame.

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "render_scale_shot")]
#[command(version = "1.0.0")]
#[command(about = "Capture a shipped scenario at a graphics preset (render-scale proof). Autopilot-only: its product is one PNG", long_about = None)]
struct Cli {
    /// Shipped scenario id to load. Overridden by `NOVA_PERF_SCENARIO` when set.
    #[arg(long, default_value = "asteroid_field")]
    scenario: String,
}

fn main() -> bevy::app::AppExit {
    let cli = Cli::parse();
    let scenario_id = std::env::var("NOVA_PERF_SCENARIO").unwrap_or(cli.scenario);

    let loader_id = scenario_id.clone();
    let mut app = AppBuilder::new()
        .with_game_plugins(move |app: &mut App| {
            let id = loader_id.clone();
            app.add_systems(
                OnEnter(GameAssetsStates::Loaded),
                move |mut commands: Commands, scenarios: Res<GameScenarios>| match scenarios
                    .get(id.as_str())
                    .cloned()
                {
                    Some(config) => commands.trigger(LoadScenario(config)),
                    None => panic!(
                        "render_scale_shot: scenario id '{id}' not found in GameScenarios \
                         (shipped ids live in assets/base/scenarios/*.content.ron)"
                    ),
                },
            );
        })
        .build();

    // Pick the preset to capture - `Low` is the one that exercises render-scale.
    if let Some(quality) = std::env::var("NOVA_PERF_QUALITY")
        .ok()
        .and_then(parse_quality)
    {
        app.insert_resource(quality);
    }

    #[cfg(feature = "debug")]
    {
        // Probe wiring (each plugin is inert without its NOVA_PERF_* env):
        // run timeline + engine-bound invariants, so `probe run` grades this
        // example instead of asserting nothing. No frame-time capture - the
        // whole point here is the PIXELS at a preset, and the render-scale
        // lever's cost is what `probe run scene_baseline` already measures.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        // The two shots are compared against each other, so both must be the
        // same known 16:9 - a window the WM sized would make "fewer pixels"
        // unreadable. Dev overlays out of the frame; the HUD stays, because a
        // Low shot proving the upscale must show world AND HUD.
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_plugins(
            nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
                // Wait for the ship to EXIST rather than for a guessed load
                // duration: the scenario spawns on
                // `OnEnter(GameAssetsStates::Loaded)`, and a frame posed before
                // that poses nothing.
                .step("load the scene")
                .enter(GameStates::Loading)
                .until(player_ship_present())
                .deadline(30.0)
                .add()
                .step("settle the render stack")
                .until(frames(SETTLE_FRAMES))
                .add()
                // The live switch is a STEP, not a branch: `switch_quality` is
                // a no-op when `NOVA_SWITCH_QUALITY` is unset, so both modes
                // walk the same beats and settle the same number of frames -
                // two shots meant to be compared are then framed identically.
                .step("switch the preset live")
                .on_enter(switch_quality)
                .until(frames(SETTLE_FRAMES))
                .add()
                .step("capture the frame")
                .on_enter(|world| shoot(world, &shot_path()))
                .until(shot_written(shot_path()))
                .deadline(SHOT_DEADLINE_SECS)
                .add(),
        );
    }

    app.run()
}

/// Frames to hold before the shot: enough for the render-scale reconcile and
/// the upscale blit to have run over a dressed scene. The old single-shot
/// driver needed 240 because it forced `Playing` on frame one and raced the
/// asset load; the script waits on the player ship instead, so the settle only
/// has to cover the render stack.
#[cfg(feature = "debug")]
const SETTLE_FRAMES: u32 = 120;

/// Runway for the capture step. A shot ACKS (`shot_written`), so this only has
/// to cover the PNG write.
#[cfg(feature = "debug")]
const SHOT_DEADLINE_SECS: f32 = 30.0;

/// Where the shot lands: `NOVA_SHOT_PATH`, resolved under `NOVA_SHOT_DIR` when
/// relative. Read per call rather than threaded through the script, so the
/// shot step and its ack name the same string with nothing to keep in sync.
#[cfg(feature = "debug")]
fn shot_path() -> String {
    std::env::var("NOVA_SHOT_PATH").unwrap_or_else(|_| "render-scale.png".to_string())
}

/// Reproduce a LIVE quality switch, the way the settings menu does it at
/// runtime: the app STARTS at `NOVA_PERF_QUALITY` and switches to
/// `NOVA_SWITCH_QUALITY` here, so the captured frame shows the post-switch
/// state rather than a fresh start (a reconcile that tears the old target down
/// and blits through a new one, which a fresh start never exercises).
///
/// Unset, this does nothing and the step is just more settle.
#[cfg(feature = "debug")]
fn switch_quality(world: &mut World) {
    let Some(target) = std::env::var("NOVA_SWITCH_QUALITY")
        .ok()
        .and_then(parse_quality)
    else {
        return;
    };
    world.insert_resource(target);
    info!("render_scale_shot: switched quality to {target:?}");
}

/// Map a `low|medium|high` env value onto the preset. Unknown values are
/// ignored (the app keeps its default High).
fn parse_quality(value: String) -> Option<GraphicsQuality> {
    match value.to_ascii_lowercase().as_str() {
        "low" => Some(GraphicsQuality::Low),
        "medium" => Some(GraphicsQuality::Medium),
        "high" => Some(GraphicsQuality::High),
        _ => None,
    }
}
