//! Frame-time capture harness for gameplay performance baselines.
//!
//! A single env-gated plugin, [`nova_frametime`], that drives a real gameplay
//! app to `Playing`, warms up, records the wall-clock delta of every frame for a
//! fixed window, then writes percentile frame-time stats (JSON + a CSV row) and
//! exits cleanly with [`AppExit::Success`]. Inert unless `NOVA_PERF` is set, so
//! an example adds it permanently and pays nothing in a normal run - the same
//! contract the [`nova_autopilot`](crate::harness::nova_autopilot) /
//! [`nova_screenshot`](crate::harness::nova_screenshot) presets follow.
//!
//! ## Why measure this way
//!
//! - **Real frame delta, not the diagnostics store.** The capture reads
//!   [`Time<Real>`] deltas directly: wall-clock time between frames, unaffected
//!   by the fixed-timestep clamp or a paused virtual clock. That is the number a
//!   player feels.
//! - **Vsync off.** [`PresentMode::AutoNoVsync`] is forced on the primary window
//!   so a fast scene is not pinned to the monitor's refresh - we want the true
//!   per-frame cost and the headroom, not "60 fps, capped". A scene that cannot
//!   hold refresh shows its real (sub-refresh) rate either way.
//! - **Continuous updates.** [`WinitSettings::game`] keeps the loop running flat
//!   out even when the window is unfocused (the headless/Xvfb case), so the
//!   capture is not throttled to reactive redraws.
//! - **Fixed resolution.** The window is forced to a known size (default
//!   1280x720) so runs are comparable across machines and renderers.
//!
//! ## Usage
//!
//! ```no_run
//! # use bevy::prelude::*;
//! # use nova_debug::perf::nova_frametime;
//! # fn add(app: &mut App) {
//! app.add_plugins(nova_frametime());
//! # }
//! ```
//!
//! Run it (needs a display; use the real GPU on `:0`, or `Xvfb` for the
//! software-raster floor that proxies the constrained web target):
//!
//! ```text
//! NOVA_PERF=1 NOVA_PERF_SCENARIO=asteroid_field NOVA_PERF_LABEL=asteroid_field-gpu \
//!   NOVA_PERF_OUT=/tmp/perf cargo run --example 20_perf_baseline --features debug
//! # look for: `nova perf: label=... frames=... mean=..ms p99=..ms mean_fps=.. 1%low_fps=..`
//! ```
//!
//! ## Environment
//!
//! | Var | Default | Meaning |
//! |-----|---------|---------|
//! | `NOVA_PERF`         | (unset) | Arms the plugin. Any value, even empty. |
//! | `NOVA_PERF_WARMUP`  | `180`   | Frames discarded after reaching `Playing` before capture (shader compile, asset upload, first-frame spikes). |
//! | `NOVA_PERF_FRAMES`  | `900`   | Frames captured for the stats window. |
//! | `NOVA_PERF_LABEL`   | `scene` | Label recorded in the JSON/CSV row. |
//! | `NOVA_PERF_OUT`     | (none)  | Directory for `<label>.json` and an appended row in `frametime.csv`. When unset, only the summary log line is emitted (the web/wasm path). |
//! | `NOVA_PERF_RES`     | `1280x720` | Forced primary-window resolution `WxH`. |

use std::path::PathBuf;

use bevy::{
    prelude::*,
    window::{PresentMode, PrimaryWindow},
    winit::WinitSettings,
};
use nova_gameplay::GameStates;

/// Environment variable that arms [`nova_frametime`]. Any value (even empty)
/// enables it; when unset the plugin adds nothing.
pub const PERF_ENV: &str = "NOVA_PERF";

/// Default warm-up frames discarded before the capture window opens.
pub const DEFAULT_WARMUP_FRAMES: u32 = 180;

/// Default number of frames captured into the stats window.
pub const DEFAULT_CAPTURE_FRAMES: u32 = 900;

/// Default forced primary-window resolution.
pub const DEFAULT_RESOLUTION: (f32, f32) = (1280.0, 720.0);

/// Env-gated frame-time capture preset for nova examples. See the module docs.
/// Inert unless `NOVA_PERF` is set.
pub fn nova_frametime() -> FrameTimePlugin {
    FrameTimePlugin
}

/// Plugin returned by [`nova_frametime`]. Construct it through that preset.
pub struct FrameTimePlugin;

/// Capture configuration, resolved once from the environment at plugin build.
#[derive(Resource, Clone, Debug)]
struct PerfConfig {
    warmup_frames: u32,
    capture_frames: u32,
    label: String,
    out_dir: Option<PathBuf>,
    resolution: (f32, f32),
}

impl PerfConfig {
    /// Read the config from the environment, falling back to the documented
    /// defaults for anything unset or unparseable.
    fn from_env() -> Self {
        fn parse_u32(key: &str, default: u32) -> u32 {
            std::env::var(key)
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(default)
        }
        let resolution = std::env::var("NOVA_PERF_RES")
            .ok()
            .and_then(|v| parse_resolution(&v))
            .unwrap_or(DEFAULT_RESOLUTION);
        Self {
            warmup_frames: parse_u32("NOVA_PERF_WARMUP", DEFAULT_WARMUP_FRAMES),
            capture_frames: parse_u32("NOVA_PERF_FRAMES", DEFAULT_CAPTURE_FRAMES),
            label: std::env::var("NOVA_PERF_LABEL").unwrap_or_else(|_| "scene".to_string()),
            out_dir: std::env::var("NOVA_PERF_OUT")
                .ok()
                .filter(|s| !s.is_empty())
                .map(PathBuf::from),
            resolution,
        }
    }
}

/// Parse a `WxH` resolution string (e.g. `1280x720`).
fn parse_resolution(value: &str) -> Option<(f32, f32)> {
    let (w, h) = value.split_once(['x', 'X'])?;
    Some((w.trim().parse().ok()?, h.trim().parse().ok()?))
}

/// The capture phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// Waiting for the asset loader to reach `Playing`.
    WaitPlaying,
    /// In `Playing`, discarding warm-up frames.
    Warmup,
    /// Recording frame deltas.
    Capture,
    /// Stats written, exit requested.
    Done,
}

/// Live capture state.
#[derive(Resource)]
struct PerfState {
    phase: Phase,
    warmed: u32,
    /// Per-frame wall-clock deltas, milliseconds.
    samples: Vec<f64>,
}

impl Plugin for FrameTimePlugin {
    fn build(&self, app: &mut App) {
        if std::env::var(PERF_ENV).is_err() {
            return;
        }
        let config = PerfConfig::from_env();
        info!(
            "nova perf: armed (label={}, warmup={}, frames={}, res={}x{}, out={:?})",
            config.label,
            config.warmup_frames,
            config.capture_frames,
            config.resolution.0,
            config.resolution.1,
            config.out_dir,
        );
        app.insert_resource(PerfState {
            phase: Phase::WaitPlaying,
            warmed: 0,
            samples: Vec::with_capacity(config.capture_frames as usize),
        });
        app.insert_resource(config);
        // Continuous updates so an unfocused/headless window still runs flat out.
        app.insert_resource(WinitSettings::game());
        app.add_systems(Startup, perf_force_window);
        app.add_systems(Update, perf_capture);
    }
}

/// Force the primary window to the capture resolution with vsync off, so every
/// run measures the true per-frame cost at a known, comparable size.
fn perf_force_window(
    config: Res<PerfConfig>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let Ok(mut window) = windows.single_mut() else {
        return;
    };
    window
        .resolution
        .set(config.resolution.0, config.resolution.1);
    window.present_mode = PresentMode::AutoNoVsync;
    window.resizable = false;
}

/// Advance the capture state machine one frame: wait for `Playing`, discard
/// warm-up frames, record deltas, then compute + emit stats and exit.
fn perf_capture(
    time: Res<Time<Real>>,
    state_res: Res<State<GameStates>>,
    config: Res<PerfConfig>,
    mut state: ResMut<PerfState>,
    mut exit: MessageWriter<AppExit>,
) {
    match state.phase {
        Phase::WaitPlaying => {
            if *state_res.get() == GameStates::Playing {
                state.phase = Phase::Warmup;
            }
        }
        Phase::Warmup => {
            state.warmed += 1;
            if state.warmed >= config.warmup_frames {
                state.phase = Phase::Capture;
                info!(
                    "nova perf: warm-up done, capturing {} frames",
                    config.capture_frames
                );
            }
        }
        Phase::Capture => {
            state.samples.push(time.delta_secs_f64() * 1000.0);
            if state.samples.len() as u32 >= config.capture_frames {
                let stats = FrameStats::from_samples(&state.samples);
                emit_stats(&config, &stats);
                state.phase = Phase::Done;
                exit.write(AppExit::Success);
            }
        }
        Phase::Done => {}
    }
}

/// Percentile frame-time statistics over a capture window. Frame times are in
/// milliseconds; the derived FPS figures are `1000 / ms`.
#[derive(Debug, Clone, PartialEq)]
pub struct FrameStats {
    /// Number of frames captured.
    pub frames: usize,
    /// Total wall-clock time of the capture window (ms).
    pub total_ms: f64,
    /// Mean frame time (ms).
    pub mean_ms: f64,
    /// Fastest (smallest) frame time (ms).
    pub min_ms: f64,
    /// Slowest (largest) frame time (ms).
    pub max_ms: f64,
    /// Median frame time (ms).
    pub p50_ms: f64,
    /// 95th-percentile frame time (ms).
    pub p95_ms: f64,
    /// 99th-percentile frame time (ms).
    pub p99_ms: f64,
    /// 99.9th-percentile frame time (ms).
    pub p999_ms: f64,
    /// Average frame rate (`1000 / mean_ms`).
    pub mean_fps: f64,
    /// "1% low" frame rate: the rate of the 99th-percentile-slowest frame
    /// (`1000 / p99_ms`), the standard stutter-floor figure.
    pub one_pct_low_fps: f64,
}

impl FrameStats {
    /// Compute stats from a slice of per-frame times in milliseconds. Pure and
    /// order-independent (it sorts a copy), so it is unit-testable without an
    /// app. Percentiles use the nearest-rank method on the ascending sort, so
    /// `pXX` is a real observed frame time, never an interpolated value.
    pub fn from_samples(samples: &[f64]) -> Self {
        assert!(!samples.is_empty(), "FrameStats needs at least one sample");
        let mut sorted = samples.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).expect("frame times are never NaN"));
        let n = sorted.len();
        let total_ms: f64 = sorted.iter().sum();
        let mean_ms = total_ms / n as f64;

        // Nearest-rank: the smallest value at or above the p-th percentile.
        let percentile = |p: f64| -> f64 {
            let rank = (p / 100.0 * n as f64).ceil() as usize;
            let idx = rank.saturating_sub(1).min(n - 1);
            sorted[idx]
        };

        Self {
            frames: n,
            total_ms,
            mean_ms,
            min_ms: sorted[0],
            max_ms: sorted[n - 1],
            p50_ms: percentile(50.0),
            p95_ms: percentile(95.0),
            p99_ms: percentile(99.0),
            p999_ms: percentile(99.9),
            mean_fps: 1000.0 / mean_ms,
            one_pct_low_fps: 1000.0 / percentile(99.0),
        }
    }

    /// A compact, greppable one-line summary.
    fn summary_line(&self, label: &str) -> String {
        format!(
            "nova perf: label={} frames={} mean={:.3}ms p50={:.3}ms p95={:.3}ms \
             p99={:.3}ms p999={:.3}ms min={:.3}ms max={:.3}ms mean_fps={:.1} 1%low_fps={:.1}",
            label,
            self.frames,
            self.mean_ms,
            self.p50_ms,
            self.p95_ms,
            self.p99_ms,
            self.p999_ms,
            self.min_ms,
            self.max_ms,
            self.mean_fps,
            self.one_pct_low_fps,
        )
    }

    /// Render as a pretty JSON object (hand-formatted to avoid a serde dep in
    /// this dev-only crate).
    fn to_json(&self, label: &str) -> String {
        format!(
            "{{\n  \"label\": \"{}\",\n  \"frames\": {},\n  \"total_ms\": {:.3},\n  \
             \"mean_ms\": {:.4},\n  \"min_ms\": {:.4},\n  \"max_ms\": {:.4},\n  \
             \"p50_ms\": {:.4},\n  \"p95_ms\": {:.4},\n  \"p99_ms\": {:.4},\n  \
             \"p999_ms\": {:.4},\n  \"mean_fps\": {:.2},\n  \"one_pct_low_fps\": {:.2}\n}}\n",
            label,
            self.frames,
            self.total_ms,
            self.mean_ms,
            self.min_ms,
            self.max_ms,
            self.p50_ms,
            self.p95_ms,
            self.p99_ms,
            self.p999_ms,
            self.mean_fps,
            self.one_pct_low_fps,
        )
    }

    /// One CSV row (no header): matches [`CSV_HEADER`].
    fn to_csv_row(&self, label: &str) -> String {
        format!(
            "{},{},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{:.2},{:.2}\n",
            label,
            self.frames,
            self.mean_ms,
            self.min_ms,
            self.max_ms,
            self.p50_ms,
            self.p95_ms,
            self.p99_ms,
            self.p999_ms,
            self.mean_fps,
            self.one_pct_low_fps,
        )
    }
}

/// Header row for the aggregated CSV, written once when the file is created.
const CSV_HEADER: &str =
    "label,frames,mean_ms,min_ms,max_ms,p50_ms,p95_ms,p99_ms,p999_ms,mean_fps,one_pct_low_fps\n";

/// Log the summary line and, when `NOVA_PERF_OUT` is set, write a per-run JSON
/// file and append a row to the aggregated CSV. The log line is always emitted -
/// on wasm there is no filesystem, so a headless-browser driver scrapes it from
/// the console.
fn emit_stats(config: &PerfConfig, stats: &FrameStats) {
    info!("{}", stats.summary_line(&config.label));

    let Some(dir) = &config.out_dir else {
        return;
    };
    if let Err(error) = std::fs::create_dir_all(dir) {
        warn!("nova perf: could not create out dir {:?}: {error}", dir);
        return;
    }

    let json_path = dir.join(format!("{}.json", sanitize(&config.label)));
    if let Err(error) = std::fs::write(&json_path, stats.to_json(&config.label)) {
        warn!("nova perf: could not write {:?}: {error}", json_path);
    } else {
        info!("nova perf: wrote {:?}", json_path);
    }

    let csv_path = dir.join("frametime.csv");
    let need_header = !csv_path.exists();
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&csv_path)
    {
        Ok(mut file) => {
            use std::io::Write;
            let mut buf = String::new();
            if need_header {
                buf.push_str(CSV_HEADER);
            }
            buf.push_str(&stats.to_csv_row(&config.label));
            if let Err(error) = file.write_all(buf.as_bytes()) {
                warn!("nova perf: could not append {:?}: {error}", csv_path);
            }
        }
        Err(error) => warn!("nova perf: could not open {:?}: {error}", csv_path),
    }
}

/// Make a label safe for a filename (keep alnum, dash, underscore, dot).
fn sanitize(label: &str) -> String {
    label
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_resolution_accepts_wxh() {
        assert_eq!(parse_resolution("1280x720"), Some((1280.0, 720.0)));
        assert_eq!(parse_resolution("1920X1080"), Some((1920.0, 1080.0)));
        assert_eq!(parse_resolution("garbage"), None);
        assert_eq!(parse_resolution("1280x"), None);
    }

    #[test]
    fn stats_on_a_uniform_window_are_exact() {
        // Ten identical 10 ms frames: every percentile is 10 ms, 100 fps.
        let stats = FrameStats::from_samples(&[10.0; 10]);
        assert_eq!(stats.frames, 10);
        assert!((stats.mean_ms - 10.0).abs() < 1e-9);
        assert!((stats.p50_ms - 10.0).abs() < 1e-9);
        assert!((stats.p99_ms - 10.0).abs() < 1e-9);
        assert!((stats.mean_fps - 100.0).abs() < 1e-6);
        assert!((stats.one_pct_low_fps - 100.0).abs() < 1e-6);
    }

    #[test]
    fn percentiles_use_nearest_rank_on_a_known_ramp() {
        // 1..=100 ms. Nearest-rank: p50 -> rank 50 -> 50 ms, p95 -> 95 ms,
        // p99 -> 99 ms, p99.9 -> rank ceil(99.9) = 100 -> 100 ms.
        let samples: Vec<f64> = (1..=100).map(|i| i as f64).collect();
        let stats = FrameStats::from_samples(&samples);
        assert_eq!(stats.min_ms, 1.0);
        assert_eq!(stats.max_ms, 100.0);
        assert_eq!(stats.p50_ms, 50.0);
        assert_eq!(stats.p95_ms, 95.0);
        assert_eq!(stats.p99_ms, 99.0);
        assert_eq!(stats.p999_ms, 100.0);
        // 1% low uses the p99 frame (99 ms) -> ~10.1 fps.
        assert!((stats.one_pct_low_fps - 1000.0 / 99.0).abs() < 1e-6);
    }

    #[test]
    fn stats_are_order_independent() {
        let ascending: Vec<f64> = (1..=50).map(|i| i as f64).collect();
        let mut shuffled = ascending.clone();
        shuffled.reverse();
        assert_eq!(
            FrameStats::from_samples(&ascending),
            FrameStats::from_samples(&shuffled)
        );
    }

    #[test]
    fn sanitize_replaces_path_hostile_chars() {
        assert_eq!(sanitize("asteroid_field-gpu"), "asteroid_field-gpu");
        assert_eq!(sanitize("a/b c:d"), "a_b_c_d");
    }
}
