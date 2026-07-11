# Retro: Holo ribbon terminates at the arrival park point

- TASK: 20260710-214316
- BRANCH: fix/ribbon-park-point (squash-landed as efe8db0)
- REVIEW ROUNDS: 2

## What went well

- The plan phase answered the task note's open question ("publish the
  resolved radius or the effective standoff?") with a third option -
  publish the park point itself - and wrote the three options and the
  rejection reasons into TASK.md before any code. Review round 1 could
  re-derive the choice in minutes because the alternatives were already
  on paper.
- Fail-first regression paid off exactly as promoted: the new ribbon
  test run against the unmodified ribbon failed with "ends at
  [0, 0, -300], got park [0, 0, -250]" - the full 50u standoff
  overshoot as a recorded number, then passed after the one-line fix.
- The inside-envelope degeneracy (park point pinned to the ship, never
  a backward stub out of the body) was designed at plan time from the
  module's own "instruments must not out-promise the autopilot" rule,
  not discovered in review. The delivery guard on its integration
  sample (`expect("the leg passes through the park envelope")`) kept
  the new null-shaped assertion honest.
- Landing discipline (own command, no cd, pwd first) went through
  cleanly on the first try.

## What went wrong

- R1.1 (shadowed `standoff` binding): the park-point assertions were
  inserted mid-test without re-reading the rest of
  `goto_standoff_is_surface_relative_for_sized_targets`, so the
  pre-existing identical binding below became a redundant shadow. Root
  cause: top-down editing - the insertion point was treated as the end
  of the change, and the function was never re-read as a whole.
- R1.2 (fourth `to_target` normalization): minimal-diff instinct -
  avoiding touching existing lines - added `normalize_or_zero()` next
  to three existing normalizations instead of hoisting one
  `closing_dir` above the branch. The hoist was strictly better and
  the repo's rules (correct and maintainable over small) already said
  so; it cost a review round to do what write time should have done.

Both were caught by the review phase working as designed, but each is a
round that a whole-function re-read at write time would have saved.

## What to improve next time

- After inserting into an existing function or test, re-read the whole
  function before committing: look specifically for bindings and
  assertions the insertion just duplicated or obsoleted.
- When new code needs a value an adjacent branch already computes,
  hoist it at write time; leaving the duplicate "to keep the diff
  small" just moves the work into review.

## Action items

- [x] LESSONS.md: new `reread-after-insert` (x1); bumped
  `fail-first-regression-ab` to x9 and
  `delivery-guards-on-null-assertions` to x5 (applications with
  recorded numbers).
- No follow-up code tasks: R1.3 (degraded-state ribbon endpoint) was
  reviewed as strictly better than the old center endpoint and needs
  no work.
