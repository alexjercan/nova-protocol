//! perf_baseline: boot a heavy gameplay scene and capture its frame times.
//!
//! The measurement rig behind the v0.7.0 frame-time baseline
//! (tasks/20260716-123551). It boots the real gameplay app (the same plugins
//! the binary runs: physics, gravity, particles, HUD, render), loads a named
//! shipped scenario, and hands the running app to [`nova_frametime`], which
//! warms up, records the wall-clock delta of every frame for a fixed window,
//! writes percentile stats, and exits.
//!
//! Unlike a criterion microbench, this measures the WHOLE frame - render + ECS +
//! physics + gravity - of an actual loaded scene, which is what a player feels.
//! It is driven entirely by env vars (so one binary sweeps every scene x
//! renderer x preset from a shell script); the `--scenario` flag is a fallback
//! when `NOVA_PERF_SCENARIO` is unset.
//!
//! Note: the prebuilt/`cargo run` binary needs `BEVY_ASSET_ROOT="$PWD"` so Bevy
//! resolves `assets/` at the repo, not beside the executable.
//!
//! ```text
//! # Discrete GPU into a HEADLESS Xvfb window (no compositor, no visible window):
//! # the live desktop (:0) vsync-clamps and contends, so use Xvfb for clean,
//! # repeatable GPU numbers.
//! Xvfb :95 -screen 0 1280x720x24 & \
//! NOVA_PERF=1 NOVA_PERF_SCENARIO=asteroid_field NOVA_PERF_LABEL=asteroid_field-high \
//!   NOVA_PERF_OUT=/tmp/perf BEVY_ASSET_ROOT="$PWD" DISPLAY=:95 \
//!   cargo run --release --example perf_baseline --features debug
//!
//! # Software-raster floor (forced lavapipe ICD): the worst-case CPU/fill floor
//! # that brackets weak hardware. NOT a browser-WebGPU stand-in.
//! ICD=/run/opengl-driver/share/vulkan/icd.d/lvp_icd.x86_64.json
//! VK_ICD_FILENAMES=$ICD VK_DRIVER_FILES=$ICD WGPU_BACKEND=vulkan \
//!   NOVA_PERF=1 NOVA_PERF_SCENARIO=asteroid_field NOVA_PERF_LABEL=asteroid_field-sw \
//!   NOVA_PERF_OUT=/tmp/perf BEVY_ASSET_ROOT="$PWD" DISPLAY=:95 \
//!   cargo run --release --example perf_baseline --features debug
//! ```
//!
//! Extra env: `NOVA_PERF_QUALITY=low|medium|high` sweeps the graphics preset
//! (task 20260525-133013), whose `GraphicsBudget` fractions this baseline exists
//! to tune. See `nova_probe` for the capture knobs
//! (`NOVA_PERF_WARMUP` / `NOVA_PERF_FRAMES` / `NOVA_PERF_RES`).

use bevy::prelude::*;
use clap::Parser;
use nova_probe::{combat_burst_driver, nova_frametime};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "perf_baseline")]
#[command(version = "1.0.0")]
#[command(about = "Boot a heavy scenario and capture its frame-time baseline", long_about = None)]
struct Cli {
    /// Shipped scenario id to load and measure. Overridden by
    /// `NOVA_PERF_SCENARIO` when set.
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
                        "perf_baseline: scenario id '{id}' not found in GameScenarios \
                             (shipped ids live in assets/base/scenarios/*.content.ron)"
                    ),
                },
            );
        })
        .build();

    // Optional graphics-preset sweep: the baseline exists partly to tune the
    // GraphicsBudget fractions the preset drives, so let a run pick a tier.
    if let Some(quality) = std::env::var("NOVA_PERF_QUALITY")
        .ok()
        .and_then(parse_quality)
    {
        app.insert_resource(quality);
    }

    // The capture harness (from nova_probe; inert unless NOVA_PERF is set).
    // `NOVA_PERF_COMBAT=1` attaches the combat-burst driver (raise + hold fire,
    // keep combatants alive) so the capture measures particles/projectiles in
    // flight rather than the scene at rest - use it on a combat scenario.
    let capture = if std::env::var("NOVA_PERF_COMBAT").is_ok() {
        nova_frametime().drive(combat_burst_driver)
    } else {
        nova_frametime()
    };
    app.add_plugins(capture);

    // Smoke-test assertion (debug-only, in nova_debug): fail a run against a
    // typo'd id loudly instead of measuring an empty scene.
    #[cfg(feature = "debug")]
    app.add_plugins(assert_scenario_loaded(scenario_id));

    // Harness + probe wiring, UNCONDITIONAL since the completion protocol
    // (task 20260720-000609): autopilot and capture both register as
    // collectors and the app exits when BOTH are done - the old
    // `!perf_armed()` exit-ownership conditional (20260719-210443) is
    // exactly the folklore the protocol deletes. Everything here is inert
    // without its env.
    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_autopilot());
        app.add_plugins(nova_probe::nova_timeline());
        app.add_plugins(nova_probe::nova_invariants());
    }

    app.run()
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
