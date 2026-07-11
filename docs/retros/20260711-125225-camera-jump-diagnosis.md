# Retro: Camera jump at speed - diagnosed, fix routed

- TASK: 20260711-125225
- BRANCH: fix/camera-jump-hunt (squash-merged as 0ac827d)
- REVIEW ROUNDS: 1 (APPROVE, no findings)

Second diagnosis-only cycle of the day; evidence and routing in the task
file.

## What went well

- **The on-screen metric settled it in one trace.** Measuring the ship in
  camera space (what the player sees) instead of world-space camera
  motion turned "sometimes it jumps" into "6.84 u per 100 ms hitch at
  300 u/s, zero otherwise" - and simultaneously measured 121711's
  zoom-out as a 22 u lerp lag, one rig for two tasks.
- **Task-splitting followed the user's own queue**: lag fix next (zoom
  cap task), architecture decision last (the spike), each handed real
  numbers. No scope creep into a feel decision that is not mine to make.

## What went wrong

- **The landing sequence almost ran from inside the worktree again** (a
  compound command starting with `cd <worktree>` ended in
  `git merge --squash`). The exit code caught it; the retro rule from
  20260709-160753 ("landing is its own command, `pwd` first, never after
  a cd") exists precisely for this and held only on the second attempt.
  Root cause: batching review-commit + sync-check + landing into one
  command line for speed.

## What to improve next time

- The landing rule, sharpened: the squash-merge command may not contain a
  `cd` at all. If the previous command cd'd anywhere, the landing starts
  a fresh command with `pwd`.

## Action items

- [x] Numbers handed to 20260711-121711 and 20260711-125227 (task files
      updated in the landed branch).
