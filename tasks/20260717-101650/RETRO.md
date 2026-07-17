# Retro: Thruster loop as a section sound

- TASK: 20260717-101650
- BRANCH: task-20260717-101650-thruster-loop-sound (squash-landed f8fc2686)
- REVIEW ROUNDS: 1 (APPROVE)

## What went well

- The "risky" cycle came in clean: naming the preserved semantics in the plan
  (per-ship attribution, player exemption, smoothing rate, pause gating) meant
  the rework was checked against an explicit list, and the seven pre-existing
  hum tests pinned each one mechanically.
- The prose-grep lesson (101633 retro) caught the stale "one looping audio
  entity" module header BEFORE review - first cycle where the recurring
  stale-prose finding did not reach the reviewer.
- The cycle-4 lesson paid immediately at merge time: a parallel session landed
  ledger_ch2b mid-cycle, and the sweep habit caught its 8 sound-less asteroids
  as merge integration (named the source, gated, one commit) instead of
  shipping silent rocks.

## What went wrong

- Nothing structural. The torpedo's internal thruster needed a judgment call
  (direct runtime path vs a new config field); scoping it as a noted future
  lift rather than widening the branch was right, but it does leave one
  hardcoded path in gameplay code - the kind of exception that wants a
  follow-up if it multiplies.

## What to improve next time

- Behavior-preserving refactors of continuous systems go well when the plan
  enumerates the invariants first; keep doing exactly that for the final
  family task (salvage) and the WorldSfx deletion.

## Action items

- None; existing lessons covered this cycle (and two of them visibly paid).
