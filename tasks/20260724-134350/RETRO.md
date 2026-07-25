# Retro: Drawer objective log rows

- TASK: 20260724-134350
- BRANCH: feature/drawer-objective-log
- REVIEW ROUNDS: 1

## What went well

- Checking the actual completion mechanism before implementing the owner
  amendment kept the design local to the drawer: `GameObjectives` stayed active
  only, and the drawer derived completed rows from the same diff shape
  `objective_feedback` already uses.
- The tests were written at the row-structure boundary, not against styling
  intent. They prove active rows, completed rows, teardown clearing, empty state
  chrome and rebuild replacement.
- The out-of-context review approved the branch with no findings, and the
  in-session supplement re-ran the load-bearing checks plus the live doc sweep.

## What went wrong

- The first review diff included inherited Kenney license noise because local
  `master` had moved after the sprout was cut. Root cause: I spawned review
  before checking `master...branch` after a possible base move.
- The first drawer test compile failed because a helper retained `#[test]` and
  two child-iteration sites passed `&Entity` where Bevy expected `Entity`. Root
  cause: I patched the test block mechanically and let the compiler be the first
  exact Bevy-API check.

## What to improve next time

- Before spawning out-of-context review, run `git diff --name-only
  master...<branch>` against current local `master`; if it contains inherited
  base noise, merge `master` first and review the clean comparison.
- For Bevy UI test helpers, compile the helper skeleton before expanding the
  assertion set, so attribute and iterator-shape mistakes do not hide the first
  behavior failure.

## Action items

- [x] Added `review-current-base-before-ooc` to `LESSONS.md`.
