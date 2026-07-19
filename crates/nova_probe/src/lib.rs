//! nova_probe: the run-harness for Nova Protocol dev tooling - frame-time
//! capture and perf reporting over autopilot runs. (Formerly `nova_perf`;
//! grown per the spike in `tasks/20260719-112011/SPIKE.md` into the home for
//! the run-correctness recorder, invariant checks, profiling and the unified
//! run report as those tasks land.)
//!
//! Three modules today:
//!
//! - [`capture`] - the env-gated [`nova_frametime`] plugin: drives a real
//!   gameplay app to `Playing`, warms up, records the wall-clock delta of
//!   every frame for a fixed window, then writes percentile stats and exits.
//! - [`stats`] - [`FrameStats`], the per-run [`RunMeta`], and the CSV/JSON
//!   schema (writers + parsers) shared by the capture and the report.
//! - [`report`] - renders parsed runs into one self-contained HTML report
//!   (the `perf_report` bin is a thin CLI over it).
//!
//! ## Why measure this way
//!
//! - **Real frame delta, not the diagnostics store.** The capture reads
//!   [`bevy::prelude::Time`]`<Real>` deltas directly: wall-clock time between
//!   frames, unaffected by the fixed-timestep clamp or a paused virtual
//!   clock. That is the number a player feels.
//! - **Vsync off.** `PresentMode::AutoNoVsync` is forced on the primary window
//!   so a fast scene is not pinned to the monitor's refresh - we want the true
//!   per-frame cost and the headroom, not "60 fps, capped". A scene that cannot
//!   hold refresh shows its real (sub-refresh) rate either way.
//! - **Continuous updates.** `WinitSettings::game` keeps the loop running flat
//!   out even when the window is unfocused (the headless/Xvfb case), so the
//!   capture is not throttled to reactive redraws.
//! - **Fixed resolution.** The window is forced to a known size (default
//!   1280x720) so runs are comparable across machines and renderers.
//!
//! Chain [`FrameTimePlugin::drive`] (e.g. [`combat_burst_driver`]) to
//! measure an active scene - particle bursts and projectiles - not just at
//! rest.
//!
//! ## Usage
//!
//! ```no_run
//! # use bevy::prelude::*;
//! # use nova_probe::nova_frametime;
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
//! it by URL - see `scripts/perf-web.sh`). The `NOVA_PERF_*` prefix predates the
//! crate rename and stays until the runner-CLI task redesigns the surface. The
//! knobs:
//!
//! | Native env / wasm query | Default | Meaning |
//! |-------------------------|---------|---------|
//! | `NOVA_PERF` / `?perf`         | (unset) | Arms the plugin. |
//! | `NOVA_PERF_WARMUP` / `warmup` | `180`   | Frames discarded after reaching `Playing` (shader compile, asset upload, spikes; also lets a combat burst saturate). |
//! | `NOVA_PERF_FRAMES` / `frames` | `900`   | Frames captured for the stats window. |
//! | `NOVA_PERF_LABEL` / `label`   | `scene` | Label recorded in the row. |
//! | `NOVA_PERF_OUT` / (n/a)       | (none)  | Native only: dir for `<label>.json` + a `frametime.csv` row. Web has no fs, so it logs the summary line only. |
//! | `NOVA_PERF_RES` / `res`       | `1280x720` | Forced primary-window resolution `WxH`. |
//! | `NOVA_PERF_RENDER_SCALE` / `render_scale` | (tier default) | Forces `GraphicsBudget::render_scale`, holding the rest of the preset fixed - isolates the render-scale lever (measure a tier at `1.0` vs a fraction; task 20260718-004723). |
//! | `NOVA_PERF_QUALITY` / `quality` | (app default) | Graphics preset for the run (read by the example/bin); recorded in the run metadata. |
//! | `NOVA_PERF_SHA` / `sha`       | `git rev-parse` | Overrides the recorded git SHA (the web build cannot shell out). |
//! | `NOVA_PERF_HOST` / `host`     | `/etc/hostname` | Overrides the recorded host tag (`browser` on wasm). |

pub mod capture;
// The run-timeline recorder writes a JSONL file; the browser has no
// filesystem, so the module is native-only and wasm gets no-op stubs with the
// same signatures (cross-target callers compile; the runner-CLI task owns the
// web story).
#[cfg(not(target_arch = "wasm32"))]
pub mod recorder;
#[cfg(target_arch = "wasm32")]
pub mod recorder {
    //! Wasm stubs for the native-only run-timeline recorder.
    use bevy::prelude::*;

    /// No-op on wasm (no filesystem for the JSONL sink).
    pub fn nova_timeline() -> RunRecorderPlugin {
        RunRecorderPlugin
    }

    /// Inert wasm stand-in for the native recorder plugin.
    pub struct RunRecorderPlugin;

    impl RunRecorderPlugin {
        /// No-op on wasm.
        pub fn out(self, _path: impl Into<std::path::PathBuf>) -> Self {
            self
        }
    }

    impl Plugin for RunRecorderPlugin {
        fn build(&self, _app: &mut App) {}
    }

    /// No-op on wasm.
    pub fn probe_marker(_world: &mut World, _name: &str, _data: serde_json::Value) {}
}
pub mod report;
pub mod stats;

pub use capture::{
    combat_burst_driver, nova_frametime, perf_armed, perf_param, FrameTimePlugin, PerfDriver,
    DEFAULT_CAPTURE_FRAMES, DEFAULT_RESOLUTION, DEFAULT_WARMUP_FRAMES, PERF_ENV,
};
pub use recorder::{nova_timeline, probe_marker, RunRecorderPlugin};
#[cfg(not(target_arch = "wasm32"))]
pub use recorder::{parse_timeline, ProbeTimeline, TimelineEvent};
pub use stats::{parse_frametime_csv, FrameStats, PerfRun, RunMeta, CSV_HEADER, CSV_HEADER_V1};
