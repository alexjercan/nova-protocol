# Decision: The runnable proof is a windowed app, run as a subprocess

- DATE: 20260803-001040
- STATUS: ACCEPTED
- TASK: 20260802-183352
- TAGS: decision, autopilot, testing, ci

## Context

`nova_autopilot` needs a runnable example that proves the crate stands alone,
plus an automated check on it. The crate already has App-driven `MinimalPlugins`
coverage in `src/autopilot.rs` and `tests/`, so the open questions were what the
example must be to add anything, and how CI should exercise it.

## Decision

The example (`examples/driven_app.rs`) runs `DefaultPlugins`, and
`tests/autopilot_example.rs` spawns it as a subprocess via `env!("CARGO")`,
asserting on exit status and stderr lines - the same shape as the root
`tests/examples_smoke.rs`. CI runs it as `-p nova_autopilot` with no
`--features`.

## Alternatives considered

- `MinimalPlugins`: would need no display and would ride the plain CI test step,
  but duplicates existing lib coverage. What is NOT covered is the real thing: a
  windowed app whose state machine, input collection and exit path are Bevy's
  own. That is what the probe run-harness reuses, and what the "runs headless to
  a clean exit" DoD means.
- Driving the example in-process: not an option - the subjects under test are
  the process exit code and the log output a supervisor reads.
- Never running the test in CI: would leave the crate's only end-to-end proof
  permanently skipped.

## Consequences

- A display is required. The DoD accounts for it: no `DISPLAY`/`WAYLAND_DISPLAY`
  means a loud skip, not a failure, so a bare `cargo test` on a headless box
  still passes.
- CI compiles one extra Bevy variant, once, then caches it. The root `debug`
  feature turns on `bevy/track_location`; the nested `cargo run -p
  nova_autopilot` resolves without it. Writing the CI step as `-p nova_autopilot`
  with no `--features` keeps the test binary and its nested run on one graph -
  one extra variant, not two.
