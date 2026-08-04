# Migrate nova_debug, nova_probe, and the example fleet onto nova_autopilot

- PRIORITY: 92
- TAGS: v0.10.0, tooling, autopilot, examples
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE
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

- [x] Add the dependency edge: `nova_autopilot = { path = "../nova_autopilot" }`
      in `crates/nova_debug/Cargo.toml` and `crates/nova_probe/Cargo.toml`.
      Direction is one way - neither crate is ever named by `nova_autopilot`.
- [x] Rewrite `crates/nova_debug/src/harness.rs` as the Nova adapter over the
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
- [x] Update `crates/nova_debug/src/lib.rs`: import `AUTOPILOT_ENV` from
      `nova_autopilot` instead of `bevy_common_systems::debug::harness`, and
      refresh the prelude (`nova_reel` and `reel_beat` in, `ScreenshotReelPlugin`
      out) and the module docs.
- [x] Point `nova_probe` at the crate: `crates/nova_probe/src/capture.rs` takes
      `nova_autopilot::completion::{self, HarnessCompletion}` instead of the
      `nova_gameplay::bevy_common_systems` path;
      `crates/nova_probe/src/bin/probe/native/env.rs` writes the renamed envs
      through the crate's consts (`autopilot::AUTOPILOT_ENV`,
      `screenshot::SCREENSHOT_ENV`, `completion::DEADLINE_ENV`) rather than
      literals, and its unit tests assert the new keys;
      `crates/nova_probe/src/bin/probe/native/spec.rs` updates its pass prose.
- [x] Rename the activation contract in the remaining readers and writers:
      the `HarnessMute` env list in `crates/nova_gameplay/src/settings.rs:92`
      (string literals - `nova_gameplay` does not take the dependency, see
      `DECISION.md`), the `BCS_SHOT` note and the `RUST_LOG` filter lists in
      `crates/nova_core/src/lib.rs`, the doc comments in
      `crates/nova_ui/src/widget/slider.rs`, the `.env("BCS_AUTOPILOT", "1")`
      spawn and header docs in `tests/examples_smoke.rs`, and the command
      recipes in `scripts/gen-web-screenshots.py`.
- [x] Sweep the example fleet: every `std::env::var_os("BCS_*")` read
      (`menu_newgame`, `menu_scenarios`, `screenshot_combat`, `screenshot_juice`,
      `screenshot_orbit`) and every `//!` run recipe across `examples/`. Switch
      the two reel call sites (`screenshot_reel.rs:65`,
      `screenshot_sections.rs:46,159`) onto `nova_reel(..)` and `reel_beat(..)`.
      ALSO (not in the plan, found by running the fleet): the ten examples that
      named a BARE `AutopilotPlugin`, which resolved to the bcs prelude's inert
      twin, and the four that reported completion through
      `nova_gameplay::bevy_common_systems::completion`. Both classes compile
      clean either way - see `DECISION.md`'s two addenda.

## Definition of Done

- No repository-owned code names a BCS activation env, the BCS harness path, or
  the BCS completion protocol. (The path is spelled in full: the bare
  `debug::harness` the plan first wrote also matches the `nova_debug::harness::`
  paths the Notes below say must survive - see `DECISION.md`'s addendum.)
  (cmd: `! rg -n "BCS_AUTOPILOT|BCS_SHOT|BCS_REEL|BCS_HARNESS_DEADLINE|bevy_common_systems::debug::harness|bevy_common_systems::completion" crates examples scripts tests --glob '*.rs' --glob '*.py'`)
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
  target should be unaffected. CONFIRMED 2026-08-03, not assumed:
  `cargo check -p nova_probe --lib --target wasm32-unknown-unknown` is clean.
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

## Close-out

### What and why

`nova_debug::harness` is now the Nova adapter over `nova_autopilot` rather than
a preset layer over the `bevy_common_systems` harness. It re-exports the
drivers, keeps the Nova-shaped pieces (`GameStates` presets, the `ScenarioLoaded`
smoke assertion, `ReelCamera`, `reel_pose_camera`, body freezing, overlay
hiding) and fills the crate's three caller hooks through two new adapters -
`reel_beat(camera, path)` for `ReelBeat::apply`, and `nova_reel(beats)` wiring
`ready` + `hide_overlay` and adding `reel_freeze_bodies`. The driver internals
(`ReelState`, `reel_drive`, `reel_resize_window`, the local `capture_window` /
`reel_capture_path` twins) are gone: -434/+263 in that one file. `nova_probe`
names `nova_autopilot::completion` directly and writes the child-run envs
through the crate's consts instead of literals. The activation contract renamed
`BCS_* -> NOVA_*` everywhere in one commit, no aliases, per the epic.

`hide_dev_overlays` became an exclusive `fn(&mut World)` so a single function
serves both the `Startup` registration four screenshot examples use and the
drivers' `Fn(&mut World)` hook - the one signature change the fleet felt.

### Alternatives

The adapter-layer choice, the rejected alternatives (per-example hooks, pushing
hooks back into the crate, a converting Nova `ReelBeat`, aliases) and the
`nova_gameplay` string-literal call are in `DECISION.md`, decided at plan time
and unchanged by the work. Two decisions were made DURING the work and are
recorded as `DECISION.md` addenda: routing the four self-ending examples'
completion reach-in through `nova_debug::harness` re-exports, and guarding the
bare-driver-name shadow with a test rather than by re-exporting
`AutopilotPlugin` from `nova_debug`'s prelude (which would re-open the
`ScreenshotPlugin`-versus-`bevy` clash the prelude's withholding prevents).

### Difficulties and diagnosis

Two breaks, both invisible to `cargo check`, both found only by RUNNING the
fleet - which is exactly why the plan made `examples_smoke` a DoD proof.

1. Four self-ending examples (`broadside`, `lifeline`, `menu_scenarios`,
   `screenshot_nova_os`) reported their collector done through
   `nova_gameplay::bevy_common_systems::completion`. Nothing registers that
   resource any more, so `resource_mut` would have panicked mid-script. Found by
   grepping for bcs reach-ins the plan's absence proof could not express (they
   carry no `BCS_*` env and no harness path); fixed before it ever ran.

2. `playable` aborted on frame 1 with `MessageReader<AutopilotLoop>::messages
   failed validation: Message not initialized`. Diagnosis: no
   `AutopilotPlugin: build` line in the log at all, while `scenario` (which uses
   the `nova_autopilot()` preset) had one - so the plugin was inert despite
   `NOVA_AUTOPILOT=1`. The cause was name resolution, not the env: ten examples
   named a BARE `AutopilotPlugin`, which `nova_debug`'s prelude deliberately
   withholds, so it resolved to the bcs prelude's twin arriving through
   `nova_protocol::prelude::*`. Those ten were still building the BCS driver.
   `playable` failed loudly only because it also reads the MIGRATED
   `AutopilotLoop`; the other nine would have booted silently autopilot-less.
   Full write-up in `DECISION.md` addendum 2.

### Evidence

| Proof | Result |
| --- | --- |
| absence grep (corrected, see below) | exit 1, clean |
| `rg '^nova_autopilot' <both Cargo.toml>` | both present |
| `cargo check --workspace --all-targets --features debug` | clean |
| `cargo test --test examples_smoke` | 6 passed, 0 failed (119s) |
| `cargo run -p nova_probe -- run playable --fps` | verdict OK; 5 PASS, 1 SKIPPED (needs a baseline) |
| reel capture (manual) | 3 PNGs at 1920x1080, overlays + HUD hidden, framing per the beat comments |
| `cargo test -p nova_debug --lib` | 12 passed |
| `cargo test -p nova_probe --bin probe` | 26 passed |
| `cargo check -p nova_probe --lib --target wasm32-unknown-unknown` | clean |

The probe run proves the exit is still NEGOTIATED, not unilateral, now through
`nova_autopilot::completion`: `capture done (1 still pending)` at t+11s,
`autopilot done (0 still pending)` at t+29s, then `all collectors done,
exiting`.

The DoD's absence grep was corrected: its bare `debug::harness` alternative also
matched the `nova_debug::harness::` paths the task's own Notes require to
survive, so it could never have gone green. It now names
`bevy_common_systems::debug::harness` - what it was written to catch - and gains
`bevy_common_systems::completion` for finding 1.

New regression guard: `examples_smoke::examples_name_drivers_through_the_nova_harness`
fails any example naming `AutopilotPlugin`, `AutopilotLoop`, `ScreenshotPlugin`,
`ScreenshotReelPlugin` or `HarnessCompletion` without the
`nova_debug::harness::` path. Display-free, so it runs on a bare `cargo test`
next to `catalog_matches_disk`. Verified failing-for-the-right-reason before
being taken green.

### Reflection

The plan's proof set was sound where it looked, and blind where it did not: it
reasoned about the rename as a STRING substitution, so both real breaks - a
resource reach-in and a name-resolution shadow - sat outside every `cmd:` proof
it wrote. What caught them was the one proof that runs the code. The lesson for
the next migration off a glob-exporting prelude: enumerate the names the OLD
prelude exports that the new one does not, because those are precisely the sites
where deleting the old wiring leaves working, compiling, silently-wrong code. A
grep for the new name finds nothing there; only a grep for the ABSENCE of
qualification does, which is why that became a test rather than a one-off check.

One process note: recovering from the fail-first check on the new guard, a
`git checkout examples/gameplay/playable.rs` reverted that file's migration
edits along with the temporary one. Caught immediately and reapplied - but the
cheap habit is to revert with a targeted `sed` back, or stage first, rather than
to checkout a file that already carries uncommitted work.

### Verification, 20260803 (all DoD proofs run)

| Proof | Result |
|-|-|
| absence grep | green (exit 1, no hits) |
| Cargo.toml dependency grep | green (both crates) |
| `cargo check --workspace --all-targets --features debug` | green |
| `cargo fmt --check` | green |
| `cargo test --test examples_smoke` (Xvfb :99) | 6 passed, 0 failed, 107s |
| `cargo run -p nova_probe -- run playable --fps` | `OK`, 6/6 checks pass, fps +0.1% |
| reel manual proof (Xvfb :99) | 3 PNGs captured, framing unchanged |
| `cargo check --target wasm32-unknown-unknown -p nova_probe` | green |

The absence grep needed one more edit to go green: the guard test's own doc
prose spelled `BCS_AUTOPILOT` while explaining the shadow it guards. Reworded to
"the retired bcs activation env" - the proof is an absence grep, so it cannot
carve out prose without weakening what it checks.

The wasm check is the Notes' open question ("confirm with the existing web build
rather than assuming"): `nova_probe`'s new `nova_autopilot` dependency compiles
for `wasm32-unknown-unknown`, so the web build is unaffected.
