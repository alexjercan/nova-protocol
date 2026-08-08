//! nova_probe: the IN-GAME half of the run-harness - the plugins an example
//! wires to collect evidence about its own run, plus the wire format that
//! evidence is written in. The host half (spawning runs, grading artifacts,
//! rendering reports) is `nova_probe_cli`; the two meet at the filesystem, so
//! nothing here reads a run's output back.
//!
//! - [`capture`] - the env-gated [`nova_frametime`] plugin: drives a real
//!   gameplay app to `Playing`, warms up, records the wall-clock delta of
//!   every frame for a fixed window, then writes percentile stats and exits.
//! - [`recorder`] - the run-timeline JSONL sink.
//! - [`invariants`] - continuous invariant checks, riding that sink.
//! - [`contract`] - what an example CLAIMS to collect, declared by the plugins
//!   it wires.
//! - [`stats`] - [`FrameStats`], the per-run [`RunMeta`], and the CSV/JSON
//!   schema (writers + parsers) both halves speak.
//! - [`fixtures`] - the scenario builders the examples share.
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
//! lavapipe software-raster floor - see `probe run --render sw`):
//!
//! ```text
//! Xvfb :95 -screen 0 1280x720x24 &
//! NOVA_PERF=1 NOVA_PERF_SCENARIO=asteroid_field NOVA_PERF_LABEL=asteroid_field-gpu \
//!   NOVA_PERF_OUT=/tmp/perf BEVY_ASSET_ROOT="$PWD" DISPLAY=:95 \
//!   cargo run --release --example scene_baseline --features debug
//! # look for: `nova perf: label=... frames=... mean=..ms p99=..ms mean_fps=.. 1%low_fps=..`
//! ```
//!
//! ## Config source
//!
//! Parameters come from [`perf_param`]: **native** reads env vars
//! `NOVA_PERF_<UPPER>`; **wasm** reads the URL query `<name>` (so a browser drives
//! it by URL - see `probe run --platform web`). NOTE: the `NOVA_PERF_*` prefix
//! predates the crate rename and stays, so scripts and docs do not churn. The
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
//! | `NOVA_PERF_RENDER_SCALE` / `render_scale` | (tier default) | Forces `GraphicsBudget::render_scale`, holding the rest of the preset fixed - isolates the render-scale lever (measure a tier at `1.0` vs a fraction). |
//! | `NOVA_PERF_QUALITY` / `quality` | (app default) | Graphics preset for the run (read by the example/bin); recorded in the run metadata. |
//! | `NOVA_PERF_SHA` / `sha`       | `git rev-parse` | Overrides the recorded git SHA (the web build cannot shell out). |
//! | `NOVA_PERF_HOST` / `host`     | `/etc/hostname` | Overrides the recorded host tag (`browser` on wasm). |
#![warn(missing_docs)]

pub mod capture;
// What an example CLAIMS, declared by the plugins it wires. Both targets: the
// frame-time capture builds for wasm and declares like the rest (only the
// filesystem write is native-only).
pub mod contract;
// Continuous invariant checks ride the recorder's timeline sink, so they are
// native-only with it (nothing wasm-side references them - the examples that
// wire them never build for wasm).
#[cfg(not(target_arch = "wasm32"))]
pub mod invariants;
// Scenario fixture builders shared by the examples. Native-only with the rest
// of the example-facing harness: nothing in the wasm bundle builds a scenario
// config by hand.
#[cfg(not(target_arch = "wasm32"))]
pub mod fixtures;
// The run-timeline recorder writes a JSONL file; the browser has no
// filesystem, so the module is native-only and wasm gets no-op stubs with the
// same signatures, so cross-target callers compile.
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
pub mod stats;

pub use capture::{
    capture_reload_begin, capture_reload_end, capture_reloading, combat_burst_driver,
    nova_frametime, perf_armed, perf_param, FrameTimePlugin, PerfDriver, ReloadGate,
    DEFAULT_CAPTURE_FRAMES, DEFAULT_RESOLUTION, DEFAULT_WARMUP_FRAMES, PERF_ENV,
};
pub use contract::{Capability, ProbeContract};
#[cfg(not(target_arch = "wasm32"))]
pub use invariants::{nova_invariants, InvariantState, InvariantsPlugin};
pub use recorder::{nova_timeline, probe_marker, RunRecorderPlugin};
#[cfg(not(target_arch = "wasm32"))]
pub use recorder::{parse_timeline, ProbeTimeline, TimelineEvent};
pub use stats::{
    append_frametime_row, parse_frametime_csv, parse_summary_line, FrameStats, PerfRun, RunMeta,
    CSV_HEADER, CSV_HEADER_V1,
};
