# Review: Add a runnable nova_autopilot example with a headless integration test

- TASK: 20260802-183352
- BRANCH: feat/autopilot-driven-example

## Round 1

- REVIEWER: primary, in-context. EXCEPTION to the out-of-context default: this
  session runs under an operator policy that forbids spawning subagents unless
  the user asks for one, and no fresh `/flow` session started at REVIEWING. To
  compensate, every check was re-run from the worktree and the two load-bearing
  claims (the `OnEnter(Done)` guard actually fails the process; the example is
  a real build target where `--examples` used to be a no-op) were re-derived by
  mutation rather than accepted from the implementation notes.
- VERDICT: APPROVE

Checks re-run in `/home/alex/.cache/sprouts/nova-protocol/feat/autopilot-driven-example`:

- `cargo check -p nova_autopilot --examples`: compiles `nova_autopilot` (on
  base this printed "no targets matched; this is a no-op"), so the DoD command
  is no longer green-by-vacuum.
- `cargo test -p nova_autopilot` with `-u DISPLAY -u WAYLAND_DISPLAY`: 27 tests
  pass across six binaries, and `autopilot_example_completes_a_cycle` prints
  its SKIP line and passes - the "skipped, not failed" DoD.
- `DISPLAY=:99 cargo test -p nova_autopilot --test autopilot_example`: 1 passed
  (5.65s).
- Real run under `Xvfb :99`: exit 0, with `autopilot: -> Flying (t=0.5s)`,
  `driven_app: thrust moved the cube`, `autopilot: -> Done (t=2.5s)`,
  `autopilot: cycle complete, no panic (t=3.0s)` and
  `harness completion: all collectors done, exiting`. This is the `manual:`
  DoD item, performed.
- No-coupling grep: exit 0 (the example names only `nova_autopilot`).
- `cargo fmt --check`, `tatr check 20260802-183352`: clean.

Mutation: short-circuiting the input closure (never pressing Space) made the
same run panic in `assert_the_cube_moved` and exit 101, with no
`driven_app: thrust moved the cube` line. So the in-example guard and the
test's fourth assertion each independently fail a run that stops being driven -
the DoD's whole point, and not something a green-by-construction example could
claim.

Deviations, both accepted:

- Step 5 asked for the command inside the existing "Examples smoke test" step;
  the branch adds its own "Autopilot example" step
  (`.github/workflows/ci.yaml:118`). Same command, same one-extra-variant cost
  (`-p nova_autopilot`, no `--features`, per the Notes), and a failure is
  attributed to the crate rather than to the game's example fleet.
- `DECISION.md` was rewritten from the free-form planning shape into the
  repository record schema; `tatr check` rejected the original. Content is
  preserved.

- [ ] R1.1 (NIT) crates/nova_autopilot/tests/autopilot_example.rs:28 - the test
  has no timeout of its own. `NOVA_AUTOPILOT_DEADLINE=30` bounds a run that
  reaches the app's schedule, but a stall before the first frame (winit/GPU
  init on a bad runner) blocks `Command::output` until the CI job timeout kills
  it with no log. Prior art (`tests/examples_smoke.rs`) has the same shape, so
  this is consistency, not a new hazard; fix it in both or neither.

- [ ] R1.2 (NIT) crates/nova_autopilot/src/lib.rs:29 - the crate docs do not
  point at `examples/driven_app.rs`, so the runnable proof is discoverable only
  from the test. The crate docs/prelude task (`20260802-183355`) owns that
  surface and is the cheaper place to add the pointer.

No BLOCKER or MAJOR findings. Pending user checks: none - the single `manual:`
DoD item was performed and its log lines are quoted above.
