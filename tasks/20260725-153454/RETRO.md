# Retro: Keep final completed objectives in drawer log

- TASK: 20260725-153454
- BRANCH: master
- REVIEW ROUNDS: 1

## What went well

- User feedback mapped directly to a small uncovered edge case: the final active
  objective disappearing from `GameObjectives`.
- The fix stayed local by moving clear semantics to drawer teardown instead of
  changing shared objective state.

## What went wrong

- The original task over-generalized an empty `GameObjectives` list as teardown.
  Root cause: it copied `objective_feedback`'s transient-feedback rule into a
  persistent drawer-log surface where empty can also mean "final objective done."

## What to improve next time

- For persistent history surfaces, test the zero-active terminal state
  separately from teardown; the same source value can mean different things at
  different UI persistence levels.

## Action items

- [x] Covered final-completion retention with
  `drawer_objectives_keep_final_completed_row_with_strike`.
