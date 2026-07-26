# Retro: NOVA OS casing playtest polish

- TASK: 20260726-230237
- BRANCH: feature/nova-os-casing-polish
- REVIEW ROUNDS: 1 (APPROVE, in-session)

## What went well

- The playtest feedback was concrete and item-by-item, so it mapped straight to
  a 6-step task with a capture per item as the proof. Fast, clean cycle.
- Pulling the exact PoC `:root` `--case-*` hexes before recolouring meant the
  "too blue" fix landed in one pass instead of eyeball-tuning RGB by hand.

## What went wrong

- Nothing broke, but the follow-up task was created in the MAIN checkout with
  `tatr new` BEFORE sprouting, so the worktree (branched from the prior landed
  commit) did not contain it. Had to carry-and-clean the stub into the branch as
  the first work step. Root cause: ran `tatr new` from the main checkout out of
  habit instead of creating it inside the worktree.

## What to improve next time

- For a follow-up cycle, sprout FIRST, then `tatr new` inside the worktree (or
  create the task on the branch), so the task file is born on the branch and
  never needs carrying. (The `no-worktree-task-born-off-branch` pattern.)

## Action items

- [x] Ledger: bumped `tatr-new-then-sprout-strands-the-task-file` to x3 (already
  promoted to the tatr + flow skills).
- No follow-up code task; this closes the owner's playtest feedback on the
  casing. The one prior NIT (reflection wording) is moot - reflection retuned.
