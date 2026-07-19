# Retro: probe shared Xvfb (field bug)

- TASK: 20260719-224011
- BRANCH: fix/probe-shared-xvfb (squash-landed as 41bee2a9)
- REVIEW ROUNDS: 1 (APPROVE; 1 NIT)

## What went well

- Field evidence beat review reasoning: T1's review had explicitly
  blessed per-run Xvfb spawn as "zero new lifecycle risk", and the first
  real user fleet run falsified it in one sweep. The task recorded the
  falsification against the original deviation instead of quietly
  fixing - the paper trail now explains WHY the spike's shared-server
  design is load-bearing.
- The failing e2e was treated as data, not friction: attempt 1 dying on
  the user's live :84 server was the OTHER face of the same weakness
  (cross-process pid%10 collision), so the fix grew from "share the
  server" to "share the server AND walk the band" - one task, both
  mechanisms removed, each reproduced before being fixed.
- Continue-on-failure did its job in the field: the user's sweep lost
  one row, not the run, and the error message reached me verbatim.

## What went wrong

- The original deviation shipped on a review argument ("~1s cost, zero
  risk") without asking what RESOURCE the per-run spawn contended on.
  Display numbers are allocation, and T1's own retro had ALREADY written
  the lesson ("resource allocation is part of the tested surface" -
  20260719-112317); it was not consulted when the deviation was made.
- Two e2e cycles instead of one because the first ran before thinking
  through what ELSE holds displays on a shared dev box (the user's own
  sweep - announced in this very session).

## What to improve next time

- When deviating from a spike's design, name the resource the deviation
  touches and grep the retro ledger for it first - the 112317 retro
  would have flagged this in a minute.

## Action items

- [x] User unblocked: pull master, repair the one ERROR row
      (`probe run perf_baseline ... --out probe-runs/perf_baseline`,
      then `probe report probe-runs`).
