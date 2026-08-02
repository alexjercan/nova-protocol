# Decisions - 20260802-183352

## The example runs `DefaultPlugins`, not `MinimalPlugins`

A `MinimalPlugins` example would need no display and would run in the plain CI
test step - but the crate already has App-driven `MinimalPlugins` coverage in
`src/autopilot.rs` and `tests/`. What is NOT covered is the real thing: a
windowed app whose state machine, input collection and exit path are Bevy's own.
That is what `nova_probe` will reuse, and what the "runs headless to a clean
exit" DoD means. The cost is a display requirement, which the DoD already
accounts for by demanding a loud skip rather than a failure.

## The test spawns the example as a subprocess

Same shape as `tests/examples_smoke.rs`: `env!("CARGO")` + `cargo run --example`,
assert on exit status and stderr lines. In-process is not an option - the thing
under test is the process exit code and the log output a supervisor reads.

## CI gets one extra Bevy variant

The root `debug` feature turns on `bevy/track_location`; the nested
`cargo run -p nova_autopilot` resolves without it, so CI compiles a second Bevy
variant once and caches it. The alternative - never running this test in CI -
would leave the crate's only end-to-end proof permanently skipped. Accepted, and
the CI step is written as `-p nova_autopilot` with no `--features` so the test
binary and its nested run share one graph (one extra variant, not two).
