# Decision: The fleet run is a check, not a stored artifact

- DATE: 20260805-084924
- STATUS: ACCEPTED
- TASK: 20260804-095507
- TAGS: examples, testing, perf, probe

## Context

The task was scoped as "run the fleet as CI will AND record the sprint's
evidence" - four jobs in one: verify the CI smoke gate, answer whether the
gate still fits `timeout-minutes: 60`, run `probe run --all --fps`, and commit
the report under `tasks/20260804-095507/probe-results/` with a frame-time
comparison against the v0.7.0 baseline in `tasks/20260716-123551/perf-results/`.

Three of those four turned out not to be work:

- The owner already ran
  `xvfb-run --auto-servernum cargo test -p nova-protocol --test examples_smoke
  --features debug` and it passed.
- The CI-budget question is answered by CI itself on the next push. A local
  estimate costs an hour and binds nothing.
- The baseline comparison has no shared series. `broadside-*` retires with its
  example (`20260804-093910`); `many_bodies` / `many_sections` /
  `many_projectiles` are new by construction. The retained v0.7.0 numbers are
  other scenarios.

The epic's acceptance criterion (`tasks/20260802-115955/TASK.md:45`) is
"the owner accepts the generated probe report ... (manual: inspect generated
`report.html`)". It asks for a report to be INSPECTED, not committed.

## Decision

This task is one command and the reading of its output.

- Run `nix develop --command cargo run -p nova_probe -- run --all --fps`
  under Xvfb, on `master`, in place - no sprout worktree.
- The report lands in probe's default `probe-runs/`, which `.gitignore`
  already excludes. Nothing is committed: no `probe-results/` folder, no
  `--baseline`, no custom output path.
- The verdict is read from `index.json` / `checks.json` (verdict together with
  `measured`, never alone), and the run-policy contract is confirmed against
  it: `screenshots/` excluded from `--all`, `stress/` the only category with
  frame-time passes, everything else correctness-only.
- Numbers worth keeping get written into this task's Notes and its retro,
  because nothing else will outlive `target/`.
- Anything the full-fleet run surfaces that per-category runs did not is
  FILED as its own task. Only one-line corrections are fixed here.

## Alternatives considered

- **Commit the report under `tasks/20260804-095507/probe-results/`**, on the
  `tasks/20260716-123551/perf-results/` precedent. Rejected by the owner. The
  precedent existed to make a release-over-release COMPARISON possible; with
  no comparable series left, a committed folder is a large generated blob that
  nothing reads.
- **Answer the CI-budget question locally** (time the sequential smoke run
  cold against `timeout-minutes: 60`). Rejected: CI measures it for free on
  push, and a local llvmpipe timing does not transfer to the runner anyway.
- **Split into `--all` plus `stress --fps`** if the single invocation proves
  unwieldy. Kept as an operational fallback, not the plan: the epic's Done
  Means names the single command, and two invocations discharge it equally if
  the one-shot run does not finish.

## Consequences

- The task shrinks from seven Steps to essentially one plus its follow-through.
  It stops being the place leftover sprint work hides.
- No artifact survives in the repo. The frame-time numbers exist only where
  this task writes them down, so writing them down is not optional.
- The three `stress/` series get no recorded baseline for the next release
  unless a later task chooses to store one. That is accepted, not overlooked.
- If CI blows its 60-minute budget after this sprint, it surfaces as a red
  build rather than a pre-filed task. Accepted by the owner.
