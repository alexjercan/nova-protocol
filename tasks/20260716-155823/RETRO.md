# Retro: Explicit content generator bin; make content_ron_parity assert-only

- TASK: 20260716-155823
- BRANCH: refactor/gen-content-bin (landed a42dba5c)
- REVIEW ROUNDS: 1

## What went well

- Routing review finding R1.2 from the previous task into THIS task's
  TASK.md before starting meant the uniformity guard arrived pre-specced;
  zero design time was spent on it mid-implementation.
- Both new guards were A/B-proven able to fail before the review round,
  so the review had evidence to check instead of claims to trust - one
  round, no findings.
- Extracting the single content_files() map first made the bin and the
  test trivially incapable of divergence, which is the whole point of
  the task.

## What went wrong

- Nothing significant. One pre-existing doc rot instance surfaced
  (modding-ron.md cited a test name that never existed post-rename);
  the docs sweep caught it because it grepped for the CONCEPT (parity)
  not just the current symbol.

## What to improve next time

- Keep sweeping docs by concept as well as symbol; stale docs cite old
  names by definition, so a symbol-only grep structurally misses them.

## Action items

- [x] Ledger: bumped cross-cycle-warning-with-numbers (review-finding
      routing variant).
