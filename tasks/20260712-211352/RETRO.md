# Retro: examples rework - testable curriculum

- TASK: 20260712-211352
- BRANCH: refactor/examples-testability (landed as 4cbf94c)
- REVIEW ROUNDS: 1 (REQUEST_CHANGES -> APPROVE; 1 MAJOR + 4 MINOR + 2 NIT,
  all addressed and verified)

## What went well

- The task's thesis validated itself immediately: the first outcome
  assertions found two shipped bugs (both weapon ranges fire-dead since the
  weapons safety landed; the torpedo-volley ship drift) plus a
  playtest-relevant radar-pick property - none visible to the old
  reach-Playing smoke.
- Layered diagnostics converged fast once used: the lock-identity probe,
  the blast-position observer and the half-second timeline buckets each
  eliminated a hypothesis class in one run.
- The user checkpoint mechanism worked: the audit table surfaced at a
  decision point and the redirect produced a strictly better structure.
- The fresh-context reviewer ran the entire smoke suite itself and
  re-derived all four bug claims against production code before approving.

## What went wrong

- Round-1 harness work was built BEFORE the user saw the audit table, so
  three bespoke harnesses (02/04/05) were discarded on the redirect. The
  checkpoint came with sunk work attached; the table should have shipped
  with zero implementation behind it.
- Wall-clock script staging raced game state three separate times (the
  cold-press fire latch, the mid-kill nav sweep, the sim-lag arrival). Each
  fix was the same idea - wait on the state the gesture produces - applied
  one incident too late. llvmpipe stutter collapses wall windows into
  single frames, and the fixed-timestep catch-up clamp lets sim time lag
  wall time; a headless script that sleeps on the clock is wrong by
  construction.
- The sweep-then-delete for the renames missed seven doc sites (the
  review's MAJOR): README/AGENTS/architecture and - embarrassingly - the
  bug-pin section this same branch had added earlier in round 1. A sweep
  must cover top-level docs and the branch's own earlier additions.
- Full-suite verification is expensive (~2 min + builds per iteration) and
  the playable needed four suite runs to characterize the in-suite-only
  failure; a "run one example under suite conditions" knob would have cut
  that loop hard.

## What to improve next time

- Audit-then-checkpoint means checkpoint BEFORE building on the audit, even
  when the buildout looks obviously right.
- Author autopilot scripts event-driven from the first line: every stage
  waits on observable game state; wall-clock appears only in backstops.
- Rename sweeps: grep the repo root's *.md and the branch's own diff, not
  just code/docs subtrees.

## Action items

- [x] Follow-ups filed during the task: 20260713-220512 (torpedo-volley
      ship drift).
- [x] Ledger: bumped `sweep-then-delete`, `out-of-context-review-pass`;
      added `event-driven-autopilot-beats`,
      `checkpoint-before-building-on-an-audit`.
- [ ] Consider a HARNESSED_EXAMPLES env filter (run one example under the
      suite harness) if the next example iteration loop hurts again.
