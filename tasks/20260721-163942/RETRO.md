# Retro: CI fmt gate (20260721-163942)

Outcome: **falsification, no code change.** The task premise was wrong; the
requested change already existed. Closed on evidence, real gap re-filed.

## What went well

- Opened `ci.yaml` and `git log`-ed the fmt step BEFORE sprouting a worktree or
  writing a change. That one check turned a "quick one-liner" into a no-op and
  saved a whole empty implement/review cycle.
- Did not force a change to make the task "feel done". A no-op with zero diff
  is the honest result when the DoD is already green (flow falsification path).
- Separated the false literal ask ("add CI step") from the true underlying goal
  ("drift cannot land"), and routed the latter to its own task (20260722-183022)
  instead of silently dropping it or over-scoping this one.

## What went wrong

- The task should never have been filed as written. Its spawning retro
  (20260721-160842) asserted "CI does not gate cargo fmt --check" without
  opening ci.yaml - CI had gated it for 12 days. This is exactly the failure
  mode that retro's OWN lesson names: `pickaxe-hit-is-not-a-mechanism` /
  "open the diff before writing a history-evidence sentence". A `grep fmt
  ci.yaml` at filing time would have reframed the task immediately.

## What to improve next time

- A task whose DoD is a `grep`/`cmd:` check is cheap to VERIFY before planning:
  run the DoD commands first. If they already pass, the task is done or
  misframed - resolve on evidence, don't implement.
- When a task cites a prior retro's claim as its premise, verify that claim
  against the tree, not the prose. Filed claims decay; a change elsewhere can
  already have satisfied (or invalidated) them.

## Action items

- [x] Filed 20260722-183022: pre-land/pre-commit `cargo fmt --check` guard -
      the real fix (advisory CI step does not stop drift in a local-land flow).
- [ ] LESSONS: this is a fresh instance of `pickaxe-hit-is-not-a-mechanism`
      (a filed *task premise*, not just a Record sentence, went unverified).
      Fold at the next `/lessons` compile rather than editing the ledger mid-task.
