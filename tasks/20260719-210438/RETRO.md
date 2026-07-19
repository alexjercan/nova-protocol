# Retro: probe multi-run + aggregate (probe-all T1)

- TASK: 20260719-210438
- BRANCH: feature/probe-all (squash-landed as 6d004463)
- REVIEW ROUNDS: 1 (APPROVE; 2 NITs + 1 accepted risk)

## What went well

- The spike -> adjudication -> task pipeline meant zero design churn at
  implementation time: every open question (bare-run behavior, --all
  semantics, exclusion policy) had a user decision BEFORE the first edit,
  and the task body was effectively the implementation checklist.
- Parser unification paid off immediately: moving the catalog parse into
  nova_probe and pointing the drift test at it deleted the second copy
  and put the fail-closed rules (name/category collision, discovery-off)
  where BOTH consumers inherit them - the drift test now guards probe's
  spec resolution for free.
- The prior tasks' layering held: run() needed ZERO signature changes for
  multi-run - the driver loops it, reads each run's checks.json (probe
  consuming its own agent surface), and aggregates. The hardening task's
  manifest/checks discipline is what made rows buildable from artifacts
  alone.
- The e2e sequence was chosen to make the honesty property VISIBLE, not
  just asserted: the list run put a wired 5/6 row next to an unwired 2/6
  row in one table - T2's motivation shipped as evidence inside T1's
  close-out.
- Shell discipline from this session's own retro held: every e2e wrote to
  a log file with a bare exit code; no pipes ate anything; job control
  stayed in dedicated backgrounded commands.

## What went wrong

- One landing-protocol stumble caught by the protocol itself: the
  cleanup-task commit (filed mid-implementation on the user's request)
  moved master ahead of the sprout point, and the pre-land ancestry check
  failed exactly as designed - sync merge in the worktree, then a clean
  squash. Filing tasks on master DURING a worktree cycle makes this the
  expected path, not an anomaly; the check is why it cost one merge
  commit instead of a bad land.
- A closure-mutability lint (`let mut flush` on a non-capturing closure)
  survived until the first live run's warning output - `cargo test`'s
  earlier pass had scrolled it by. Warnings deserve a grep in the
  verify step, not just eyeballs on a tail.

## What to improve next time

- When a mid-cycle user request files tasks on master, note it in the
  active task's worktree immediately - the land step then EXPECTS the
  sync merge instead of being surprised by the ancestry check.

## Action items

- [x] T2 (20260719-210443) is next: fleet wiring turns the 2/6 rows into
      5/6; its exit gate is the first real `probe run --all`.
- [ ] NIT carried: re-running a multi spec into the same base replaces
      the aggregate (spec line says what it covers) - revisit only if it
      confuses in practice.
