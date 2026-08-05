# Review: menu_scenarios is killed by a signal in the ui smoke, roughly 1 run in 5

- TASK: 20260805-111329
- BRANCH: fix/sync-pipeline-compilation

## Round 1

- REVIEWER: out-of-context
- VERDICT: APPROVE

- [ ] R1.1 (NIT) web/src/wiki/dev/architecture.md:15 - the crate table still
  reads "window/log/asset setup, status UI" while the same phrase two rows
  down at line 84, and project-tour.md:73, were both updated to
  "window/log/asset/render setup"; change line 15 to match.

Verified in the recording pass (re-derived independently of the round-1
reviewer):

- The diff is 3 code lines plus one import in `crates/nova_core/src/lib.rs`,
  2 doc lines, and task records. No unrequested scope.
- Steps 1 and 2 match their literal text: `render_plugin()` sits beside
  `window_plugin()`/`log_plugin()` (lib.rs:213-225), returns
  `RenderPlugin { synchronous_pipeline_compilation: true, ..default() }`,
  carries the NOTE naming the teardown race and task ID, and is the fourth
  `.set()` in `AppBuilder::new` (lib.rs:96-101). `RenderPlugin` is imported
  explicitly from `bevy::render` with the "not in prelude" note.
- `nix develop --command cargo fmt --check` rc 0;
  `cargo check --workspace --features debug` rc 0, only the pre-existing
  `proc-macro-error2` future-incompat note.
- DoD grep proof hits lib.rs:223. DoD ui-smoke proof passed for the reviewer
  (`ok. 1 passed`, 46.7 s, rc 0).
- R1.1 re-derived from the diff: the two updated doc lines are
  architecture.md:84 and project-tour.md:73; architecture.md:15 carries the
  same stale phrase and was not touched.
- `NOTES.md` after-numbers section exists and is internally consistent with
  the baseline the task recorded (2/20 -> 0/60, kernel segfaults 0, median
  8.0 s -> 7.6 s).

Process signal: the 30-run loop proof costs ~4 minutes per pass, so neither
pass re-ran it; the round-1 reviewer substituted a 3-run plus kernel-log spot
check (all rc 0, 0 segfault records). The recorded 60/60 stands on the
implementer's run, corroborated by the kernel-log reading.

Process signal: no automated test guards the flag - a grep is the only
regression barrier. Reasonable, since a plugin field is not observable after
`add_plugins`, so not filed as a finding.

Pending user checks:

- `manual:` read `NOTES.md` and confirm the after-numbers (pass count, kernel
  segfault count, median run time) against the recorded baseline.

Inspection commands:

```bash
cd "$(sprout show fix/sync-pipeline-compilation)"
git diff master...HEAD
nix develop --command bash -c 'cargo fmt --check && cargo check --workspace --features debug'
nix develop --command env DISPLAY=:99 cargo test --test examples_smoke ui
```
