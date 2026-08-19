# probe measures a scenario directly, no example required

- STATUS: OPEN
- PRIORITY: 60
- TAGS: v0.11.0,harness,performance,scenario

Epic: `20260818-220812`. Owner, 2026-08-19: "`scene_baseline` should be a
FEATURE of `nova_probe_cli`... like `probe` subcommand should allow you to load
a scenario from a .ron file basically. It's like I want to probe a scenario,
well use `probe` - why would you use an example for it."

## The point

Measuring a scenario should not require an example. `probe` already owns
running, measuring and reporting; a scenario is data. Point the tool at the
data.

```
probe scenario <path-to.ron>      # or an id from GameScenarios
```

It loads the scenario, runs it, and reports the same frame data any probe run
reports. No `[[example]]` entry, no Rust file per scenario, no example name
baked into library code.

## What this replaces

`examples/screenshots/scene_baseline.rs`, deleted with the screenshot split. It
took `--scenario <id>` or `NOVA_PERF_SCENARIO` and measured whatever it loaded,
which is exactly this subcommand wearing an example costume.

It also removes the reason `nova_probe_cli` ever wanted to know an example's
NAME - see the deleted `budgets.rs`, which hardcoded example names and frame
numbers into library code. **Nothing in `nova_probe_cli` may name a real
example.** Test fixtures use obviously fake names.

## Design notes

- **Report, do not judge.** The probe measures and presents; a human reads the
  HTML report and decides whether the frame rate is acceptable. No pass/fail on
  a frame-time number, no per-scenario budget table. This is settled - it is
  why `budgets.rs` was deleted.
- Loading from a PATH matters as much as loading by id: a modder or a
  contributor should be able to measure a scenario that is not in the shipped
  catalog, without adding it to one.
- `editor_sandbox` is the awkward case and worth handling deliberately. It is
  registered into `GameScenarios` at editor-Play time, AFTER the `--scenario`
  membership check runs, so it cannot be loaded by id at all today
  (`crates/nova_editor/src/scenario.rs:203-212` against
  `crates/nova_core/src/lib.rs:257-266`). Either the check learns about late
  registration, or the sandbox registers up front like everything else. That
  hole is what hid a 2 FPS bug for a whole cycle.
- `crates/nova_perf_web/src/main.rs` mirrors what `scene_baseline` did for the
  web perf page. Decide whether it becomes a caller of this or stays separate.

## Done when

- `probe` can measure any shipped scenario by id, and any scenario file by
  path, with no example involved.
- The report presents frame data clearly enough that a human can judge it in
  one look.
- No real example name appears anywhere in `nova_probe_cli` outside test
  fixtures, and those use fake names.
