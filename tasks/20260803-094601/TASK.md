# Give the example subprocess tests their own timeout

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: backlog,testing,autopilot,wontdo

## Story

`crates/nova_autopilot/tests/autopilot_example.rs` and `tests/examples_smoke.rs`
both spawn an example as a subprocess and block on `Command::output` with no
timeout of their own. `NOVA_AUTOPILOT_DEADLINE` bounds a run that reaches the
app's schedule, but a stall before the first frame (winit/GPU init on a bad
runner) hangs until the CI job timeout kills the job with no log to read.

Raised as `20260802-183352` R1.1, deferred as consistency rather than a new
hazard: fix it in both places or neither.

## Steps

- [ ] Give both subprocess tests a wall-clock timeout that kills the child and
      fails with the partial output captured so far.

## Definition of Done

- Both tests fail with captured output rather than hanging when the child never
  reaches its first frame.
  (test: `TODO: name the timeout test once the shape is planned`)

## Closed 2026-08-12: wontdo, overtaken

`tests/examples_smoke.rs` was deleted; its replacement, the CI
`probe run --all` gate, supervises each child with its own `--timeout`
(`nova_probe_cli/src/native/supervise.rs`). Residual risk is only a
pre-first-frame hang in `autopilot_example.rs`, bounded by the 60-min CI
job timeout. Not worth a task.
