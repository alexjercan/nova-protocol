# Retro: flip remaining player scenarios to finite auto-reloading ammo

- TASK: 20260717-101414
- BRANCH: feature/scenarios-finite-ammo (landed 1f02bbb0)
- REVIEW ROUNDS: 1 (APPROVE, no findings)

## What went well

- **Whole-tree grep decided scope, not memory.** Grepping `infinite_ammo: true`
  across the tree gave an exact, classifiable list (Broadside RON+Rust, example
  mod, one loader test fixture) instead of guessing which scenarios to touch -
  and made "no player scenario left behind" a checkable claim in review.
- **Flipping both the RON and its Rust builder together kept `content_ron_parity`
  green.** The parity test is the guard against exactly the drift a half-flip
  would cause; running it was the fast confirmation.
- **Surfaced the one judgment call instead of burying it.** The example mod is a
  modding showcase, not a mission; flipping it is defensible but reversible, so
  it went in NOTES/TASK/REVIEW as an explicit "revert this one line if you meant
  it as a sandbox" rather than a silent decision.
- The feature landed across three tasks (mechanic -> readout -> scenarios) each
  small and independently reviewable, which is what let the seams
  (`progress()`, the catalog reload default) do the heavy lifting.

## What went wrong

- Nothing of substance. One minor friction: `tatr new` ran in the shared main
  checkout wrote an uncommitted TASK.md that the freshly-sprouted worktree did
  not inherit (worktrees share commits, not the main checkout's dirty tree), so
  the bare file had to be removed from the main checkout and recreated in the
  worktree to avoid a shared-checkout write leak.

## What to improve next time

- When creating a task that will immediately be worked in a NEW worktree, sprout
  first and run `tatr new` inside the worktree - so the task file is born on the
  branch, not left uncommitted in the shared checkout.

## Action items

- [x] Feature complete (mechanic + readout + scenario flips all landed). No
  follow-up tasks; `infinite_ammo` deliberately retained for test/debug.
