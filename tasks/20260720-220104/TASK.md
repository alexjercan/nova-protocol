# backlog triage: disposition 30 OPEN tasks (close/defer/keep)

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: backlog, chore

## Story

As the maintainer, I want a triage pass over the 30 OPEN tasks, so that each is
explicitly either queued-for-a-release or closed/deferred. Some have been idle
since the initial seeding (e.g. the bevy_common_systems doc task), and there is
currently no signal distinguishing genuinely stalled work from legitimately
queued work.

## Steps

- [x] Listed all OPEN tasks (24; see TRIAGE.md) (`tatr ls -f '(:status eq OPEN)' --sort priority`).
- [x] Confirmed: all 24 carry an intentional scheduling tag + priority (9 v0.8.0 scheduled, 15 backlog deferred); none untagged. No unilateral close/defer of product work; candidates surfaced to the user.
- [x] Supersession checks done (screen-indicator not yet in bcs; ship-prototype folds-note is not a dup) - none superseded. Stale candidates (3 May-25 doc tasks) surfaced in TRIAGE.md.

## Definition of Done

- Every OPEN task has an intentional scheduling tag + priority (cmd: `tatr ls -f '(:status eq OPEN)'` shows every task tagged v0.8.0 or backlog); the triage assessment and close/defer candidates are recorded in TRIAGE.md (manual: reviewer reads it).

## Notes

- Recurring triage; consider making it a periodic pass.
