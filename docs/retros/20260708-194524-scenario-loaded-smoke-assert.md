# Retro: Assert on ScenarioLoaded payload in the smoke harness

- TASK: 20260708-194524
- BRANCH: test/scenario-loaded-smoke-assert (squash-merged as 0af7f4e)
- REVIEW ROUNDS: 1 (self-review, no findings)

Follow-up to 20260525-133011, which added the `ScenarioLoaded` init-status
payload but left nothing reading it. This task closes that loop.

## What shipped

`examples/03_scenario.rs` now observes `ScenarioLoaded` under the `debug`
feature and fails the `BCS_AUTOPILOT` smoke run when scenario init is broken:
the observer asserts `scenario_id == "asteroid_field"` and `handler_count > 0`
and `object_count > 0`, and a `ScenarioLoadProbe { fired }` flag checked in
`OnEnter(GameStates::Playing)` catches the case where the event never fires at
all. A `SCENARIO_ID` const is shared between the loader and the assertion so
they cannot drift. Verified headless: `handlers=8 objects=22`, reached Playing,
`cycle complete, no panic`.

## What went well

- Reading the harness before writing. `crates/nova_debug/src/harness.rs` spells
  out that the autopilot holds `Loading` for 6s and exits `AppExit::Success`,
  and that the loader drives `Loaded -> Playing` asset-gated. That told me a
  panic is the failure mechanism (there is no assertion hook), and that
  `OnEnter(Playing)` is a valid "load must have happened by now" checkpoint.
- Guarding both failure modes, not just the obvious one. Asserting the payload
  in the observer only fires if the event fires; a scenario that silently never
  loads would sail past it. The `fired` flag + `OnEnter(Playing)` check is the
  "what newly qualifies / what slips through" discipline from the torpedo retro
  applied to a test: enumerate how the thing under test can fail to happen, not
  just how it can happen wrong.
- Verifying live rather than trusting the green compile. The run confirmed the
  ordering assumption (observer fires before `OnEnter(Playing)`, so the `fired`
  guard does not false-positive) that a type-check alone could not.

## What went wrong

- Wasted a full ~4-minute cycle by giving `timeout 240` to a `cargo run` that
  had to compile from cold first: the build ate the whole window and the run
  never happened (exit 124, no output). The previous retro's own action item
  was "let the build finish, then act" - I half-applied it. The fix that worked
  was to build once (cold) and then re-run against the warm binary with the
  timeout sized for the *run*, not the build.
- Minor: hit the fmt-check-in-a-pipe trap - `... | tail -3; echo FMT=$?`
  reports `tail`'s exit, not `cargo fmt --check`'s, so a real formatting diff
  looked like a pass until I read the diff text. rustfmt wanted the long
  observer signature wrapped; `cargo fmt` fixed it.

## What to improve next time

- For any headless example run, split compile from execute: kick off a plain
  `cargo build --example X --features debug` (or reuse the clippy/build that
  already ran), then time only the run. Never wrap a cold `cargo run` in a
  timeout meant for the run itself.
- When checking a command's status through a pipe, check the right stage:
  `${PIPESTATUS[0]}` for the head, not `$?` after a `tail`. This bit both the
  fmt check here and is worth a standing habit.

## Action items

- [x] `ScenarioLoaded` is now both produced (133011) and consumed/asserted
  (this task) - the 0.4.0 testability loop for scenario init is closed.
- [ ] Optional: extend the same observe-and-assert pattern to the other
  scenario-loading smoke examples (`10_gameplay`, `07_torpedo_guidance`) if a
  future task wants every headless scene to guarantee a non-empty init. Left
  out here to keep this task to one example.
