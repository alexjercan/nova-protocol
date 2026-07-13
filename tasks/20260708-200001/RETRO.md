# Retro: Share the ScenarioLoaded smoke assertion across scenario examples

- TASK: 20260708-200001
- BRANCH: test/scenario-assert-helper (squash-merged as a461b5a)
- REVIEW ROUNDS: 1 (self-review, no findings)

Follow-up to 20260708-194524, which added the per-example assertion to
`03_scenario`. This task extracts it and spreads it to the other two
scenario-loading smoke examples.

## What shipped

A reusable `nova_debug::harness` preset `assert_scenario_loaded(expected_id)`
returning a `ScenarioLoadedAssertPlugin`, mirroring the `nova_autopilot()` /
`nova_screenshot()` preset style. It observes `ScenarioLoaded`, asserts the id
matches and handler/object counts are non-zero, and guards the never-fired case
via a `fired` flag checked on entering `Playing`. `03_scenario`, `10_gameplay`
and `07_torpedo_guidance` all use it now, each passing its own id via a
`SCENARIO_ID` const shared with its `ScenarioConfig`. `nova_debug` gained a
`nova_scenario` dependency (no cycle: `nova_scenario` does not depend on
`nova_debug`). Verified headless: asteroid_field 8/22, gameplay_minimal 1/4,
torpedo_guidance 1/2, all `cycle complete, no panic`.

## What went well

- The previous task's build-cold-then-time-the-run lesson paid off immediately.
  I built all three examples in one cold pass, then ran each against the warm
  binary with a 60s per-run timeout. Zero wasted timeout-kill cycles this time,
  versus the one burned last task. Writing the lesson down and reading it back
  at the top of the task is exactly the compounding the retro loop is for.
- Checking the dependency direction before adding the dep. `nova_debug` already
  pulled `nova_gameplay`; confirming `nova_scenario` does not depend on
  `nova_debug` up front meant the new edge could not cycle, so `nova_debug` was
  the right home for the shared helper rather than a copy-pasted example module.
- Keeping the anti-drift constant per example. Each example owns a `SCENARIO_ID`
  used in both the `ScenarioConfig.id` and the assertion, so a future rename
  cannot desync the scene from the check. Verifying the log source was
  `nova_debug::harness` (not the old bespoke `03_scenario` code) confirmed the
  extraction actually replaced the duplicate rather than shadowing it.

## What went wrong

- The final `cargo test` ran from the workspace root, which tests only the root
  package, not the member crates -- so the `nova_scenario` unit tests did not
  actually run in the final suite (they were untouched this task and green from
  the prior task, and `clippy --all-targets` compiled every crate, so nothing
  regressed). Still, "I ran cargo test" meant less than it looked. For a
  workspace, the honest suite is `cargo test --workspace`.
- Minor: reasoned at length about a possible `ScreenshotPlugin` name clash from
  adding `use bevy::prelude::*` to `harness.rs` before just letting the compiler
  answer it (explicit `pub use` wins over the glob; no clash). The build is the
  cheapest oracle for "does this name resolve" -- ask it first.

## What to improve next time

- Run `cargo test --workspace` (not bare `cargo test`) as the verify step in
  this repo, since it is a root-package-plus-members workspace and the unit
  tests live in the members. Bare `cargo test` gives false comfort here.
- For "does this import/type resolve" questions, spend one compile instead of a
  paragraph of speculation. Reserve the reasoning for behavior the compiler
  cannot check (ordering, feel, invariants).

## Action items

- [x] `ScenarioLoaded` is now produced (133011), consumed in one example
  (194524), and shared across all three scenario-loading smoke examples via one
  helper (this task). The 0.4.0 testability loop for scenario init is complete.
- [ ] Standing lesson for this repo: verify with `cargo test --workspace`. Not
  yet promoted to AGENTS.md or a repo doc; do so if a third task trips on
  root-only `cargo test`.
