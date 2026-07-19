# Retro: continuous invariant checks

- TASK: 20260719-114931
- BRANCH: feature/probe-invariants (squash-landed as 735bb198)
- REVIEW ROUNDS: 1 (APPROVE)

## What went well

- The surface map (Explore agent, delegated while sprouting) flagged the two
  facts that shaped the whole design BEFORE any code: the speed cap is a
  soft taper gate (so a hard-cap assert would flake on autopilot/gravity)
  and variables carry no monotonicity guarantee (so registration must be
  opt-in). The natural wrong version of this task - assert the design's
  aspirations instead of the engine's guarantees - was avoided at the plan
  step, not discovered as CI flake later.
- Second consecutive first-run-green cycle: 36 tests passed on the first
  cargo test. The verify-hooks-in-source discipline is now paying
  compounding dividends.
- The half-ticked-step catch worked as designed: the summary-entry clause
  was ticked but unimplemented; the post-commit TASK.md re-read caught it,
  and it was implemented + tested (summary precedes run_end, pinned
  positionally) instead of quietly amending the step.

## What went wrong

- The tick-then-implement inversion itself: the sed-tick-all-steps habit
  ticked the whole checklist at once, including the clause that was not
  done. Cheap this time only because the re-read caught it.

## What to improve next time

- Do not bulk-tick steps with sed. Tick each step individually while
  re-reading its EVERY clause against the diff - the bulk tick is exactly
  how half-ticked-compound-steps happens despite knowing the lesson.

## Action items

- [x] Lesson bumped: half-ticked-compound-steps (the bulk-tick habit is the
      mechanism; tick individually).
- [ ] T5 (20260719-112304) notes inherited: present violation counts
      per invariant name (a stuck entity violates every frame), alongside
      the two recorder NITs from T2's review.
