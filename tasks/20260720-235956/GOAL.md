# Goal: resolve nova pending-promotion ledger backlog and triage the OPEN task backlog

- DATE: 20260720
- UMBRELLA TASK: 20260720-235956
- LANDING SCOPE: squash-merge each task to local master; do NOT push (user's call).

## Goal

nova-protocol has 6 x3+ pending-promotion lessons awaiting disposition and a
backlog of OPEN tasks (some idle since the initial seeding). This run resolves
the ledger (each x3+ lesson promoted with a concrete home - the flow skills
where already institutionalized, or nova's own AGENTS.md - or retired) and does
a triage pass over the OPEN tasks (each confirmed keep-with-current-tag, or
surfaced for close/defer). Triage decisions that need the user's product call
are surfaced, not made unilaterally.

## Done means

1. Every x3+ pending lesson is annotated promoted or retired; ledger lints clean (cmd: `tatr check --ledger LESSONS.md 2>&1 | grep -c promotion-stalled` -> 0).
2. Lessons promoted into nova's AGENTS.md are actually present there (manual: reviewer spot-checks).
3. Every OPEN task has a current, intentional scheduling tag/priority, or is surfaced for a close/defer decision recorded in GOAL.md (manual: reviewer scans the OPEN list against the triage record).

Overall: `tatr check` and `tatr check --ledger LESSONS.md` clean (modulo the goal umbrella); no code touched.

## Tasks

- [ ] 20260720-220051 (p0) lessons: resolve the 6 pending promotions
- [ ] 20260720-220104 (p0) backlog triage: disposition the OPEN tasks

## Manual acceptance (batched for the user at Finish)

Accumulates `manual:` DoD items as tasks land; presented at Finish.
