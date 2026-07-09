# Retro: Targeting module + signature auto-acquisition

- TASK: 20260709-192503
- BRANCH: feature/signature-acquisition (squash-merged as 0c3e408)
- REVIEW ROUNDS: 2 (round 1 APPROVE with 1 MINOR doc finding; round 2
  APPROVE)

## What went well

- **Three mechanical/behavioral commits on the branch.** Move, rename,
  behavior - each diff reviewable alone; the reviewer could textually verify
  the move was faithful instead of re-reading the whole system. Worth
  repeating whenever a task mixes refactor and feature.
- **The python-scripted extraction.** Moving ~180 lines plus their tests
  with exact-string slicing (assert-guarded) instead of retyping meant the
  move introduced zero drift; the compiler and 14 green tests confirmed on
  the first run.
- **An old TODO retired for free.** The lock resource carried a
  scuffed-refactor TODO pointing at a long-closed torpedo task; the
  move+rename is exactly that refactor, and saying so in the Resolution
  closes the loop instead of leaving a dangling TODO reference.

## What went wrong

- **Living docs missed the rename sweep (R1.1).** The workspace-wide sed
  covered code but not docs/; the widget doc kept naming a type that no
  longer exists. Root cause: treating "rename across the workspace" as a
  code operation. Renames must sweep docs/ too, splitting living docs
  (update) from dated decision records (leave).

## What to improve next time

- Mechanical renames: grep code AND docs; update living docs, leave dated
  spike/retro records as history.

## Action items

- None new; hostility generalization already tracked as 20260708-203708.
