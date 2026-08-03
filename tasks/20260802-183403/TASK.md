# Migrate nova_debug, nova_probe, and the example fleet onto nova_autopilot

- PRIORITY: 92
- TAGS: v0.10.0, tooling, autopilot, examples
- KIND: TASK
- ACTIVITY: WORKING
- GATES: PLAN
- RESOLUTION: -
- PARENT: 20260802-120019
- DEPENDS ON: 20260802-183349, 20260802-183352

## Story

Switch Nova onto `nova_autopilot`. `nova_debug::harness` keeps only the
Nova-shaped layer - the `GameStates` presets, the scenario-loaded smoke
assertion, and the reel hooks (camera pose, body freeze, HUD and overlay
hiding) - over the crate's drivers. `nova_probe` registers its capture
collector against the crate directly instead of reaching through
`nova_gameplay::bevy_common_systems`. One atomic migration, because the
activation envs rename with no compatibility aliases.

The four ports left three reach-ins to the caller (`hide_overlay`, `ready`,
`ReelBeat::apply`) and dropped `ReelCamera` outright. `nova_debug` is where
those hooks get filled in, so the example fleet keeps the vocabulary it already
uses and this migration stays a rename plus an adapter layer, not 25 example
rewrites. See `DECISION.md` for the adapter shape and the two call-site changes
it accepts.

## Steps

- [ ] Add the dependency edge: `nova_autopilot = { path = "../nova_autopilot" }`
      in `crates/nova_debug/Cargo.toml` and `crates/nova_probe/Cargo.toml`.
      Direction is one way - neither crate is ever named by `nova_autopilot`.
- [ ] Rewrite `crates/nova_debug/src/harness.rs` as the Nova adapter over the
      crate:
      - re-export `AutopilotPlugin`, `AutopilotLoop`, `ScreenshotPlugin`,
        `ReelBeat` and `capture_window` from `nova_autopilot` (replacing the
        `bevy_common_systems::debug::harness::prelude` re-export and deleting
        the local `capture_window` / `reel_capture_path` twins);
      - keep `nova_autopilot()` as is; give `nova_screenshot()` the
        `.hide_overlay(hide_dev_overlays)` hook;
      - change `hide_dev_overlays` from a parameterised system to an exclusive
        `fn(&mut World)` so one function satisfies both the `Startup` system
        registration the screenshot examples already use and the crate's
        `Fn(&mut World)` hook;
      - keep `ReelCamera`, `reel_pose_camera` and the `ScenarioLoaded` smoke
        assertion; add `reel_beat(camera, path) -> ReelBeat` wiring the pose
        into `ReelBeat::apply`, and `nova_reel(beats) -> impl Plugin` adding
        `ScreenshotReelPlugin` with the `ready` (scenario camera present) and
        `hide_overlay` (overlays + `HudVisibility::Cinematic`) hooks plus the
        `reel_freeze_bodies` system, all gated on `REEL_ENV`;
      - delete the moved driver internals (`ReelState`, `ReelWindowSize`,
        `ReelCaptureDone`, `reel_drive`, `reel_resize_window`,
        `SCREENSHOT_REEL_ENV`, `REEL_CAPTURE_RESOLUTION`).
        `NOVA_AUTOPILOT_SECS` and `NOVA_SCREENSHOT_SETTLE_FRAMES` stay - the
        presets and an example read them.
- [ ] Update `crates/nova_debug/src/lib.rs`: import `AUTOPILOT_ENV` from
      `nova_autopilot` instead of `bevy_common_systems::debug::harness`, and
      refresh the prelude (`nova_reel` and `reel_beat` in, `ScreenshotReelPlugin`
      out) and the module docs.
- [ ] Point `nova_probe` at the crate: `crates/nova_probe/src/capture.rs` takes
      `nova_autopilot::completion::{self, HarnessCompletion}` instead of the
      `nova_gameplay::bevy_common_systems` path;
      `crates/nova_probe/src/bin/probe/native/env.rs` writes the renamed envs
      through the crate's consts (`autopilot::AUTOPILOT_ENV`,
      `screenshot::SCREENSHOT_ENV`, `completion::DEADLINE_ENV`) rather than
      literals, and its unit tests assert the new keys;
      `crates/nova_probe/src/bin/probe/native/spec.rs` updates its pass prose.
- [ ] Rename the activation contract in the remaining readers and writers:
      the `HarnessMute` env list in `crates/nova_gameplay/src/settings.rs:92`
      (string literals - `nova_gameplay` does not take the dependency, see
      `DECISION.md`), the `BCS_SHOT` note and the `RUST_LOG` filter lists in
      `crates/nova_core/src/lib.rs`, the doc comments in
      `crates/nova_ui/src/widget/slider.rs`, the `.env("BCS_AUTOPILOT", "1")`
      spawn and header docs in `tests/examples_smoke.rs`, and the command
      recipes in `scripts/gen-web-screenshots.py`.
- [ ] Sweep the example fleet: every `std::env::var_os("BCS_*")` read
      (`menu_newgame`, `menu_scenarios`, `screenshot_combat`, `screenshot_juice`,
      `screenshot_orbit`) and every `//!` run recipe across `examples/`. Switch
      the two reel call sites (`screenshot_reel.rs:65`,
      `screenshot_sections.rs:46,159`) onto `nova_reel(..)` and `reel_beat(..)`.

## Definition of Done

- No repository-owned code names a BCS activation env or harness path.
  (cmd: `! rg -n "BCS_AUTOPILOT|BCS_SHOT|BCS_REEL|BCS_HARNESS_DEADLINE|debug::harness" crates examples scripts tests --glob '*.rs' --glob '*.py'`)
- `nova_debug` and `nova_probe` depend on the crate directly.
  (cmd: `rg -n '^nova_autopilot' crates/nova_debug/Cargo.toml crates/nova_probe/Cargo.toml`)
- The workspace builds with the debug feature and all targets.
  (cmd: `nix develop --command cargo check --workspace --all-targets --features debug`)
- The example fleet still smokes under the renamed env.
  (cmd: `nix develop --command cargo test --test examples_smoke`)
- A probe run still produces a report with the capture collector negotiating
  the exit. (cmd: `nix develop --command cargo run -p nova_probe -- run playable --fps`)
- The reel still captures every beat after the hooks moved to the caller.
  (manual: run `NOVA_SHOT_DIR=target/reel NOVA_REEL=1 cargo run --example screenshot_reel --features debug` under Xvfb and confirm three PNGs land, framed as before)

## Notes

- Parent: `20260802-120019`. Depends on the driver ports.
- A harness run mutes audio off the env list in
  `crates/nova_gameplay/src/settings.rs`; missing it makes probe runs audible.
- `nova_probe` gains a direct `nova_autopilot` dependency; the direction is
  probe -> autopilot, never the reverse.
- Base-branch proof status, checked 2026-08-03: the absence grep hits ~25 files
  and the Cargo.toml grep exits 1, so both are red. `cargo check`,
  `examples_smoke` and the probe run are green on base - they are the regression
  guards that make this atomic rename safe, not discovery proofs.
- Do NOT reach for a compatibility alias. The epic decided a hard rename; a
  half-renamed tree that still boots is the failure mode this atomicity guards
  against.
- `nova_autopilot::completion` is compiled for wasm through
  `crates/nova_probe/src/capture.rs`; the crate is `bevy`-only, so the wasm
  target should be unaffected. Confirm with the existing web build rather than
  assuming.
- `examples/screenshots/screenshot_reel.rs:164` reads
  `nova_debug::harness::NOVA_AUTOPILOT_SECS` through the full path; that const
  stays in `nova_debug`, so the line is untouched.
- Watch the `ScreenshotPlugin` name clash: `nova_debug`'s prelude deliberately
  withholds it so a glob next to `bevy::prelude::*` stays clean
  (`crates/nova_debug/src/lib.rs:34`). Keep it reachable only under
  `nova_debug::harness::`, which is how `hud_range` and `com_range` reach
  `AutopilotPlugin` today.
- The docs sweep (`AGENTS.md`, `web/src/wiki/dev/*`, `CHANGELOG.md`) is
  `20260802-183406`, deliberately not here - this task is code and its run
  recipes only.
