# Retro: Tighten and re-section CHANGELOG.md

- TASK: 20260716-102950
- BRANCH: changelog-revamp
- REVIEW ROUNDS: 2

## What went well

- The review's information-loss check was mechanical, not vibes: a grep of ~44
  distinctive tokens/numbers from the old file against the new one, plus a
  bullet-count reconciliation (93 = 94 - 1 deliberate merge). That turned "did I
  drop anything?" from an eyeball guess into a closed proof and caught the one
  real defect.
- Fixing the subsystem vocabulary up front in the spike/task (11 named sections,
  canonical order) made the rewrite mechanical and the review's consistency
  check trivial.

## What went wrong

- R1.1 (BLOCKER): the "Screenshot Reel capture set" entry was silently dropped
  during the manual regroup. Root cause: regrouping N source bullets into M
  sections by hand has no built-in conservation check, so a single bullet that
  did not obviously belong to any early section fell through unnoticed while
  writing the file top-to-bottom.

## What to improve next time

- When mechanically rewriting or regrouping a list-shaped doc, run the
  conservation check as part of implementation, before handing to review: pull
  the distinctive token/number from each source item and grep the new file,
  and reconcile counts. Do not rely on reading the diff.

## Action items

- [x] Recorded `conserve-on-regroup` in docs/LESSONS.md.
- [x] Restored the dropped entry (CHANGELOG.md 0.6.0 Internals & Tooling).
