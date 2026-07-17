# Retro: Comms pacing queue

- TASK: 20260717-163033
- BRANCH: feature/comms-pacing-queue (landed 2fc938be)
- REVIEW ROUNDS: 2 (REQUEST_CHANGES -> APPROVE; 2 MAJOR, 2 MINOR, 1 NIT)

## What went well

- The spike's HUD research (the latest-wins single-line finding) made the
  design obvious and the fail-first natural: the arrival-order test is
  literally the old behavior inverted.
- Contract-shift test breakage was read as information: three "broken"
  objective tests pointed at a real pre-existing leak (ghosts fading over
  the menu after teardown), which the cycle then closed.
- The reviewer's structural pass caught a bug NO test I had written could
  see: the sync's length-only diff aliasing a retry's equal-count repush.

## What went wrong

- R1.1's first regression pin was VACUOUS: my rig put an update between
  clear and repush, so the sync masked the bug and the pin passed even
  with the fix reverted. The sabotage caught it (that is what sabotage is
  for), but I had written the pin from the fix's description instead of
  from the BUG's exact timing shape. A regression pin must reproduce the
  failure WINDOW, not just the failure ingredients.
- R1.2: the close-out claimed "sync carries dwell" as tested when no such
  test existed - the checklist said tests were written per feature and I
  ticked on the aggregate green run. A claimed test must be nameable.

## What to improve next time

- When pinning a race/window bug, write the pin FIRST against the broken
  code and watch it fail before implementing the fix (true fail-first
  order), rather than fix-then-pin-then-sabotage.

## Action items

- [x] docs/LESSONS.md: new lesson pin-the-window-not-the-ingredients (x1);
  would-it-fail-without-it bumped (the vacuous pin variant).
