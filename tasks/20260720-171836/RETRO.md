# Retro: Adopt flow v2 (nova-protocol)

- TASK: 20260720-171836
- BRANCH: chore/flow-v2-adoption (landed as ae7ef799; amendment repair 7264013f)
- REVIEW ROUNDS: 1 (out-of-context APPROVE; 1 MINOR + 1 NIT taken)

## What went well

- The largest migration (132 files, 89 findings adjudicated, 65 path
  mentions swept across every surface including wipe/guard scripts, CI and
  wiki) landed with zero invented verdicts and a reviewer-verified
  every-changed-line audit.
- The reviewer adjudicated AGAINST one of the work agent's sub-reviewers
  (a proposed over-tick) and FOR the residue - both directions of honesty.

## What went wrong

- The R1 amendments were claimed in REVIEW.md but not applied at landing:
  the untick script's single-line assert missed the multi-line step text
  and aborted, while the separately-written REVIEW.md and the land
  proceeded. The residue count (30 vs the claimed 31) exposed it right
  after landing; repaired in 7264013f with the incident documented. Root
  cause: writing the claim (REVIEW.md) in a different command than the
  change it claims, with no artifact re-check between them.

## What to improve next time

- Apply a review fix and record its Response from the same verified state:
  after any scripted edit, re-run the check that would expose its absence
  BEFORE writing the claim that it happened.

## Action items

- [x] Repair landed (7264013f); incident recorded in the close record.
