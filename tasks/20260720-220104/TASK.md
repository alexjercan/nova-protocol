# backlog triage: disposition 30 OPEN tasks (close/defer/keep)

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog,chore

## Story

As the maintainer, I want a triage pass over the 30 OPEN tasks, so that each is
explicitly either queued-for-a-release or closed/deferred. Some have been idle
since the initial seeding (e.g. the bevy_common_systems doc task), and there is
currently no signal distinguishing genuinely stalled work from legitimately
queued work.

## Steps

- [ ] List all OPEN tasks (`tatr ls -f '(:status eq OPEN)' --sort priority`).
- [ ] For each: confirm its release tag + priority is still right, OR move to backlog, OR close/defer with a reason recorded in the task.
- [ ] Flag any that are superseded and should become CLOSED archive stubs.

## Definition of Done

- Every OPEN task has a current, intentional scheduling tag and priority, or is closed/deferred with a recorded reason (manual: reviewer scans the OPEN list).

## Notes

- Recurring triage; consider making it a periodic pass.
