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
//! NOVA_SHOT_PATH=target/rs-high.png NOVA_PERF_QUALITY=high BCS_SHOT=1 \
//!   cargo run --example render_scale_shot --features debug
//! NOVA_SHOT_PATH=target/rs-low.png  NOVA_PERF_QUALITY=low  BCS_SHOT=1 \
//!   cargo run --example render_scale_shot --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`): a plain
//! run with `BCS_SHOT` reaches Playing, captures, and exits; `--scenario <id>`
//! or `NOVA_PERF_SCENARIO` picks the scene (default `asteroid_field`).

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "render_scale_shot")]
#[command(version = "1.0.0")]
#[command(about = "Capture a shipped scenario at a graphics preset (render-scale proof)", long_about = None)]
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

    // Single-shot capture (nova_debug; inert unless BCS_SHOT is set): force to
    // Playing, settle, capture the primary window, and exit. The path is the
    // real upscaled frame on Low.
    //
    // The single-shot harness forces `Playing` on the first frame (it and the
    // autopilot fight over `NextState`, so they are mutually exclusive - we take
    // the capture path, not the autopilot path). That forced transition races
    // ahead of asset loading, and the scenario only loads on
    // `OnEnter(GameAssetsStates::Loaded)`, so we settle generously
    // (`SETTLE_FRAMES`) to let the assets finish, the scenario spawn, and the
    // render-scale reconcile + upscale blit run before the frame is grabbed.
    // No `assert_scenario_loaded` here: it checks the load happened by the time
    // `Playing` is entered, which the forced-early transition guarantees to
    // violate - the loaded-scene proof is the captured PNG itself.
    // Reproduce a LIVE quality switch: NOVA_SWITCH_QUALITY sets the preset the
    // app STARTS at, then partway through the settle window we switch to
    // NOVA_PERF_QUALITY - so the captured frame shows the post-switch state, not
    // a fresh start. This is what the settings menu does at runtime.
    if let Some(target) = std::env::var("NOVA_SWITCH_QUALITY")
        .ok()
        .and_then(parse_quality)
    {
        // Start at the OTHER preset (the env-set NOVA_PERF_QUALITY above was the
        // start; here NOVA_SWITCH_QUALITY is where we end up). Insert a counter
        // and flip at frame 120 (well inside the 240-frame settle).
        app.insert_resource(SwitchTo(target));
        app.add_systems(Update, switch_quality_midway);
    }

    #[cfg(feature = "debug")]
    {
        const SETTLE_FRAMES: u32 = 240;
        let path =
            std::env::var("NOVA_SHOT_PATH").unwrap_or_else(|_| "render-scale.png".to_string());
        app.add_plugins(nova_screenshot().path(path).settle_frames(SETTLE_FRAMES));
    }

    app.run()
}

/// The preset to switch to partway through, and a frame counter.
#[derive(Resource)]
struct SwitchTo(GraphicsQuality);

/// Flip `GraphicsQuality` to the target once the scene is up (frame 150), so the
/// capture at frame ~240 shows the result of a live switch.
fn switch_quality_midway(
    mut frames: Local<u32>,
    switch: Res<SwitchTo>,
    mut done: Local<bool>,
    mut commands: Commands,
) {
    *frames += 1;
    if !*done && *frames == 150 {
        commands.insert_resource(switch.0);
        *done = true;
        info!(
            "render_scale_shot: switched quality to {:?} at frame 150",
            switch.0
        );
    }
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
