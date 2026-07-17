//! Frame-time capture harness for gameplay performance baselines.
//!
//! A single env-gated plugin, [`nova_frametime`], that drives a real gameplay
//! app to `Playing`, warms up, records the wall-clock delta of every frame for a
//! fixed window, then writes percentile frame-time stats (JSON + a CSV row) and
//! exits cleanly with [`AppExit::Success`]. Inert unless `NOVA_PERF` is set, so
//! an example adds it permanently and pays nothing in a normal run - the same
//! contract the `nova_autopilot` / `nova_screenshot` harness presets (in
//! `nova_debug`) follow.
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
//! Chain [`drive`](FrameTimePlugin::drive) (e.g. [`combat_burst_driver`]) to
//! measure an active scene - particle bursts and projectiles - not just at rest.
//!
//! ## Usage
//!
//! ```no_run
//! # use bevy::prelude::*;
//! # use nova_perf::nova_frametime;
//! # fn add(app: &mut App) {
//! app.add_plugins(nova_frametime());
//! # }
//! ```
//!
//! Run it (needs a display; use the real GPU headless via `Xvfb`, or force the
//! lavapipe software-raster floor - see `scripts/perf-baseline.sh`):
//!
//! ```text
//! Xvfb :95 -screen 0 1280x720x24 &
//! NOVA_PERF=1 NOVA_PERF_SCENARIO=asteroid_field NOVA_PERF_LABEL=asteroid_field-gpu \
//!   NOVA_PERF_OUT=/tmp/perf BEVY_ASSET_ROOT="$PWD" DISPLAY=:95 \
//!   cargo run --release --example 20_perf_baseline --features debug
//! # look for: `nova perf: label=... frames=... mean=..ms p99=..ms mean_fps=.. 1%low_fps=..`
//! ```
//!
//! ## Config source
//!
//! Parameters come from [`perf_param`]: **native** reads env vars
//! `NOVA_PERF_<UPPER>`; **wasm** reads the URL query `<name>` (so a browser drives
//! it by URL - see `scripts/perf-web.sh`). The knobs:
//!
//! | Native env / wasm query | Default | Meaning |
//! |-------------------------|---------|---------|
//! | `NOVA_PERF` / `?perf`         | (unset) | Arms the plugin. |
//! | `NOVA_PERF_WARMUP` / `warmup` | `180`   | Frames discarded after reaching `Playing` (shader compile, asset upload, spikes; also lets a combat burst saturate). |
//! | `NOVA_PERF_FRAMES` / `frames` | `900`   | Frames captured for the stats window. |
//! | `NOVA_PERF_LABEL` / `label`   | `scene` | Label recorded in the row. |
//! | `NOVA_PERF_OUT` / (n/a)       | (none)  | Native only: dir for `<label>.json` + a `frametime.csv` row. Web has no fs, so it logs the summary line only. |
//! | `NOVA_PERF_RES` / `res`       | `1280x720` | Forced primary-window resolution `WxH`. |

use std::{path::PathBuf, sync::Arc};

use bevy::{
    prelude::*,
    window::{PresentMode, PrimaryWindow},
    winit::WinitSettings,
};
// Health is re-exported by nova_gameplay, so nova_perf pins the same
// bevy_common_systems version the game uses (no direct bcs dep, no version skew).
use nova_gameplay::{
    bevy_common_systems::health::Health,
    prelude::{PlayerSpaceshipMarker, WeaponsHot},
    GameStates,
};

/// Environment variable that arms [`nova_frametime`] on native. Any value (even
/// empty) enables it; when unset the plugin adds nothing. On wasm the arm is the
/// `?perf` URL query flag instead (there are no process env vars in a browser).
pub const PERF_ENV: &str = "NOVA_PERF";

/// A per-frame combat/scene driver run under [`FrameTimePlugin::drive`]: given
/// `&mut World` and a monotonic frame counter (frames since `Playing`), it can
/// fire weapons, spawn hostiles, or poke input so the capture measures an
/// *active* scene (particle bursts, projectiles) rather than the scene at rest.
pub type PerfDriver = dyn Fn(&mut World, u32) + Send + Sync;

/// Read a perf parameter by logical name. Native: env var `NOVA_PERF_<UPPER>`
/// (e.g. `warmup` -> `NOVA_PERF_WARMUP`). Wasm: the URL query parameter `<name>`
/// (e.g. `?warmup=300`). One source abstraction so the same harness runs from a
/// shell env sweep and from a browser URL.
pub fn perf_param(name: &str) -> Option<String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::env::var(format!("NOVA_PERF_{}", name.to_ascii_uppercase()))
            .ok()
            .filter(|s| !s.is_empty())
    }
    #[cfg(target_arch = "wasm32")]
    {
        query_param(name)
    }
}

/// Whether frame-time capture is requested. Native: `NOVA_PERF` is set. Wasm:
/// the `?perf` query flag is present.
pub fn perf_armed() -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::env::var(PERF_ENV).is_ok()
    }
    #[cfg(target_arch = "wasm32")]
    {
        query_param("perf").is_some()
    }
}

/// Parse `window.location.search` for `name` (browser config channel).
#[cfg(target_arch = "wasm32")]
fn query_param(name: &str) -> Option<String> {
    let search = web_sys::window()?.location().search().ok()?;
    let query = search.strip_prefix('?').unwrap_or(&search);
    query.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        (key == name).then(|| value.replace('+', " "))
    })
}

/// Default warm-up frames discarded before the capture window opens.
pub const DEFAULT_WARMUP_FRAMES: u32 = 180;

/// Default number of frames captured into the stats window.
pub const DEFAULT_CAPTURE_FRAMES: u32 = 900;

/// Default forced primary-window resolution.
pub const DEFAULT_RESOLUTION: (f32, f32) = (1280.0, 720.0);

/// Env-gated frame-time capture preset for nova examples. See the module docs.
/// Inert unless `NOVA_PERF` (native) / `?perf` (wasm) is set. Chain
/// [`drive`](FrameTimePlugin::drive) to measure an *active* scene.
pub fn nova_frametime() -> FrameTimePlugin {
    FrameTimePlugin { driver: None }
}

/// Plugin returned by [`nova_frametime`]. Construct it through that preset.
pub struct FrameTimePlugin {
    driver: Option<Arc<PerfDriver>>,
}

impl FrameTimePlugin {
    /// Attach a per-frame [`PerfDriver`] run every frame the app is in
    /// `Playing` (warm-up included, so the scene is already active when capture
    /// opens). Use it to fire weapons / spawn hostiles so the capture measures a
    /// combat burst (particles, projectiles) rather than the scene at rest - see
    /// [`combat_burst_driver`].
    pub fn drive(mut self, driver: impl Fn(&mut World, u32) + Send + Sync + 'static) -> Self {
        self.driver = Some(Arc::new(driver));
        self
    }
}

/// Holds the active [`PerfDriver`] so the exclusive driving system can run it.
#[derive(Resource, Clone)]
struct PerfDriverRes(Arc<PerfDriver>);

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
    /// Read the config from the active source ([`perf_param`]: env on native,
    /// URL query on wasm), falling back to the documented defaults for anything
    /// unset or unparseable.
    fn resolve() -> Self {
        fn parse_u32(key: &str, default: u32) -> u32 {
            perf_param(key)
                .and_then(|v| v.parse().ok())
                .unwrap_or(default)
        }
        Self {
            warmup_frames: parse_u32("warmup", DEFAULT_WARMUP_FRAMES),
            capture_frames: parse_u32("frames", DEFAULT_CAPTURE_FRAMES),
            label: perf_param("label").unwrap_or_else(|| "scene".to_string()),
            out_dir: perf_param("out").map(PathBuf::from),
            resolution: perf_param("res")
                .and_then(|v| parse_resolution(&v))
                .unwrap_or(DEFAULT_RESOLUTION),
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
    /// Frames the driver has run (monotonic since `Playing`).
    driven: u32,
    /// Per-frame wall-clock deltas, milliseconds.
    samples: Vec<f64>,
}

impl Plugin for FrameTimePlugin {
    fn build(&self, app: &mut App) {
        if !perf_armed() {
            return;
        }
        let config = PerfConfig::resolve();
        info!(
            "nova perf: armed (label={}, warmup={}, frames={}, res={}x{}, out={:?}, driven={})",
            config.label,
            config.warmup_frames,
            config.capture_frames,
            config.resolution.0,
            config.resolution.1,
            config.out_dir,
            self.driver.is_some(),
        );
        app.insert_resource(PerfState {
            phase: Phase::WaitPlaying,
            warmed: 0,
            driven: 0,
            samples: Vec::with_capacity(config.capture_frames as usize),
        });
        app.insert_resource(config);
        // Continuous updates so an unfocused/headless window still runs flat out.
        app.insert_resource(WinitSettings::game());
        app.add_systems(Startup, perf_force_window);
        // The driver runs before the capture read so its work is inside the
        // measured frame.
        if let Some(driver) = &self.driver {
            app.insert_resource(PerfDriverRes(driver.clone()));
            app.add_systems(Update, perf_drive.before(perf_capture));
        }
        app.add_systems(Update, perf_capture);
    }
}

/// Run the attached [`PerfDriver`] every frame the app is in `Playing`
/// (warm-up + capture), passing a monotonic frame counter. Exclusive because a
/// driver needs `&mut World` to fire weapons / spawn entities.
fn perf_drive(world: &mut World) {
    let phase = world.resource::<PerfState>().phase;
    if !matches!(phase, Phase::Warmup | Phase::Capture) {
        return;
    }
    let frame = {
        let mut state = world.resource_mut::<PerfState>();
        let frame = state.driven;
        state.driven += 1;
        frame
    };
    let driver = world.resource::<PerfDriverRes>().0.clone();
    driver(world, frame);
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

/// A [`PerfDriver`] that drives a sustained combat burst, so a capture measures
/// the active-scene cost (turret muzzle flashes, projectiles in flight, torpedo
/// blasts - the particle load the graphics preset exists to cut) instead of the
/// scene at rest. Pass it to [`FrameTimePlugin::drive`] on a combat scenario
/// (e.g. `broadside`).
///
/// It does two things every frame:
///
/// 1. **Holds the player's fire.** Raises the combat stance (RMB held) and, once
///    the player's weapons read hot, holds the fire key - the exact proven
///    headless fire chain from the weapon-range examples (raise, wait for
///    [`WeaponsHot`], then hold, because the safety denies a press that lands
///    while cold). The player's turrets then fire continuously.
/// 2. **Keeps every combatant alive** (tops up [`Health`] to full). A kill would
///    end the burst early and can advance/reload the scenario mid-capture; the
///    top-up pins a steady-state burst for the whole window. Detonations still
///    fire (torpedoes blast on proximity, not only on kill), so the blast
///    particles are still measured. AI hostiles engage on their own and add
///    return fire and torpedo blasts on top.
pub fn combat_burst_driver(world: &mut World, _frame: u32) {
    // Sustain: no combatant dies, so the burst does not fizzle and no kill
    // advances the scenario out from under the capture.
    {
        let mut healths = world.query::<&mut Health>();
        for mut health in healths.iter_mut(world) {
            if health.current < health.max {
                health.current = health.max;
            }
        }
    }

    // Fire: hold the combat stance (RMB -> "Combat Mode" -> weapons hot), then,
    // once the player reads hot, hold the turret trigger (LMB -> "Turret"). The
    // safety denies a trigger press that lands while cold, so the wait matters;
    // fire is LMB (Space is the main-thruster "Flight Burn", not the gun).
    world
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Right);
    let player_hot = {
        let mut hot = world.query_filtered::<&WeaponsHot, With<PlayerSpaceshipMarker>>();
        hot.iter(world).next().is_some_and(|hot| hot.0)
    };
    if player_hot {
        world
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
    }
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
