# Retro: Remove the base demo scenario

- TASK: 20260716-155816
- BRANCH: refactor/drop-base-demo-scenario (landed 564ff12d)
- REVIEW ROUNDS: 2

## What went well

- The audit-first plan (141620's FINDINGS.md) made the work nearly
  mechanical: every edit site was named with file:line before the
  worktree existed, and none of them was wrong.
- Running the touched integration test caught the third base-"demo"
  assertion (demo_scenario.rs:543) that the planning sweep had missed -
  the verify step did its job as the safety net.
- The reviewer pass independently re-derived two claims instead of
  trusting the summary: gauntlet's `dependencies: ["base", "demo"]`
  names the demo MOD (a false-blocker avoided), and the picker default
  was already Broadside (no behavior surprise).

## What went wrong

- The planning sweep listed two of the file's three "demo" assertions.
  Root cause: the grep output was piped through `head` for readability
  during planning, and the truncated list was then treated as complete
  during work. Cost: one failed test run.
- The parent audit undercounted base scenarios (missed Broadside)
  because it read the WORKING copy of base.bundle.ron in the shared
  checkout while a parallel session had it in flux. Reads race too, not
  just commits.
- The CHANGELOG line was missed until review (R1.1): the change was
  mentally filed as "refactor" although deleting a Scenarios picker row
  is player-visible; the docs-sync map explicitly routes scenario
  changes to CHANGELOG.

## What to improve next time

- Sweeps that feed a work checklist are evidence, not display: never
  head-truncate them - dump to full output (or a file) and count the
  matches into the plan.
- When auditing in the shared main checkout, read facts via
  `git show HEAD:<path>` instead of the working file; parallel sessions
  own the working tree.
- Before calling a change "refactor", ask what a PLAYER loses or gains;
  any yes routes through the keeping-docs-in-sync map.

## Action items

- [x] FINDINGS.md correction note added (Broadside undercount).
- [x] Ledger: bumped keep-docs-in-sync-with-code (x2), added
      truncated-sweep-is-not-a-sweep (x1) and
      shared-checkout-reads-race (x1).
- [x] R1.2 routed: bundle-list-equals-file-map assertion recorded in
      tasks/20260716-155823/TASK.md (rides that task).
