//! The frame-time capture harness: an env-gated plugin ([`nova_frametime`])
//! that drives a real gameplay app to `Playing`, warms up, records the
//! wall-clock delta of every frame for a fixed window, then writes percentile
//! frame-time stats (JSON + a CSV row, schema in [`crate::stats`]) and exits
//! cleanly with [`AppExit::Success`]. Inert unless `NOVA_PERF` is set, so an
//! example adds it permanently and pays nothing in a normal run - the same
//! contract the `nova_autopilot` / `nova_screenshot` harness presets (in
//! `nova_debug`) follow.
//!
//! See the crate docs for the measurement rationale and the full knob table.

use std::{path::PathBuf, sync::Arc};

use bevy::{
    prelude::*,
    render::renderer::RenderAdapterInfo,
    window::{PresentMode, PrimaryWindow},
    winit::WinitSettings,
};
// Health is re-exported by nova_gameplay, so nova_probe pins the same
// bevy_common_systems version the game uses (no direct bcs dep, no version skew).
use nova_gameplay::{
    bevy_common_systems::health::Health,
    prelude::{GraphicsBudget, PlayerSpaceshipMarker, WeaponsHot},
    GameStates,
};

use crate::stats::{FrameStats, RunMeta, CSV_HEADER};

/// Environment variable that arms [`nova_frametime`] on native. Any value (even
/// empty) enables it; when unset the plugin adds nothing. On wasm the arm is the
/// `?perf` URL query flag instead (there are no process env vars in a browser).
/// The `NOVA_PERF_*` prefix predates the crate's rename to `nova_probe`; it is
/// the stable measurement surface the runner-CLI task redesigns, kept as-is so
/// scripts and docs do not churn twice.
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

/// Env-gated frame-time capture preset for nova examples. See the crate docs.
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
    /// Optional forced `GraphicsBudget::render_scale`, holding the rest of the
    /// preset fixed. Set (`NOVA_PERF_RENDER_SCALE` / `render_scale=`) to isolate
    /// the render-scale lever from the tier's particle/scatter cuts - measure
    /// the SAME tier at `1.0` vs a fraction so the delta is pure resolution
    /// (task 20260718-004723). Unset leaves the tier's own default.
    render_scale_override: Option<f32>,
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
            render_scale_override: perf_param("render_scale").and_then(|v| v.trim().parse().ok()),
        }
    }
}

impl RunMeta {
    /// Resolve the run metadata at emit time. The adapter comes from bevy's
    /// [`RenderAdapterInfo`], which `RenderPlugin` clones into the MAIN world
    /// (bevy_render-0.19.0/src/settings.rs:197 `main_world.insert_resource`),
    /// so a plain main-world system can read it; `None` (e.g. a `--norender`
    /// build) degrades to `unknown`. The rest comes from [`perf_param`]
    /// overrides with platform fallbacks - see each helper.
    fn resolve(config: &PerfConfig, adapter: Option<&RenderAdapterInfo>) -> Self {
        let (backend, adapter_name) = match adapter {
            Some(info) => (info.backend.to_str().to_string(), info.name.clone()),
            None => ("unknown".to_string(), "unknown".to_string()),
        };
        Self {
            backend,
            adapter: adapter_name,
            resolution: format!("{}x{}", config.resolution.0, config.resolution.1),
            quality: perf_param("quality").unwrap_or_else(|| "default".to_string()),
            git_sha: resolve_git_sha(),
            host: resolve_host(),
        }
    }
}

/// The measured tree's short git SHA: the `NOVA_PERF_SHA` / `?sha=` override
/// wins (the web build cannot shell out); otherwise ask git, degrading to
/// `unknown` outside a repo or without git on PATH.
pub(crate) fn resolve_git_sha() -> String {
    if let Some(sha) = perf_param("sha") {
        return sha;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Ok(output) = std::process::Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .output()
        {
            if output.status.success() {
                if let Ok(sha) = String::from_utf8(output.stdout) {
                    let sha = sha.trim();
                    if !sha.is_empty() {
                        return sha.to_string();
                    }
                }
            }
        }
    }
    "unknown".to_string()
}

/// The host tag: the `NOVA_PERF_HOST` / `?host=` override wins; native falls
/// back to `/etc/hostname`, wasm to the literal `browser`.
pub(crate) fn resolve_host() -> String {
    if let Some(host) = perf_param("host") {
        return host;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Ok(name) = std::fs::read_to_string("/etc/hostname") {
            let name = name.trim();
            if !name.is_empty() {
                return name.to_string();
            }
        }
        "unknown".to_string()
    }
    #[cfg(target_arch = "wasm32")]
    {
        "browser".to_string()
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
            "nova perf: armed (label={}, warmup={}, frames={}, res={}x{}, render_scale={:?}, out={:?}, driven={})",
            config.label,
            config.warmup_frames,
            config.capture_frames,
            config.resolution.0,
            config.resolution.1,
            config.render_scale_override,
            config.out_dir,
            self.driver.is_some(),
        );
        app.insert_resource(PerfState {
            phase: Phase::WaitPlaying,
            warmed: 0,
            driven: 0,
            samples: Vec::with_capacity(config.capture_frames as usize),
        });
        let force_render_scale = config.render_scale_override.is_some();
        app.insert_resource(config);
        // Continuous updates so an unfocused/headless window still runs flat out.
        app.insert_resource(WinitSettings::game());
        app.add_systems(Startup, perf_force_window);
        // Isolation knob: pin render_scale to the override every frame (it wins
        // over the tier's apply, which only runs on a quality change).
        if force_render_scale {
            app.add_systems(Update, perf_force_render_scale);
        }
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

/// Pin [`GraphicsBudget::render_scale`] to the configured override, holding the
/// rest of the preset fixed - the isolation knob for measuring the render-scale
/// lever on its own (task 20260718-004723). Only added when the override is set;
/// the `!=` guard avoids marking the budget changed every frame.
fn perf_force_render_scale(config: Res<PerfConfig>, budget: Option<ResMut<GraphicsBudget>>) {
    let (Some(scale), Some(mut budget)) = (config.render_scale_override, budget) else {
        return;
    };
    if budget.render_scale != scale {
        budget.render_scale = scale;
    }
}

/// Advance the capture state machine one frame: wait for `Playing`, discard
/// warm-up frames, record deltas, then compute + emit stats and exit. The
/// adapter resource feeds the run metadata (schema v2) at emit time.
fn perf_capture(
    time: Res<Time<Real>>,
    state_res: Res<State<GameStates>>,
    config: Res<PerfConfig>,
    adapter: Option<Res<RenderAdapterInfo>>,
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
                let meta = RunMeta::resolve(&config, adapter.as_deref());
                emit_stats(&config, &stats, &meta);
                state.phase = Phase::Done;
                exit.write(AppExit::Success);
            }
        }
        Phase::Done => {}
    }
}

/// Log the summary line and, when `NOVA_PERF_OUT` is set, write a per-run JSON
/// file and append a row to the aggregated CSV (schema v2, run metadata
/// included). The log line is always emitted - on wasm there is no filesystem,
/// so a headless-browser driver scrapes it from the console.
fn emit_stats(config: &PerfConfig, stats: &FrameStats, meta: &RunMeta) {
    info!("{}", stats.summary_line(&config.label));
    info!(
        "nova perf: meta backend={} adapter={:?} res={} quality={} sha={} host={}",
        meta.backend, meta.adapter, meta.resolution, meta.quality, meta.git_sha, meta.host
    );

    let Some(dir) = &config.out_dir else {
        return;
    };
    if let Err(error) = std::fs::create_dir_all(dir) {
        warn!("nova perf: could not create out dir {:?}: {error}", dir);
        return;
    }

    let json_path = dir.join(format!("{}.json", sanitize(&config.label)));
    if let Err(error) = std::fs::write(&json_path, stats.to_json(&config.label, meta)) {
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
            buf.push_str(&stats.to_csv_row(&config.label, meta));
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
    fn sanitize_replaces_path_hostile_chars() {
        assert_eq!(sanitize("asteroid_field-gpu"), "asteroid_field-gpu");
        assert_eq!(sanitize("a/b c:d"), "a_b_c_d");
    }
}
