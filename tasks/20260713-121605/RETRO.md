# Retro: Lock-wins turret routing

- TASK: 20260713-121605
- BRANCH: fix/lock-wins-turret-routing (landed 9c2d36c, recovered - see below)
- REVIEW ROUNDS: 1 (APPROVE)

## What went well

- The original spike marked this routing default as an explicit playtest
  knob with the alternative already analyzed, so the user's verdict
  translated to a deletion, not a design session: lock-wins removed the
  whole raised special-case (the ray fallback IS manual aim when no lock
  exists).
- The pinned test inverted cleanly into a fail-first A/B against the old
  feed, with the lead-velocity delta as the delivery guard.

## What went wrong

- The landing ABORTED and the worktree was removed anyway: `tatr new` had
  left an uncommitted task stub in the main checkout, the squash merge
  refused to overwrite it, and the newline-separated landing sequence ran
  `sprout rm` regardless - deleting the branch before anything landed.
  Recovered because `sprout rm` prints the tip hash and squash-merge
  accepts a commit-ish (`git merge --squash b8f1d2a`); the stub was
  removed and the landing redone.
- Root cause of the collision: earlier cycles committed task files on
  master at plan time BEFORE sprouting; this ad-hoc cycle ran `tatr new`
  on master, never committed the stub, and recreated the file in the
  worktree.

## What to improve next time

- The landing must be one `&&`-chain: `git merge --squash <branch> &&
  git commit ... && sprout rm <branch>` - an aborted merge then stops the
  teardown. Keep `pwd`/branch checks as separate preceding commands.
- Ad-hoc tasks: commit the `tatr new` stub on master (or create the task
  file only inside the worktree) before branching - never both.

## Action items

- [x] LESSONS.md: new lesson `landing-chain-and-stub-collision`.
