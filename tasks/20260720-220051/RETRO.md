# Retro: promote the 6 pending lessons

## What went well

- Unlike bevy (where most lessons were already in AGENTS.md), nova's AGENTS.md
  did NOT document these, so 5 needed a genuine fold into a "Promoted ledger
  lessons" Conventions block. The 6th (out-of-context-review-pass) is already
  flow round-1 practice, so it just got the annotation. Verified each fold is
  present and faithful before annotating; the reviewer re-verified every rule
  (including cross-checking the out-of-context annotation against the review
  skill) and found no false promotions.

## What went wrong

- A parallel session committed a tooling spike to nova master mid-cycle, so the
  branch was behind at land time (`no-concurrent-git-same-tree`). The land
  protocol handled it: merged master into the branch (clean, the parallel commit
  only touched tasks/ files), re-verified the ledger + fold intact, then landed.

## What to improve next time

- On an actively-developed repo like nova, expect master to move during a cycle;
  the merge-then-land step is not a formality. The re-verify after merge caught
  nothing broken here only because the parallel commit was disjoint - always
  re-run the proof after the merge.

## Action items

- [x] 6 lessons promoted (5 folded into AGENTS.md); Pending promotions empty; landed 2669d332.
