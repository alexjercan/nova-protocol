//! perf_web: frame-time capture binary for the web/WebGPU build (and native).
//!
//! Same measurement as `examples/20_perf_baseline`, but its config comes from
//! the cross-platform perf-param source ([`perf_param`]: the URL query string on
//! wasm, `NOVA_PERF_*` env vars on native), so Trunk can build it into the wasm
//! bundle and a headless browser can drive it by URL. The capture summary is
//! logged to the console (`nova perf: label=...`) - on web there is no
//! filesystem, so a browser driver scrapes that console line.
//!
//! Built for web via `scripts/perf-web.sh` / `perf.html`; no `debug` feature
//! needed (the harness lives in this crate, `nova_probe`, not `nova_debug`).
//!
//! Query/env params (all optional): `scenario`, `quality` (low|medium|high),
//! `combat` (present = drive a combat burst), plus the capture knobs the plugin
//! reads (`warmup`, `frames`, `label`, `res`; `perf` arms it on web).

use bevy::prelude::*;
use nova_probe::{combat_burst_driver, nova_frametime, perf_param};
use nova_protocol::prelude::*;

fn main() {
    let scenario_id = perf_param("scenario").unwrap_or_else(|| "asteroid_field".to_string());

    let loader_id = scenario_id.clone();
    let mut app = AppBuilder::new()
        .with_game_plugins(move |app: &mut App| {
            let id = loader_id.clone();
            app.add_systems(
                OnEnter(GameAssetsStates::Loaded),
                move |mut commands: Commands, scenarios: Res<GameScenarios>| {
                    match scenarios.get(id.as_str()).cloned() {
                        Some(config) => commands.trigger(LoadScenario(config)),
                        // A browser panic is a dead canvas; log and let the
                        // capture time out visibly instead.
                        None => error!("perf_web: scenario '{id}' not found in GameScenarios"),
                    }
                },
            );
        })
        .build();

    if let Some(quality) = perf_param("quality").and_then(parse_quality) {
        app.insert_resource(quality);
    }

    let capture = if perf_param("combat").is_some() {
        nova_frametime().drive(combat_burst_driver)
    } else {
        nova_frametime()
    };
    app.add_plugins(capture);

    app.run();
}

/// Map a `low|medium|high` value onto the preset; unknown values keep the
/// default High.
fn parse_quality(value: String) -> Option<GraphicsQuality> {
    match value.to_ascii_lowercase().as_str() {
        "low" => Some(GraphicsQuality::Low),
        "medium" => Some(GraphicsQuality::Medium),
        "high" => Some(GraphicsQuality::High),
        _ => None,
    }
}
