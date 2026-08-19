//! nova_probe: the IN-GAME half of the run-harness - the plugins an example
//! wires to collect evidence about its own run, plus the wire format that
//! evidence is written in. The host half (spawning runs, grading artifacts,
//! rendering reports) is `nova_probe_cli`; the two meet at the filesystem, so
//! nothing here reads a run's output back.
//!
//! - [`capabilities`] - one module per kind of evidence, each a plugin an
//!   example wires: [`capabilities::frametime`] (the env-gated
//!   [`nova_frametime`] capture - drives a real gameplay app to `Playing`,
//!   warms up, records the wall-clock delta of every frame for a fixed window,
//!   then writes percentile stats and exits), [`capabilities::timeline`] (the
//!   run-timeline JSONL sink), `capabilities::invariants` (continuous
//!   invariant checks, riding that sink) and `capabilities::snapshot` (the
//!   world-state serializer: every ship, section, fixture, weapon and round in
//!   flight as one JSON object, on demand). [`NovaProbePlugin`] bundles all
//!   four.
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
//! - **Every captured frame SIMULATED.** Because the deltas are wall-clock, a
//!   scene whose simulation has stopped - a result screen, a pause menu, an
//!   outcome overlay - keeps producing them, at a steady and entirely plausible
//!   cost. The capture reads `Time<Virtual>` to detect that directly and
//!   REFUSES the window: it logs at ERROR and writes no stats, because a
//!   statistic over a still picture is worse than a missing one. A scene that
//!   can reach an end therefore needs a window sized to close before it does
//!   ([`FrameTimePlugin::window`]).
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
//! NOVA_PERF=1 NOVA_PERF_LABEL=stress_bullets-gpu \
//!   NOVA_PERF_OUT=/tmp/perf BEVY_ASSET_ROOT="$PWD" DISPLAY=:95 \
//!   cargo run --release --example stress_bullets --features debug
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
//! | `NOVA_PERF_WARMUP` / `warmup` | `180`   | Frames discarded after reaching `Playing` (shader compile, asset upload, spikes; also lets a combat burst saturate). Wins over an example's declared window. |
//! | `NOVA_PERF_FRAMES` / `frames` | `900`   | Frames captured for the stats window. Wins over an example's declared window. |
//! | `NOVA_PERF_LABEL` / `label`   | `scene` | Label recorded in the row. |
//! | `NOVA_PERF_OUT` / (n/a)       | (none)  | Native only: dir for `<label>.json` + a `frametime.csv` row. Web has no fs, so it logs the summary line only. |
//! | `NOVA_PERF_RES` / `res`       | `1280x720` | Forced primary-window resolution `WxH`. |
//! | `NOVA_PERF_RENDER_SCALE` / `render_scale` | (tier default) | Forces `GraphicsBudget::render_scale`, holding the rest of the preset fixed - isolates the render-scale lever (measure a tier at `1.0` vs a fraction). |
//! | `NOVA_PERF_MAX_DELTA` / `max_delta` | (bevy's 0.25 s) | Forces `Time<Virtual>::max_delta`, in seconds - the ceiling on how many fixed steps one frame may run. Isolates fixed-step amplification; it trades a bounded tail for simulation time the world never runs, so it is a measurement knob, never a default. |
//! | `NOVA_PERF_QUALITY` / `quality` | (app default) | Graphics preset for the run (read by the example/bin); recorded in the run metadata. |
//! | `NOVA_PERF_SHA` / `sha`       | `git rev-parse` | Overrides the recorded git SHA (the web build cannot shell out). |
//! | `NOVA_PERF_HOST` / `host`     | `/etc/hostname` | Overrides the recorded host tag (`browser` on wasm). |
#![warn(missing_docs)]

pub mod capabilities;
// What an example CLAIMS, declared by the plugins it wires. Both targets: the
// frame-time capture builds for wasm and declares like the rest (only the
// filesystem write is native-only).
pub mod contract;
// Scenario fixture builders shared by the examples. Native-only with the rest
// of the example-facing harness: nothing in the wasm bundle builds a scenario
// config by hand.
#[cfg(not(target_arch = "wasm32"))]
pub mod fixtures;
pub mod stats;

/// Everything an example needs to be probeable: the capability plugins, the
/// contract they declare into, and the wire format they write.
///
/// Each module owns its own `prelude`, so publishing a new name is a one-file
/// edit rather than an edit here as well. `fixtures` is deliberately absent -
/// scenario builders are wired explicitly by the examples that want them.
pub mod prelude {
    pub use crate::{capabilities::prelude::*, contract::prelude::*, stats::prelude::*};
}

pub use prelude::*;
