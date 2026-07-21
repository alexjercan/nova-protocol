# Retro: Final Tally (ch3b) - gravity-well finale

- TASK: 20260721-161020
- BRANCH: content/final-tally-ch3b (landed 09463091)
- REVIEW ROUNDS: 1 (APPROVE; MINOR + NIT fixed in-round)

## What went well

- The whole cycle ran on inherited patterns: the slice harness, the
  variant-handler idiom, terminal acts (the lesson minted one cycle ago
  applied at authoring time - both outcome paths shipped correct on the
  first pass), and the example's clock fast-forward. First-compile green
  on all 15 new/updated tests.
- The balance lint caught the flagship berth inside its own torpedo
  envelope; moving the berth (and pinning the clearance) beat acking - the
  audit ends the whole chain with zero base-campaign acks.
- The clock-mark vocabulary (mark_clock/clock_past) turned "breathe then
  cast off" and the paced epilogue into data the harness drives
  deterministically.

## What went wrong

- The kill-pickets-first ordering shipped with a cosmetic wart (orphaned
  objective + stale line) that the harness EXERCISED but did not assert
  on - the test drove the alternate ordering and checked only the
  mechanism (no deadlock), not the player-facing residue. Root cause:
  alternate-path tests written for machine correctness, not for what the
  HUD shows on that path.

## What to improve next time

- When a test drives an alternate ordering, assert the PLAYER-FACING state
  (objectives list, markers) on that path too, not just the act machine -
  residue is what the player sees.

## Action items

- none new; the R1.1/R1.2 fixes landed in-round.
