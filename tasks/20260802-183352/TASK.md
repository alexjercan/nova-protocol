# Add a runnable nova_autopilot example with a headless integration test

- PRIORITY: 94
- TAGS: v0.10.0, tooling, autopilot, testing
- KIND: TASK
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE
- PARENT: 20260802-120019
- DEPENDS ON: 20260802-183343

## Story

Prove the crate stands alone and give it the runnable-example seam later work
builds on: a `nova_autopilot` example that is a self-contained Bevy app with its
own three-state machine, driven end to end by the autopilot, plus an integration
test that runs it headless and asserts the exit and log lines. This is the
pattern `nova_probe` reuses for correctness and profiling runs.

## Steps

- [x] Add `crates/nova_autopilot/examples/driven_app.rs`: a self-contained
      `DefaultPlugins` app with its own `DemoState { Boot, Flying, Done }`,
      a camera, a light and one cube. `AutopilotPlugin::<DemoState>::new()`
      holds `Boot 0.5s -> Flying 2.0s -> Done 0.5s`; the `input` closure is
      gated to `Flying` and presses `KeyCode::Space`; an `Update` system reads
      `ButtonInput<KeyCode>` and translates the cube, logging
      `driven_app: thrust moved the cube` once. `fn main() -> AppExit` returns
      `app.run()`. No `nova_*` import but the crate itself.
- [x] Add the in-example behavior assertion: an `OnEnter(DemoState::Done)`
      system that panics when the cube never moved, so a driven run that stops
      driving fails the process instead of exiting green.
- [x] Add `crates/nova_autopilot/tests/autopilot_example.rs` with
      `autopilot_example_completes_a_cycle`: skip loudly (eprintln + return)
      when neither `DISPLAY` nor `WAYLAND_DISPLAY` is set; otherwise spawn
      `cargo run --quiet -p nova_autopilot --example driven_app` via
      `env!("CARGO")` with `NOVA_AUTOPILOT=1` and `NOVA_AUTOPILOT_DEADLINE=30`,
      then assert a success exit plus the stderr lines
      `autopilot: cycle complete, no panic`,
      `harness completion: all collectors done, exiting` and the example's own
      `driven_app: thrust moved the cube`. Failure messages print a `tail()` of
      stderr (same helper shape as `tests/examples_smoke.rs`).
- [x] RUN the example for real once under `Xvfb :99` (repository rule), then run
      the new test both with and without a display.
- [x] Wire the test into CI: add
      `xvfb-run --auto-servernum cargo test -p nova_autopilot --test autopilot_example`
      to the existing "Examples smoke test" step in `.github/workflows/ci.yaml`,
      with a comment stating the one-extra-Bevy-variant cost (see Notes).

## Definition of Done

- The example runs headless to a clean exit under the arming env.
  (test: `autopilot_example_completes_a_cycle`)
- The example builds as part of the crate's targets.
  (cmd: `nix develop --command cargo check -p nova_autopilot --examples`)
- The run is skipped, not failed, without a display.
  (cmd: `nix develop --command env -u DISPLAY -u WAYLAND_DISPLAY cargo test -p nova_autopilot --test autopilot_example`)
- The example imports no other Nova crate - the standing no-game-coupling proof.
  (cmd: `test -f crates/nova_autopilot/examples/driven_app.rs && ! rg -n 'nova_(assets|core|debug|editor|events|gameplay|info|menu|modding|mod_format|os|probe|scenario|ui)' crates/nova_autopilot/examples/`)
- The example has been run once for real under a display.
  (manual: `Xvfb :99 & DISPLAY=:99 NOVA_AUTOPILOT=1 nix develop --command cargo run -p nova_autopilot --example driven_app` exits 0 and logs the cycle-complete line; kill the recorded Xvfb PID)

## Notes

- Parent: `20260802-120019`. Depends on the autopilot driver port.
- Repository rule: an example is not done until it has been RUN once; use Xvfb
  `:99` locally.
- Keep the example free of Nova types - it is the standing proof the crate has
  no hidden game coupling.

Discovered facts (verified on `master` while planning):

- `autoexamples = false` is a `[package]` key on the ROOT manifest only, so
  `crates/nova_autopilot/examples/*.rs` is auto-discovered normally and needs no
  `[[example]]` block. The root `catalog_matches_disk` test reads
  `<root>/examples` only, so a crate-level example cannot trip it.
- `cargo check -p nova_autopilot --examples` currently prints
  `target filter 'examples' specified, but no targets matched; this is a no-op`
  and exits 0 - that DoD command is GREEN on base. The red proof for this task
  is the new test (target does not exist); the check command is the guard that
  the example stays a real, building target afterwards.
- `cargo test --workspace --features debug --test screenshot_stand_down` selects
  a target that exists in one non-root package and reuses the debug-feature
  build (13.7s, no Bevy rebuild) - so target-name selection across the
  workspace works, if the CI wiring ever wants that form.
- CI cost, stated up front: the root feature `debug = [..., "bevy/track_location"]`,
  and the CI "Tests"/"Examples smoke test" steps build with `--features debug`.
  The new test shells out to `cargo run -p nova_autopilot`, whose feature
  resolution has track_location OFF, so CI pays ONE extra Bevy variant
  (cached by `Swatinem/rust-cache` after the first run). Running the CI step as
  `-p nova_autopilot` with no `--features` keeps it at exactly one extra
  variant, since the test binary and its nested `cargo run` then share a graph.
- Prior art for the test shape: `tests/examples_smoke.rs` (spawn via
  `env!("CARGO")`, skip on missing `DISPLAY`, `tail()` on failure).
- Crate docs/prelude are `20260802-183355`; the example fleet migration is
  `20260802-183403`. This task touches neither.
- No user-visible behavior changes, so no CHANGELOG or wiki surface is due.

## Close-out

What/why: `crates/nova_autopilot/examples/driven_app.rs` is a standalone
`DefaultPlugins` app - own `DemoState { Boot, Flying, Done }`, camera, light,
one cube - driven `Boot 0.5s -> Flying 2.0s -> Done 0.5s` by
`AutopilotPlugin`, with the input closure gated to `Flying` pressing Space and
an `Update` system that reads real `ButtonInput` and translates the cube. An
`OnEnter(Done)` guard asserts the cube moved, so a run that stops being driven
exits 101 instead of green. `crates/nova_autopilot/tests/autopilot_example.rs`
spawns it via `env!("CARGO")` and asserts exit status plus the three log
lines; it skips loudly without a display.

Alternatives: `MinimalPlugins` (rejected in DECISION.md - the lib tests
already cover that; the uncovered thing is a windowed app's real state
machine, input collection and exit path). In-process run (rejected: the
subjects are the process exit code and the stderr a supervisor reads).

Deviation from Step 5: CI gets its OWN step ("Autopilot example") rather than
a second command inside "Examples smoke test". Same command and same
one-extra-variant cost as planned (`-p nova_autopilot`, no `--features`), but
a separate step attributes a failure to the crate instead of to the game's
example fleet.

Difficulties/diagnosis: the no-coupling DoD grep initially failed on the
example's own doc comment, which named `nova_probe` in prose. Reworded to "the
probe run-harness" with a comment saying why - the proof greps the whole
directory, comments included.

Evidence:
- Real run under `Xvfb :99`: exit 0, logged `autopilot: -> Flying (t=0.5s)`,
  `driven_app: thrust moved the cube`, `autopilot: -> Done (t=2.5s)`,
  `autopilot: cycle complete, no panic (t=3.0s)`,
  `harness completion: all collectors done, exiting`.
- Falsification: with the input closure short-circuited, the same run panicked
  in the `OnEnter(Done)` guard and exited 101 - the guard is load-bearing.
- `cargo test -p nova_autopilot --test autopilot_example` with `DISPLAY=:99`:
  1 passed (5.65s); with `-u DISPLAY -u WAYLAND_DISPLAY`: 1 passed, SKIP line
  printed.
- `cargo check -p nova_autopilot --examples`: clean (no longer a no-op).
- No-coupling grep: exit 0. `cargo fmt --check`: clean.

Reflection: the example is deliberately dumb - one cube, one key, one
assertion - because its job is to prove the seam, not to demo anything. Per
repository policy the full suite and clippy were NOT run locally; CI covers
them on the PR.
