# Retro: clock-derive the orbit-hold and lock-refire 5s windows

- TASK: 20260717-151537
- BRANCH: refactor/scenario-clock-timers
- REVIEW ROUNDS: 1 (APPROVE)

Process notes only; the what/why/evidence is in TASK.md + NOTES.md, the
findings in REVIEW.md.

## What went well

- Verified the load-bearing premise against the actual code instead of trusting
  the comment. The old trackers' pause-correctness rested on "Time<Virtual> is
  frozen while paused"; rather than assume it, I grepped and found
  `pause_clocks` -> `virtual_time.pause()` (nova_menu/src/lib.rs:322), which
  confirmed true parity in production AND surfaced the real win (dropping a
  latent nova_scenario -> nova_menu coupling). Matches the ledger's
  `measure-before-writing` / `cited-finding-reread-not-recalled`.
- Doubled the review against the shared-session blind spot: an out-of-context
  reviewer agent independently re-derived both load-bearing claims (frame-by-frame
  parity + `.after` ordering semantics) while I re-derived them here. Both
  converged; the only finding was a cosmetic f32->f64 nit. Cheap insurance on a
  behavior-parity refactor where a silent off-by-one-frame would be invisible.
- Kept the test rigs production-faithful: registering `(tick_scenario_clock,
  track_*).chain()` so the clock actually advances, rather than leaving `now`
  frozen at 0 (which would have made "held is quiet" pass for the wrong reason).

## What went wrong

- Paid a cold ~4min compile on `cargo test -p nova_scenario` before discovering
  the serde round-trip tests need `--features serde` - the crate-in-isolation
  feature-unification trap. Root cause: ran the crate-scoped test BEFORE grepping
  the lessons ledger, which already documents this exact crate + failure
  (`crate-solo-tests-miss-unified-features`, now x5). The lesson says "grep the
  ledger for the crate name before crate-scoped runs" and I did the run first.
- Minor churn from tool ordering: `tatr new` (in the plan phase) wrote the task
  folder into the shared checkout before I had sprouted, so the bg-isolation
  guard then blocked editing that TASK.md and I had to re-author it inside the
  sprout and delete the stray. Sprout first, then create task files, when the
  bg-isolation guard is active.

## What to improve next time

- Before the FIRST crate-scoped `cargo test -p <crate>`, grep `LESSONS.md`
  for the crate name; for nova_scenario always pass `--features serde` (or run
  workspace-wide as CI does).
- When the bg-isolation guard is on, create the sprout at the very start of the
  cycle (before `tatr new`), so task files are authored in the isolated worktree
  from the outset.

## Action items

- [x] Bumped `crate-solo-tests-miss-unified-features` to x5 in LESSONS.md.
- [ ] tatr 20260717-151537 follow-up (pending user decision): make the orbit-hold
      and lock-refire durations author-configurable instead of hardcoded 5s
      constants - raised by the user during this cycle. Filed separately, not
      folded into this parity refactor.
