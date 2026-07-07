# Convert examples into integration tests

- STATUS: CLOSED
- PRIORITY: 85
- TAGS: v0.4.0, test

Convert smoke-test examples into proper integration tests; keep small examples where useful. Legacy #96.

## Resolution

Added `tests/examples_smoke.rs`, a `cargo test` integration test that turns the
examples' built-in autopilot harness into an automated regression check. It runs
each autopilot-harnessed example (`03_scenario`, `06_torpedo_range`,
`07_torpedo_guidance`, `08_turret_range`) headless via `BCS_AUTOPILOT=1` and asserts
each one exits `AppExit::Success` and logs `nova harness: reached Playing` and
`autopilot: cycle complete, no panic`.

Design notes:
- Runs the examples as a subprocess (`cargo run --example ... --features debug`);
  verified this does NOT deadlock the outer `cargo test` on the target lock. Runs
  the four sequentially.
- The examples open a real window, so a display is required; the test skips loudly
  (a passing no-op) when `DISPLAY` is unset, so a plain `cargo test` on a headless
  box does not fail. Under a virtual display (CI Xvfb) it runs and validates.
- The small demo/tuning examples that have no autopilot harness (01, 02, 04, 05,
  07b) are kept as-is - they are interactive demos, not smoke-testable, matching
  "keep small examples where useful".

Verified: under Xvfb all four pass (~32s); with no DISPLAY the test skips and passes;
clippy clean.
