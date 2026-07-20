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

- [x] 20260720-220051 (p0) lessons: resolve the 6 pending promotions
      landed 2669d332; 1 review round (APPROVE, no findings). 5 folded into
      AGENTS.md Conventions, out-of-context-review-pass -> flow round-1; Pending
      promotions empty. (Master moved mid-cycle; merged + re-verified before land.)
- [x] 20260720-220104 (p0) backlog triage: disposition the OPEN tasks
      landed 37d3e3a3; in-session review (out-of-context reviewer stopped by user).
      24 OPEN tasks all intentionally tagged; no unilateral closes; candidates
      surfaced in TRIAGE.md.

## Manual acceptance (batched for the user at Finish)

- (pending) 20260720-220051: skim the AGENTS.md "Promoted ledger lessons" block
  and the LESSONS.md Promoted annotations.
- (pending) 20260720-220104: rule on the 3 idle May-25 doc tasks (20260525-133030
  doc nova_gameplay, 133032 inline plugin docs, 133031 doc bevy_common_systems -
  the last is a cross-repo/likely-redundant close candidate). See TRIAGE.md.
