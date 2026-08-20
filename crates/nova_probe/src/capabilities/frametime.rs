//! The frame-time capture harness: an env-gated plugin ([`nova_frametime`])
//! that drives a real gameplay app to `Playing`, warms up, records the
//! wall-clock delta of every frame for a fixed window, then writes percentile
//! frame-time stats (JSON + a CSV row, schema in [`crate::stats`]) and exits
//! cleanly through the harness completion protocol (the app exits when
//! every registered collector is done). Inert unless `NOVA_PERF` is set, so an
//! example adds it permanently and pays nothing in a normal run - the same
//! contract the `nova_autopilot` / `nova_screenshot` harness presets (in
//! `nova_debug`) follow.
//!
//! See the crate docs for the measurement rationale and the full knob table.

/// Glob-import surface for the frame-time capture capability.
pub mod prelude {
    pub use super::{
        capture_reload_begin, capture_reload_end, capture_reloading, combat_burst_driver,
        nova_frametime, perf_armed, perf_param, resolve_git_sha, resolve_host, FrameTimePlugin,
        PerfDriver, PerfReady, ReloadGate, ABORT_SIMULATION_STOPPED, CAPTURE_COLLECTOR,
        DEFAULT_CAPTURE_FRAMES, DEFAULT_RESOLUTION, DEFAULT_WARMUP_FRAMES, PERF_ENV,
    };
}

use std::{path::PathBuf, sync::Arc};

use bevy::{
    prelude::*,
    render::renderer::RenderAdapterInfo,
    time::TimeSystems,
    window::{PresentMode, PrimaryWindow},
    winit::WinitSettings,
};
use nova_autopilot::completion::{self, HarnessCompletion};
/// Environment variable that arms [`nova_frametime`] on native. Any value (even
/// empty) enables it; when unset the plugin adds nothing. On wasm the arm is the
/// `?perf` URL query flag instead (there are no process env vars in a browser).
///
/// Declared in `nova_core` because the window builder needs it too - it gives an
/// armed run a distinct `WM_CLASS` so a window manager can place captures away
/// from the desk - and `nova_core` is the lowest crate both already depend on.
pub use nova_core::PERF_ENV;
// `Health` comes from nova_gameplay's prelude, the same path the game's own
// code resolves it through: naming any other path is how this query silently
// stopped matching once nova took ownership of the type.
use nova_gameplay::{
    prelude::{GraphicsBudget, Health, HealthZeroMarker, PlayerSpaceshipMarker},
    GameStates,
};
use nova_ship::prelude::WeaponsHot;

use crate::stats::prelude::*;

/// Collector name the capture registers with the harness completion
/// protocol: the app exits when EVERY registered collector - this capture,
/// the autopilot - is done, so a wall-clock
/// timeline can no longer end the app mid-window (the 11-frames-short
/// scenario capture that silently lost 229 samples).
pub const CAPTURE_COLLECTOR: &str = "capture";

/// A per-frame combat/scene driver run under [`FrameTimePlugin::drive`]: given
/// `&mut World` and a monotonic frame counter (frames since `Playing`), it can
/// fire weapons, spawn hostiles, or poke input so the capture measures an
/// *active* scene (particle bursts, projectiles) rather than the scene at rest.
pub type PerfDriver = dyn Fn(&mut World, u32) + Send + Sync;

/// A readiness predicate run under [`FrameTimePlugin::ready_when`]: the
/// capture holds in `WaitPlaying` until it returns true, so an example whose
/// interesting load STARTS some way into the run measures that load instead of
/// whatever the scene was doing when it reached `Playing`.
///
/// `Playing` alone is the right gate for a scene that is fully loaded when it
/// arrives; it is the wrong one for a fight that has to be joined first, where
/// a fixed warm-up buys an arbitrary slice of the approach.
pub type PerfReady = dyn Fn(&World) -> bool + Send + Sync;

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

/// Token the abort line carries when the simulation stopped inside the window.
/// Shared with the host half, which scrapes the line out of the run log.
pub const ABORT_SIMULATION_STOPPED: &str = "simulation_stopped";

/// Default forced primary-window resolution.
pub const DEFAULT_RESOLUTION: (f32, f32) = (1280.0, 720.0);

/// Env-gated frame-time capture preset for nova examples. See the crate docs.
/// Inert unless `NOVA_PERF` (native) / `?perf` (wasm) is set. Chain
/// [`drive`](FrameTimePlugin::drive) to measure an *active* scene.
pub fn nova_frametime() -> FrameTimePlugin {
    FrameTimePlugin {
        driver: None,
        ready: None,
        window: None,
    }
}

/// Plugin returned by [`nova_frametime`]. Construct it through that preset.
#[derive(Clone)]
pub struct FrameTimePlugin {
    driver: Option<Arc<PerfDriver>>,
    ready: Option<Arc<PerfReady>>,
    window: Option<(u32, u32)>,
}

impl FrameTimePlugin {
    /// Hold the capture in its wait phase until `ready` holds, on top of
    /// reaching `Playing`. See [`PerfReady`]; the warm-up starts after the
    /// gate opens, so the window lands on the load the predicate names.
    pub fn ready_when(mut self, ready: impl Fn(&World) -> bool + Send + Sync + 'static) -> Self {
        self.ready = Some(Arc::new(ready));
        self
    }

    /// Declare this scene's own capture window, `(warmup, frames)`, in place of
    /// the [`DEFAULT_WARMUP_FRAMES`] / [`DEFAULT_CAPTURE_FRAMES`] baseline. An
    /// operator's `NOVA_PERF_WARMUP` / `NOVA_PERF_FRAMES` still wins.
    ///
    /// For a scene that can REACH AN END - a fight that can be won, a chapter
    /// that can be completed - the baseline window is not a free choice: the
    /// capture must close while the scene is still running, because everything
    /// past the end is a paused result screen and the capture refuses it
    /// ([`ABORT_SIMULATION_STOPPED`]). Size it from a measured run of the
    /// scene, not from a guess, and say in the example where the number came
    /// from. A shorter window costs percentile resolution, which is the price
    /// of measuring one scene instead of two.
    pub fn window(mut self, warmup: u32, frames: u32) -> Self {
        self.window = Some((warmup, frames));
        self
    }

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

/// Holds the active [`PerfReady`] gate and the latch it sets. Absent when the
/// example named no gate, which is `Playing` alone.
///
/// The latch is an atomic rather than a `ResMut` because the predicate needs
/// `&World` and nothing else may: [`perf_watch_ready`] is a READ-ONLY system,
/// so it cannot write a resource, and making it exclusive instead would put a
/// command-flush barrier in the middle of `Update` for every armed capture -
/// which reorders the whole schedule's deferred work and is not something a
/// measurement may do to the thing it measures.
#[derive(Resource, Clone)]
struct PerfReadyRes {
    ready: Arc<PerfReady>,
    open: Arc<std::sync::atomic::AtomicBool>,
}

/// Reload bookkeeping for LOOPED captures: frames inside a scene reload are
/// EXCLUDED from the scene stats - how many
/// reloads land in a window is host-speed-dependent, so including them
/// makes baseline deltas measure reload count instead of scene cost - and
/// tallied here for the report's own reload line.
#[derive(Resource, Default)]
pub struct ReloadGate {
    reloading: bool,
    /// Skip one more frame after the gate closes: that frame's delta spans
    /// the reload boundary.
    skip_next: bool,
    started_secs: f64,
    reload_ms: Vec<f64>,
}

/// Whether a looped-capture scene reload is currently in flight. Enrolled
/// example scripts gate on this: between the loop trigger and the
/// scenario-loaded signal the scene's variables/entities do not exist, and
/// a script frame that reads them would panic on torn-down state.
pub fn capture_reloading(world: &World) -> bool {
    world
        .get_resource::<ReloadGate>()
        .is_some_and(|gate| gate.reloading)
}

/// Mark a scene reload IN FLIGHT (an enrolled example's `AutopilotLoop`
/// observer calls this before re-triggering its scenario load). No-op when
/// the capture is not armed.
pub fn capture_reload_begin(world: &mut World) {
    let now = world.resource::<Time<Real>>().elapsed_secs_f64();
    if let Some(mut gate) = world.get_resource_mut::<ReloadGate>() {
        if !gate.reloading {
            gate.reloading = true;
            gate.started_secs = now;
        }
    }
}

/// Mark the reload COMPLETE (the example's scenario-loaded observer). The
/// very first scene load calls this too - a no-op unless a reload was in
/// flight.
pub fn capture_reload_end(world: &mut World) {
    let now = world.resource::<Time<Real>>().elapsed_secs_f64();
    if let Some(mut gate) = world.get_resource_mut::<ReloadGate>() {
        if gate.reloading {
            gate.reloading = false;
            gate.skip_next = true;
            let ms = (now - gate.started_secs) * 1000.0;
            gate.reload_ms.push(ms);
            debug!("nova perf: reload interval closed ({ms:.1} ms, excluded from stats)");
        }
    }
}

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
    /// Unset leaves the tier's own default.
    render_scale_override: Option<f32>,
    /// Optional forced `Time<Virtual>::max_delta`, in seconds
    /// (`NOVA_PERF_MAX_DELTA` / `max_delta=`). The isolation knob for the
    /// fixed-step loop: bevy's 0.25 s default lets one slow frame queue up to
    /// `0.25 / timestep` fixed steps, and whether that AMPLIFIES a stutter or
    /// merely tracks it is answerable by capping the ceiling and re-measuring.
    /// It buys a bounded tail with simulation time debt - the discarded delta
    /// is time the world never simulates - so it is a measurement knob here,
    /// never a default.
    max_delta_override: Option<f32>,
    /// Presentation mode forced on the primary window
    /// (`NOVA_PERF_PRESENT` / `present=`), defaulting to
    /// [`PresentMode::AutoNoVsync`].
    ///
    /// The default is a REQUEST and wgpu answers it silently: it falls back
    /// Immediate -> Mailbox -> Fifo on whatever the surface offers, and bevy
    /// logs the fallback only for an EXPLICITLY named mode. A capture that
    /// landed on `Fifo` measures the display's refresh period, not the game, so
    /// naming `immediate` here is how a run proves it did not.
    present_mode: PresentMode,
}

/// Parse the `NOVA_PERF_PRESENT` / `present=` override.
fn parse_present_mode(value: &str) -> Option<PresentMode> {
    match value.trim().to_ascii_lowercase().as_str() {
        "immediate" => Some(PresentMode::Immediate),
        "mailbox" => Some(PresentMode::Mailbox),
        "fifo" => Some(PresentMode::Fifo),
        "fiforelaxed" | "fifo_relaxed" => Some(PresentMode::FifoRelaxed),
        "autovsync" => Some(PresentMode::AutoVsync),
        "autonovsync" => Some(PresentMode::AutoNoVsync),
        _ => None,
    }
}

impl PerfConfig {
    /// Read the config from the active source ([`perf_param`]: env on native,
    /// URL query on wasm), falling back to the example's declared window
    /// ([`FrameTimePlugin::window`]) and then to the documented defaults for
    /// anything unset or unparseable.
    fn resolve(declared_window: Option<(u32, u32)>) -> Self {
        fn parse_u32(key: &str, default: u32) -> u32 {
            perf_param(key)
                .and_then(|v| v.parse().ok())
                .unwrap_or(default)
        }
        let (warmup, frames) =
            declared_window.unwrap_or((DEFAULT_WARMUP_FRAMES, DEFAULT_CAPTURE_FRAMES));
        Self {
            warmup_frames: parse_u32("warmup", warmup),
            capture_frames: parse_u32("frames", frames),
            label: perf_param("label").unwrap_or_else(|| "scene".to_string()),
            out_dir: perf_param("out").map(PathBuf::from),
            resolution: perf_param("res")
                .and_then(|v| parse_resolution(&v))
                .unwrap_or(DEFAULT_RESOLUTION),
            render_scale_override: perf_param("render_scale").and_then(|v| v.trim().parse().ok()),
            max_delta_override: perf_param("max_delta")
                .and_then(|v| v.trim().parse::<f32>().ok())
                .filter(|secs| *secs > 0.0),
            present_mode: perf_param("present")
                .and_then(|v| parse_present_mode(&v))
                .unwrap_or(PresentMode::AutoNoVsync),
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
            // The CAPTURE binary's own build profile (schema v3): dev
            // numbers are not baselines, and every row must say which it
            // is. cfg! resolves at the capture's compile time - exactly the
            // binary that produced the frames.
            profile: if cfg!(debug_assertions) {
                "dev".to_string()
            } else {
                "release".to_string()
            },
        }
    }
}

/// The measured tree's short git SHA: the `NOVA_PERF_SHA` / `?sha=` override
/// wins (the web build cannot shell out); otherwise ask git, degrading to
/// `unknown` outside a repo or without git on PATH.
pub fn resolve_git_sha() -> String {
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
pub fn resolve_host() -> String {
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
    /// The window was refused: it contained frames the simulation did not run
    /// through. No stats were written and none will be.
    Aborted,
}

impl Phase {
    /// The word the abort line names the phase with.
    fn token(self) -> &'static str {
        match self {
            Phase::WaitPlaying => "wait",
            Phase::Warmup => "warmup",
            Phase::Capture => "capture",
            Phase::Done => "done",
            Phase::Aborted => "aborted",
        }
    }
}

/// Whether the SIMULATION advanced this frame, given the app's virtual clock.
///
/// A capture measures wall-clock deltas, so a scene whose clock is stopped -
/// a result screen, a pause menu, NOVA OS - still produces frames, and they
/// still cost real milliseconds to draw. Averaged in they read as an ordinary
/// (even a plausible) window while measuring a still picture, which is the one
/// failure this whole capture cannot survive silently.
///
/// An app with no `Time<Virtual>` at all cannot pause one; nothing to refuse.
fn simulation_running(virtual_time: Option<&Time<Virtual>>) -> bool {
    virtual_time.is_none_or(|time| !time.is_paused() && time.relative_speed() > 0.0)
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
    /// Fixed steps that ran inside each sampled frame, index-parallel to
    /// [`Self::samples`].
    fixed_steps: Vec<u32>,
}

/// How many fixed steps `RunFixedMainLoop` ran this frame, counted by
/// [`perf_tally_fixed_step`] and drained once per frame by [`perf_capture`].
///
/// `Time<Virtual>::max_delta` (0.25 s by default) against the fixed timestep
/// bounds this, so a slow frame can carry a burst of steps into the next one.
/// Whether that happens is a measurement, not an assumption, which is why the
/// count is recorded beside the frame time it belongs to.
#[derive(Resource, Default)]
struct FixedStepTally(u32);

/// Count one fixed step. Runs in `FixedFirst`, which is inside
/// `RunFixedMainLoop` - and that schedule runs BEFORE `Update`, so the count
/// [`perf_capture`] drains belongs to the frame it is recorded against.
fn perf_tally_fixed_step(mut tally: ResMut<FixedStepTally>) {
    tally.0 += 1;
}

impl Plugin for FrameTimePlugin {
    fn build(&self, app: &mut App) {
        // Declared by WIRING, above the arming guard: adding this plugin IS
        // the frame-time claim, whether or not this run armed the capture.
        crate::contract::declare(app, crate::contract::Capability::FrameTime);
        if !perf_armed() {
            return;
        }
        completion::register(app, CAPTURE_COLLECTOR);
        app.init_resource::<ReloadGate>();
        let config = PerfConfig::resolve(self.window);
        info!(
            "nova perf: armed (label={}, warmup={}, frames={}, res={}x{}, render_scale={:?}, max_delta={:?}, out={:?}, driven={}, gated={})",
            config.label,
            config.warmup_frames,
            config.capture_frames,
            config.resolution.0,
            config.resolution.1,
            config.render_scale_override,
            config.max_delta_override,
            config.out_dir,
            self.driver.is_some(),
            self.ready.is_some(),
        );
        app.insert_resource(PerfState {
            phase: Phase::WaitPlaying,
            warmed: 0,
            driven: 0,
            samples: Vec::with_capacity(config.capture_frames as usize),
            fixed_steps: Vec::with_capacity(config.capture_frames as usize),
        });
        app.init_resource::<FixedStepTally>();
        app.add_systems(FixedFirst, perf_tally_fixed_step);
        let force_render_scale = config.render_scale_override.is_some();
        let force_max_delta = config.max_delta_override.is_some();
        app.insert_resource(config);
        // Continuous updates so an unfocused/headless window still runs flat out.
        app.insert_resource(WinitSettings::game());
        app.add_systems(Startup, perf_force_window);
        // Isolation knob: pin render_scale to the override every frame (it wins
        // over the tier's apply, which only runs on a quality change).
        if force_render_scale {
            app.add_systems(Update, perf_force_render_scale);
        }
        // Held every frame, not set once: a scenario load or a pause/unpause
        // hands `Time<Virtual>` back at its default, and a ceiling that
        // silently lifts mid-window measures neither setting.
        if force_max_delta {
            app.add_systems(First, perf_force_max_delta.before(TimeSystems));
        }
        // The driver runs before the capture read so its work is inside the
        // measured frame.
        if let Some(driver) = &self.driver {
            app.insert_resource(PerfDriverRes(driver.clone()));
            app.add_systems(Update, perf_drive.before(perf_capture));
        }
        // The gate watcher exists ONLY for an example that named one: an
        // ungated capture must schedule exactly what it scheduled before.
        if let Some(ready) = &self.ready {
            app.insert_resource(PerfReadyRes {
                ready: ready.clone(),
                open: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            });
            app.add_systems(Update, perf_watch_ready.before(perf_capture));
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

/// Latch the example's [`PerfReady`] gate once it holds. Read-only (`&World`
/// is the whole parameter list, which is what a predicate over arbitrary world
/// state needs) - see [`PerfReadyRes`] for why it may not be exclusive.
/// Added only when a gate was named.
fn perf_watch_ready(world: &World) {
    let Some(gate) = world.get_resource::<PerfReadyRes>() else {
        return;
    };
    if gate.open.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    if (gate.ready)(world) {
        gate.open.store(true, std::sync::atomic::Ordering::Relaxed);
        info!("nova perf: readiness gate open, warm-up starts");
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
    window.present_mode = config.present_mode;
    window.resizable = false;
}

/// Pin [`GraphicsBudget::render_scale`] to the configured override, holding the
/// rest of the preset fixed - the isolation knob for measuring the render-scale
/// lever on its own. Only added when the override is set;
/// the `!=` guard avoids marking the budget changed every frame.
fn perf_force_render_scale(config: Res<PerfConfig>, budget: Option<ResMut<GraphicsBudget>>) {
    let (Some(scale), Some(mut budget)) = (config.render_scale_override, budget) else {
        return;
    };
    if budget.render_scale != scale {
        budget.render_scale = scale;
    }
}

/// Pin `Time<Virtual>`'s `max_delta` to the configured override, before the
/// clock advances this frame. Only added when the override is set; the `!=`
/// guard keeps the resource from reading as changed every frame.
fn perf_force_max_delta(config: Res<PerfConfig>, mut virtual_time: ResMut<Time<Virtual>>) {
    let Some(secs) = config.max_delta_override else {
        return;
    };
    let max_delta = std::time::Duration::from_secs_f32(secs);
    if virtual_time.max_delta() != max_delta {
        virtual_time.set_max_delta(max_delta);
    }
}

/// Advance the capture state machine one frame: wait for the scene to be
/// ready, discard warm-up frames, record deltas, then compute + emit stats and
/// exit. The adapter resource feeds the run metadata (schema v2) at emit time.
#[expect(
    clippy::too_many_arguments,
    reason = "one system owns the whole capture state machine; splitting it would \
              split the frame it measures"
)]
fn perf_capture(
    time: Res<Time<Real>>,
    // The SIMULATION clock, read only to find out whether it moved. Optional
    // for the same reason `GameStates` is: a bare `App` rig may have neither.
    virtual_time: Option<Res<Time<Virtual>>>,
    // OPTIONAL, and the reason is a real crash: a `systems/` rig that wires
    // `NovaProbePlugin` on a bare `App` has no `GameStates` at all, and a
    // required `Res` there fails parameter validation and takes the whole run
    // down the moment NOVA_PERF is set. A capture with no state machine to
    // wait on cannot measure anything, so it stands down and releases its
    // collector instead of holding the app to the deadline.
    state_res: Option<Res<State<GameStates>>>,
    ready: Option<Res<PerfReadyRes>>,
    config: Res<PerfConfig>,
    adapter: Option<Res<RenderAdapterInfo>>,
    mut gate: ResMut<ReloadGate>,
    mut state: ResMut<PerfState>,
    mut tally: ResMut<FixedStepTally>,
    mut completion: ResMut<HarnessCompletion>,
) {
    // Drained unconditionally, before any early return: a count left standing
    // would be attributed to a later frame.
    let fixed_steps = std::mem::take(&mut tally.0);
    match state.phase {
        Phase::WaitPlaying => {
            let Some(state_res) = state_res else {
                warn!(
                    "nova perf: no GameStates in this app - nothing to wait on, \
                     capture stands down"
                );
                state.phase = Phase::Done;
                completion.done(CAPTURE_COLLECTOR);
                return;
            };
            // `Playing` plus the example's own gate, when it named one - see
            // [`PerfReady`]. The gate is latched by `perf_watch_ready`.
            let gated =
                ready.is_some_and(|gate| !gate.open.load(std::sync::atomic::Ordering::Relaxed));
            if *state_res.get() == GameStates::Playing && !gated {
                state.phase = Phase::Warmup;
            }
        }
        Phase::Warmup => {
            if !simulation_running(virtual_time.as_deref()) {
                abort_stopped(&config, &mut state, &mut completion, Phase::Warmup);
                return;
            }
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
            // Reload frames are not scene frames: skip the sample AND the
            // frame right after the gate closes (its delta spans the
            // reload boundary).
            if gate.reloading {
                return;
            }
            if gate.skip_next {
                gate.skip_next = false;
                return;
            }
            // BEFORE the sample, never after: a frame the simulation did not
            // run through is not a frame of this scene, and one of them in the
            // window is enough to make every statistic over it a fiction.
            if !simulation_running(virtual_time.as_deref()) {
                abort_stopped(&config, &mut state, &mut completion, Phase::Capture);
                return;
            }
            state.samples.push(time.delta_secs_f64() * 1000.0);
            state.fixed_steps.push(fixed_steps);
            if state.samples.len() as u32 >= config.capture_frames {
                let stats = FrameStats::from_samples(&state.samples);
                let steps = FixedStepStats::from_frames(&state.samples, &state.fixed_steps);
                let meta = RunMeta::resolve(&config, adapter.as_deref());
                emit_stats(&config, &stats, steps.as_ref(), &meta, &gate.reload_ms);
                state.phase = Phase::Done;
                // Negotiated, not unilateral: the watcher exits when every
                // registered collector (this capture, the autopilot) is done.
                completion.done(CAPTURE_COLLECTOR);
            }
        }
        Phase::Done | Phase::Aborted => {}
    }
}

/// Refuse the window and say so at ERROR, loudly enough that no reader has to
/// infer it from a plausible mean.
///
/// It writes NO stats. A contaminated capture that emitted a row would be
/// averaged into a repeat set by a gate built for outliers, and a stopped
/// simulation is not an outlier: it draws the same scene at a steady cost, so
/// the mean it produces looks exactly like an honest one. Discarding the
/// capture is the only reading that is true.
fn abort_stopped(
    config: &PerfConfig,
    state: &mut PerfState,
    completion: &mut HarnessCompletion,
    phase: Phase,
) {
    let frame = match phase {
        Phase::Capture => state.samples.len(),
        _ => state.warmed as usize,
    };
    state.phase = Phase::Aborted;
    error!(
        "nova perf: label={} ABORTED reason={ABORT_SIMULATION_STOPPED} phase={} frame={frame} \
         warmup={} frames={} - Time<Virtual> was stopped (paused, or running at speed 0) \
         inside the capture window. The scene reached an end - a result screen, a pause, an \
         outcome overlay - and the frames after it draw a still picture at a plausible cost. \
         No stats were written: bound the window so it closes while the scene is still \
         running, or measure a scene that cannot end inside it.",
        config.label,
        phase.token(),
        config.warmup_frames,
        config.capture_frames,
    );
    completion.done(CAPTURE_COLLECTOR);
}

/// Log the summary line and, when `NOVA_PERF_OUT` is set, write a per-run JSON
/// file and append a row to the aggregated CSV (schema v3, run metadata
/// included). The log line is always emitted - on wasm there is no filesystem,
/// so a headless-browser driver scrapes it from the console.
fn emit_stats(
    config: &PerfConfig,
    stats: &FrameStats,
    steps: Option<&FixedStepStats>,
    meta: &RunMeta,
    reload_ms: &[f64],
) {
    info!("{}", stats.summary_line(&config.label));
    if let Some(steps) = steps {
        info!("{}", steps.summary_line(&config.label));
    }
    info!(
        "nova perf: meta backend={} adapter={:?} res={} quality={} sha={} host={} profile={}",
        meta.backend,
        meta.adapter,
        meta.resolution,
        meta.quality,
        meta.git_sha,
        meta.host,
        meta.profile
    );
    if !reload_ms.is_empty() {
        let mean = reload_ms.iter().sum::<f64>() / reload_ms.len() as f64;
        let max = reload_ms.iter().cloned().fold(0.0_f64, f64::max);
        info!(
            "nova perf: {} scene reload(s) excluded from the stats (mean {mean:.1} ms, max {max:.1} ms)",
            reload_ms.len()
        );
    }

    let Some(dir) = &config.out_dir else {
        return;
    };
    if let Err(error) = std::fs::create_dir_all(dir) {
        warn!("nova perf: could not create out dir {:?}: {error}", dir);
        return;
    }

    let json_path = dir.join(format!("{}.json", sanitize(&config.label)));
    if let Err(error) = std::fs::write(
        &json_path,
        stats.to_json(&config.label, meta, steps, reload_ms),
    ) {
        warn!("nova perf: could not write {:?}: {error}", json_path);
    } else {
        info!("nova perf: wrote {:?}", json_path);
    }

    // Through the public writer, not a second copy of the append: it is the
    // one that refuses to mix schemas, which a manual NOVA_PERF_OUT into an
    // older results dir is exactly the case for.
    let csv_path = dir.join("frametime.csv");
    if let Err(error) = crate::stats::append_frametime_row(&csv_path, &config.label, stats, meta) {
        warn!("nova perf: {error}");
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
/// 2. **Tops surviving combatants back up** to full [`Health`] between hits, so
///    a burst of ordinary fire does not whittle the scene down and fizzle the
///    window. This is a top-up, NOT immortality: the pass runs once a frame,
///    after damage, so a single overkill hit (a torpedo blast on a section pool)
///    still kills, and a kill can advance/reload the scenario mid-capture.
///    Entities already spent carry [`HealthZeroMarker`] and are skipped - the
///    destruction pipeline observes the marker's insertion, so refilling their
///    pool would not revive them, only forge a full-HP-yet-destroyed entity that
///    reads as clean to the health-bounds invariant. Detonations still fire
///    (torpedoes blast on proximity, not only on kill), so the blast particles
///    are still measured. AI hostiles engage on their own and add return fire
///    and torpedo blasts on top.
pub fn combat_burst_driver(world: &mut World, _frame: u32) {
    // Sustain: ordinary fire does not whittle the scene down, so the burst does
    // not fizzle. Spent pools are left alone - see the doc above.
    {
        let mut healths = world.query_filtered::<&mut Health, Without<HealthZeroMarker>>();
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
    fn an_unnamed_or_misspelled_present_mode_leaves_the_default_request_in_place() {
        assert_eq!(
            parse_present_mode("immediate"),
            Some(PresentMode::Immediate)
        );
        assert_eq!(parse_present_mode("  FIFO "), Some(PresentMode::Fifo));
        assert_eq!(
            parse_present_mode("fifo_relaxed"),
            Some(PresentMode::FifoRelaxed)
        );
        assert_eq!(parse_present_mode("novsync"), None);
    }

    #[test]
    fn sanitize_replaces_path_hostile_chars() {
        assert_eq!(sanitize("asteroid_field-gpu"), "asteroid_field-gpu");
        assert_eq!(sanitize("a/b c:d"), "a_b_c_d");
    }

    /// The three ways a scene stops simulating while its frames keep costing
    /// real milliseconds, and the one app that cannot do it at all.
    #[test]
    fn a_paused_or_stopped_virtual_clock_is_not_a_running_simulation() {
        let mut time = Time::<Virtual>::default();
        assert!(simulation_running(Some(&time)), "a live clock runs");

        time.pause();
        assert!(!simulation_running(Some(&time)), "a result screen");
        time.unpause();

        time.set_relative_speed(0.0);
        assert!(!simulation_running(Some(&time)), "a clock held at zero");

        // No virtual clock at all - a bare `App` rig - cannot pause one.
        assert!(simulation_running(None));
    }

    /// The window is the example's to declare, and the operator's to override.
    #[test]
    fn a_declared_window_beats_the_default_and_loses_to_the_operator() {
        // No `NOVA_PERF_*` in this process (the suite runs without them), so
        // the declaration is what resolves.
        if std::env::var_os("NOVA_PERF_WARMUP").is_none()
            && std::env::var_os("NOVA_PERF_FRAMES").is_none()
        {
            let default = PerfConfig::resolve(None);
            assert_eq!(default.warmup_frames, DEFAULT_WARMUP_FRAMES);
            assert_eq!(default.capture_frames, DEFAULT_CAPTURE_FRAMES);

            let declared = PerfConfig::resolve(Some((40, 320)));
            assert_eq!(declared.warmup_frames, 40);
            assert_eq!(declared.capture_frames, 320);
        }
    }

    /// The state machine, driven: a window that meets a stopped simulation is
    /// REFUSED where it stands, and the samples it already took go nowhere.
    ///
    /// Wired by hand rather than through the plugin because arming reads the
    /// process environment, and a test that sets env decides what every other
    /// test in the binary measures.
    fn armed_app(warmup: u32, frames: u32) -> App {
        let mut app = App::new();
        app.init_resource::<Time<Real>>();
        app.init_resource::<Time<Virtual>>();
        app.init_resource::<ReloadGate>();
        app.init_resource::<FixedStepTally>();
        app.init_resource::<HarnessCompletion>();
        app.insert_resource(State::new(GameStates::Playing));
        app.insert_resource(PerfConfig {
            warmup_frames: warmup,
            capture_frames: frames,
            label: "test".into(),
            out_dir: None,
            resolution: DEFAULT_RESOLUTION,
            render_scale_override: None,
            max_delta_override: None,
            present_mode: PresentMode::AutoNoVsync,
        });
        app.insert_resource(PerfState {
            phase: Phase::WaitPlaying,
            warmed: 0,
            driven: 0,
            samples: Vec::new(),
            fixed_steps: Vec::new(),
        });
        app.add_systems(Update, perf_capture);
        app
    }

    #[test]
    fn a_capture_that_meets_a_stopped_simulation_is_refused_where_it_stands() {
        let mut app = armed_app(2, 100);
        // WaitPlaying, then two warm-up frames, then two captured frames.
        for _ in 0..5 {
            app.update();
        }
        assert_eq!(app.world().resource::<PerfState>().phase, Phase::Capture);
        assert_eq!(app.world().resource::<PerfState>().samples.len(), 2);

        app.world_mut().resource_mut::<Time<Virtual>>().pause();
        app.update();

        let state = app.world().resource::<PerfState>();
        assert_eq!(state.phase, Phase::Aborted);
        assert_eq!(
            state.samples.len(),
            2,
            "the stopped frame must not join the window"
        );

        // And it stays refused: the scene unpausing later does not make the
        // window whole again.
        app.world_mut().resource_mut::<Time<Virtual>>().unpause();
        app.update();
        assert_eq!(app.world().resource::<PerfState>().phase, Phase::Aborted);
        assert_eq!(app.world().resource::<PerfState>().samples.len(), 2);
    }

    /// A warm-up spent on a stopped clock is refused too - the window would
    /// open on a scene that had already ended.
    #[test]
    fn a_stopped_warm_up_is_refused_before_the_window_opens() {
        let mut app = armed_app(10, 100);
        app.update();
        assert_eq!(app.world().resource::<PerfState>().phase, Phase::Warmup);
        app.world_mut().resource_mut::<Time<Virtual>>().pause();
        app.update();
        assert_eq!(app.world().resource::<PerfState>().phase, Phase::Aborted);
    }

    /// The ordinary path still closes: an unstopped window fills and emits.
    #[test]
    fn an_unstopped_window_still_completes() {
        let mut app = armed_app(1, 3);
        for _ in 0..6 {
            app.update();
        }
        let state = app.world().resource::<PerfState>();
        assert_eq!(state.phase, Phase::Done);
        assert_eq!(state.samples.len(), 3);
    }
}
