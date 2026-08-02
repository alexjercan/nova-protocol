# Retro: Port the single-shot screenshot driver into nova_autopilot

- TASK: 20260802-183346
- BRANCH: feat/screenshot-port
- REVIEW ROUNDS: 2 (round 1 REQUEST_CHANGES, round 2 APPROVE)

## What went well

The plan's own hazard reasoning did most of the work. It had already decided
that the stand-down test needed its own process because `Plugin::build` reads
process-global env, and that same reasoning, applied one level up during
implementation, caught the bigger version of the hazard: `autopilot.rs`'s
`arm()` sets `NOVA_AUTOPILOT` for the whole lib-test binary, so all four
App-driven screenshot tests were silently exercising an inert plugin. The
accident was a free falsification - every one of those tests fails when the
plugin adds nothing, so none is vacuous.

The overlay hook is the one behavioural fork from the BCS source, and
DECISION.md argued it before the code existed. Review had nothing to add: the
counterfactual (would we build a `DebugEnabled`-shaped type inside the
bevy-only crate?) answers itself.

## What went wrong

The DoD proof command did not run two of the five tests it was the proof for.
`cargo test -p nova_autopilot screenshot` filters on test NAME, so it ran only
the two integration tests whose names begin `screenshot_`, and skipped
`unreached_target_state_error_exits` and
`hide_overlay_hook_runs_before_the_capture`. The close-out's Evidence bullet
then reported that command as "3 lib + 4 integration + 1 stand-down", a count
it never produced.

The failed decision, and why it looked sound: the plan authored that command
while the App-driven tests were still slated for the LIB binary, where they
would have been named `screenshot::tests::*` and the filter would genuinely
have matched all of them. The plan even reasoned explicitly about coverage -
"`--lib` is deliberately absent because it would skip the stand-down test" -
which is exactly the right instinct aimed at exactly the wrong exclusion. When
work then relocated the tests to `tests/screenshot.rs` for the env-collision
reason above, the filter's coverage assumption quietly broke, and the DoD
command was never re-derived against the new layout.

Root cause is not the filter. It is that the coverage argument was made
NEGATIVELY - "here is the one thing this does not skip" - so it could not
survive a structural change. An enumeration ("here is what this command runs")
would have failed loudly the moment the test files moved.

## What to improve next time

- Breadth: ~620 lines across one module and two test binaries. Inherent to a
  port with a real behavioural fork; nothing here was independently landable
  and no split was missed.
- Churn: both blocking findings trace to one plan-time gap. `plan` should treat
  a proof `cmd` that carries a filter, a target selector, or a path as a claim
  needing enumeration, not argument: run it and read back the test names, and
  check `0 filtered out`. `cargo test` reports exactly this and it went unread.
- Any work step that MOVES a test between binaries invalidates every proof
  command that selects tests. Re-derive the DoD `cmd` in the same step, rather
  than trusting it because it is still green - it stayed green precisely
  because it stopped running things.
- Context: no pressure observed. Implementation and review ran in separate
  sessions; this one entered cold at REVIEWING, which is what let the filter
  discrepancy be noticed at all - the number `2 filtered out` reads as a defect
  to someone who did not write the command.

## Action items

- Knowledge submission: enumerate what a proof command RUNS rather than arguing
  what it excludes; `0 filtered out` is the assertion.
- No follow-up tasks. `reel.rs` lands in the same lib binary and hits the same
  env hazard; that is already recorded in TASK.md's close-out reflection and in
  DECISION.md, so the next port inherits it without a tracker entry.
