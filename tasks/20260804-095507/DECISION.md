# Decision: The fleet run is a check, not a stored artifact

- DATE: 20260805-091032
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

- The CI-budget question is answered by CI itself on the next push. A local
  estimate costs an hour and binds nothing.
- The baseline comparison has no shared series. `broadside-*` retired with its
  example (`20260804-093910`); `many_bodies` / `many_sections` /
  `many_projectiles` are new by construction. The retained v0.7.0 numbers are
  other scenarios.
- The smoke gate is a separate run from the probe report, and the owner drives
  it directly.

The epic's acceptance criterion (`tasks/20260802-115955/TASK.md:45`) is "the
owner accepts the generated probe report ... (manual: inspect generated
`report.html`)". It asks for a report to be INSPECTED, not committed.

## Decision

This task is one command and the reading of its output.

- Run `nix develop --command cargo run -p nova_probe -- run --all --fps`
  under Xvfb, on `master`, in place - no sprout worktree.
- The report lands in probe's default `probe-runs/`, which `.gitignore`
  already excludes (`.gitignore:252`). Nothing is committed: no
  `probe-results/` folder, no `--baseline`, no custom output path.
- The verdict is read from `checks.json` (verdict together with `measured`,
  never alone), and the run-policy contract is confirmed against it:
  `screenshots/` excluded from `--all`, `stress/` the only category with
  frame-time passes, everything else correctness-only.
- Numbers worth keeping get written into this task's NOTES, because nothing
  else will outlive `target/`.
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
- **Chase the `click_named` smoke flake to a fix inside this task.**
  Rejected by the owner, twice: once when it looked unreproducible, and again
  after it WAS reproduced on `editor` during the workspace suite run. The fix
  touches every `click_named` call site and its shape is a real design choice,
  and this task is the sprint's last - it is where leftover work hides.
  Filed as `20260805-091151` and moved into the v0.10.0 sprint (priority 84,
  under the epic) rather than left in backlog, because it gates CI.
- **Split into `--all` plus `stress --fps`** if the single invocation proved
  unwieldy. Not needed: the one-shot run completed in ~9 minutes.

## Consequences

- The task shrinks from seven Steps to essentially one plus its follow-through.
- No artifact survives in the repo. The frame-time numbers exist only in this
  task's NOTES, so writing them down was not optional.
- The three `stress/` series ARE the baseline for the next release; there is
  no earlier comparable data, and `fps_within_baseline` reads `SKIPPED` on
  every example as a result. A later task may choose to store one.
- If CI blows its 60-minute budget after this sprint, it surfaces as a red
  build rather than a pre-filed task. Accepted by the owner.
- `many_projectiles`' frame spikes (p99 224 ms, 4.5 fps 1% low) are recorded
  and filed, not fixed here. Nothing gates on frame time, so this class of
  regression stays invisible to CI until a baseline is stored.
- The run was done in place on `master` while the owner kept committing, so
  the evidence spans two commits. Acceptable for a scripts-only intervening
  change; it would not be for a gameplay one.
