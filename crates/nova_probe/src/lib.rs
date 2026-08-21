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
//!   flight as one JSON object, on demand), `capabilities::census` (what the
//!   world contains while a window runs) and `capabilities::framecost` (where
//!   the milliseconds in that window went, by name). [`NovaProbePlugin`]
//!   bundles them all.
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
//! - **The scene STILL THERE.** A scene can end before its clock stops. A fight
//!   whose losing side is gone is over while `Time<Virtual>` still ticks and
//!   every gate here still passes, and the window then measures the aftermath -
//!   a near-empty scene, at a fraction of the cost, in an ordinary-looking row.
//!   Only the example knows, so it says so with
//!   [`FrameTimePlugin::live_while`], and the capture REFUSES the first frame
//!   the predicate fails.
//! - **Vsync off.** `PresentMode::AutoNoVsync` is forced on the primary window
//!   so a fast scene is not pinned to the monitor's refresh - we want the true
//!   per-frame cost and the headroom, not "60 fps, capped". It is a REQUEST:
//!   wgpu falls back Immediate -> Mailbox -> Fifo on what the surface offers and
//!   bevy logs the fallback only for an explicitly named mode, so
//!   `NOVA_PROBE_PRESENT=immediate` names the mode, and the capture then REFUSES
//!   a window whose deltas collapsed onto one period anyway - a compositor or a
//!   silent fallback pacing the swap chain reads as a plausible number and is
//!   the display's, not the game's.
//! - **Continuous updates.** `WinitSettings::continuous` keeps the loop running
//!   flat out even when the window is unfocused, and the capture REFUSES any
//!   other setting. Bevy's default `WinitSettings::game` holds an unfocused
//!   window at 60 Hz: measured on the empty gallery, 3.4 ms focused and
//!   16.67 ms not, from the event loop rather than from the frame.
//! - **Fixed resolution.** The window is sized (default 1280x720) BEFORE winit
//!   creates it, so the size hints, the surface and `Window` agree from the
//!   start - a resize asked for afterwards is one a reparenting window manager
//!   simply refuses while `Window` goes on reporting it. The capture then
//!   refuses a window that is a different size anyway. Frame cost is a function
//!   of window PIXELS, so a run at the wrong size is comparable with nothing.
//! - **The PRESENTATION PATH is part of the number, and Xvfb is a bad one.**
//!   A software X server has no scanout, so presenting a window is a CPU-side
//!   copy of every pixel, charged to the render thread inside `render_system`
//!   after the graph. Measured on this project's host at 1280x720: 11.5 ms a
//!   frame under `xvfb-run`, 0.12 ms against a real display, with the game
//!   identical. It is an ADDITIVE per-pixel constant, not a scale factor: an
//!   A/B whose arms share a window size divides it out and its RATIO stands,
//!   while any absolute figure - a budget, an FPS gate, a floor - does not.
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
//! Run it on a REAL display. A capture wears `WM_CLASS`
//! [`nova_core::MEASURE_WINDOW_CLASS`], so a window manager can place it off
//! the operator's desk without ever catching a hand-run; on i3, one line of
//! config does it and a hidden workspace measures the same as a visible one:
//!
//! ```text
//! # ~/.config/i3/config
//! for_window [class="nova-measure"] move container to workspace 3
//! ```
//!
//! ```text
//! NOVA_PROBE=1 NOVA_PROBE_LABEL=stress_bullets-gpu NOVA_PROBE_PRESENT=immediate \
//!   NOVA_PROBE_OUT=/tmp/perf BEVY_ASSET_ROOT="$PWD" DISPLAY=:0 \
//!   cargo run --example stress_bullets --features debug
//! # look for: `nova perf: label=... frames=... mean=..ms p99=..ms mean_fps=.. 1%low_fps=..`
//! ```
//!
//! ## Config source
//!
//! Parameters come from [`probe_param`]: **native** reads env vars
//! `NOVA_PROBE_<UPPER>`; **wasm** reads the URL query `<name>` (so a browser drives
//! it by URL - see `probe run --platform web`). The knobs:
//!
//! | Native env / wasm query | Default | Meaning |
//! |-------------------------|---------|---------|
//! | `NOVA_PROBE` / `?perf`         | (unset) | Arms the plugin. |
//! | `NOVA_PROBE_WARMUP` / `warmup` | `180`   | Frames discarded after reaching `Playing` (shader compile, asset upload, spikes; also lets a combat burst saturate). Wins over an example's declared window. |
//! | `NOVA_PROBE_FRAMES` / `frames` | `900`   | Frames captured for the stats window. Wins over an example's declared window. |
//! | `NOVA_PROBE_LABEL` / `label`   | `scene` | Label recorded in the row. |
//! | `NOVA_PROBE_OUT` / (n/a)       | (none)  | Native only: dir for `<label>.json` + a `frametime.csv` row. Web has no fs, so it logs the summary line only. |
//! | `NOVA_PROBE_RES` / `res`       | `1280x720` | Forced primary-window resolution `WxH`. |
//! | `NOVA_PROBE_RENDER_SCALE` / `render_scale` | (tier default) | Forces `GraphicsBudget::render_scale`, holding the rest of the preset fixed - isolates the render-scale lever (measure a tier at `1.0` vs a fraction). |
//! | `NOVA_PROBE_MAX_DELTA` / `max_delta` | (bevy's 0.25 s) | Forces `Time<Virtual>::max_delta`, in seconds - the ceiling on how many fixed steps one frame may run. Isolates fixed-step amplification; it trades a bounded tail for simulation time the world never runs, so it is a measurement knob, never a default. |
//! | `NOVA_PROBE_PRESENT` / `present`   | `autonovsync` | Presentation mode forced on the primary window (`immediate`, `mailbox`, `fifo`, `fiforelaxed`, `autovsync`, `autonovsync`). The default is only a REQUEST - name a mode explicitly and bevy logs the fallback when the surface cannot serve it. |
//! | `NOVA_PROBE_CENSUS_FRAME` / `census_frame` | `90` | Frames after `Playing` at which the scene census is taken. |
//! | `NOVA_PROBE_FRAMECOST_FRAMES` / `framecost_frames` | `200` | Frames averaged into one frame-cost report. |
//! | `NOVA_PROBE_RENDER_DIAG` | (unset) | Asks the renderer for GPU timestamp queries, so the frame-cost report can name each render pass. Costs a resolve pass and a readback per frame - a measurement knob, never a default. |
//! | `NOVA_PROBE_QUALITY` / `quality` | (app default) | Graphics preset for the run (read by the example/bin); recorded in the run metadata. |
//! | `NOVA_PROBE_SHA` / `sha`       | `git rev-parse` | Overrides the recorded git SHA (the web build cannot shell out). |
//! | `NOVA_PROBE_HOST` / `host`     | `/etc/hostname` | Overrides the recorded host tag (`browser` on wasm). |
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
