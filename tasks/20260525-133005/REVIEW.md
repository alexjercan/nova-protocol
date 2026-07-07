# Review: Convert examples into integration tests

- TASK: 20260525-133005
- BRANCH: test/examples-as-tests

## Round 1

- VERDICT: APPROVE

Turns the examples' built-in autopilot harness into a `cargo test` target
(`tests/examples_smoke.rs`): each autopilot-wired example is run headless under
`BCS_AUTOPILOT` and asserted to reach Playing and exit without panic. This is
exactly what the harness (task `20260707-100002`) was built to enable, now
automated.

Verified independently in the worktree:

- Under Xvfb: `cargo test --test examples_smoke` runs all four examples
  (`03_scenario`, `06_torpedo_range`, `07_torpedo_guidance`, `08_turret_range`) and
  passes in ~32s.
- With no `DISPLAY`: the test skips loudly and passes (0.00s) - a plain `cargo test`
  on a headless box does not fail.
- `cargo clippy --test examples_smoke`: clean.

Good calls:
- The cargo-in-cargo deadlock risk (running `cargo run` inside `cargo test`) was
  the real hazard here; it was checked empirically (the run completes, no hang)
  before committing to the approach, and the examples run sequentially rather than
  piling up concurrent builds/windows.
- The `DISPLAY` gate is the right level for windowed apps: CI sets up Xvfb and the
  test runs there; headless boxes skip instead of failing. The skip is loud, not
  silent.
- Asserts on both the reached-Playing line and the clean-exit line, plus process
  success - three independent signals, and it prints a stderr tail on failure.
- Correctly leaves the non-harnessed demo/tuning examples (01/02/04/05/07b) as
  examples, matching "keep small examples where useful".

No BLOCKER/MAJOR. Two NITs.

- [ ] R1.1 (NIT) The test runs on any display-ful `cargo test` (~32s), so a dev on a
  workstation pays that cost on every full test run. If that becomes annoying, gate
  it behind an additional opt-in env var (e.g. `NOVA_EXAMPLE_SMOKE=1`) that CI sets
  alongside the display. Left as DISPLAY-only so it runs in CI without extra config.
  - Response:
- [ ] R1.2 (NIT) The pass depends on the autopilot window (`NOVA_AUTOPILOT_SECS`, 6s)
  being long enough for assets to load and reach Playing; on a very slow/cold CI box
  that could be tight. Not observed here, and the reached-Playing assertion fails
  loudly (rather than flakily passing) if it ever is - so it degrades to a clear
  failure, not a silent one.
  - Response:
