# Retro: Examples as integration tests

- TASK: 20260525-133005
- BRANCH: test/examples-as-tests
- PR: #36 (open against master, not merged)
- REVIEW ROUNDS: 1 (APPROVE, two benign NITs)

See `tasks/20260525-133005/TASK.md`. The harness built two weeks of work ago paid
off directly here.

## What went well

- Verified the risky mechanism before building on it. The real hazard was
  cargo-in-cargo (running `cargo run --example` inside `cargo test`) deadlocking on
  the target lock - a plausible failure I could not reason away with confidence. So
  I wrote the smallest thing that would exercise it and ran it: the four examples
  completed in 32s, no hang. Only then did I flesh out the assertions. Cheaper than
  designing around a deadlock that turned out not to happen, and cheaper than
  shipping one that did.
- Chose the gate that makes the test a good CI citizen without breaking local runs.
  The examples need a display; gating on `DISPLAY` means CI-with-Xvfb runs them and
  a headless box skips (loudly, passing) instead of failing. That is the difference
  between a test people keep and one they delete.
- Reused the harness's own exit contract as the oracle. The examples already log
  `reached Playing` and `cycle complete, no panic` and exit `AppExit::Success`;
  asserting on those three independent signals is a real check, not a "did it not
  crash" tautology.
- Scoped honestly: only the autopilot-wired examples became tests; the interactive
  demos stayed demos. "Convert the smoke examples" did not mean "test every file".

## What went wrong

- Nothing notable. The one open question (deadlock) was resolved by measurement.

## What to improve next time

- For "run a real binary/app as a test", decide up front how the binary gets built
  with the right features and how you locate/launch it - subprocess `cargo run`
  turned out clean here, but the feature coupling (the example needs `--features
  debug` for the harness) and the display requirement are the parts that make or
  break it. Settling those first avoids a half-working test.

## Action items

- [ ] NIT R1.1: if the ~32s on every display-ful `cargo test` annoys anyone, add an
      opt-in env gate (`NOVA_EXAMPLE_SMOKE=1`) alongside the display check.
- [ ] The pattern generalizes: any future autopilot-wired example is picked up by
      adding its name to `HARNESSED_EXAMPLES`.
