# Retro: Live-structure aim anchor (AI + player + camera)

- TASK: 20260709-150711
- BRANCH: fix/live-structure-anchor (squash-merged as 7ffad59)
- REVIEW ROUNDS: 1 (APPROVE, no findings)

Small, clean bug-fix cycle opening the component-lock arc. What shipped is in
the task's Resolution.

## What went well

- **The discriminating test.** The cone test places a candidate inside the
  18 degree cone from the ANCHOR but 33 degrees off the ORIGIN bearing, so it
  locks only with the fix - a behavioral test that fails without the change
  by construction (the proving-the-test lesson from the camera-twitch retro,
  applied at design time instead of after review).
- **One helper, three consumers, in one pass.** Extracting the camera's COM
  lift into `live_structure_anchor` and moving AI + player onto it in the
  same task means the anchor math cannot drift apart again; the camera's
  existing tests doubled as regression cover for the refactor.
- **Concurrent master movement handled by the book.** The user landed other
  work (CI retro, gravity-wells spike) on master mid-task; flow's
  sync-before-merge step (merge master into the branch, re-verify, then
  squash) absorbed it without touching master history.

## What went wrong

- **Forgot the RunSystemOnce import in an existing test module.** player.rs's
  tests predate run_system_once usage; pasting new tests in without checking
  the module's imports cost one compile round. Trivial, but the third session
  in a row where a first test compile failed on something checkable by
  reading the test module header first.
- **`grep -cE` as a success check bit again.** `grep -c` exits nonzero on a
  zero count, which short-circuited a && chain after a CLEAN check. Use
  `grep -c ... ; echo` or invert explicitly when zero matches is the good
  case.

## What to improve next time

- Before appending tests to an existing module, read its use-block; new
  test-only imports (RunSystemOnce, SystemState) go at the module head, not
  discovered by the compiler.
- Treat `grep -c` in command chains as a counter, never as a gate.

## Action items

- [x] Own-origin half of the AI vector noted on task 20260709-155921 during
  review (both ends of the chase vector should track live structure).
