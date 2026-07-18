# Retro: Menu ships crash the asteroid - ORBIT-RCS gravity-authority gate

- TASK: 20260718-204640
- BRANCH: fix/orbit-rcs-gravity-authority
- REVIEW ROUNDS: 1 (APPROVE)

## What went well

- Diagnostic-first paid off with REAL numbers, not a theory. An Explore agent
  mapped the exact menu scene and surfaced the non-obvious key: the well derives
  `mu` from the asteroid's GEOMETRIC collider radius (~85u), not the nominal 20u,
  so `mu ~= 43000` and the orbit's local gravity (~2.2 u/s^2) exceeds
  `rcs_accel` (1.5). That single fact explained the crash and pointed straight
  at the fix criterion.
- fail-first-regression-ab: the reproduction test failed on the pre-fix code
  (REPROEXIT=101, `saw_rcs`) and passed after, so the fix is pinned to a
  discriminating assertion.
- The fix is conservative and principled: instead of blanket-disabling the
  speculative ORBIT-RCS trim, the authority gate (`g < rcs_accel * 0.5`) encodes
  the physical validity condition, so the feature keeps working in the weak
  wells where it is sound and reverts to the known-good main drive where it is
  not. For the affected wells it restores the exact pre-151102 path the user
  confirmed worked.
- Honest self-review: flagged that the headless guard is mechanism-level (RCS
  must not engage) rather than a full crash reproduction - a clean headless
  orbit is perturbation-stable, so `r_min` did not collapse; the real spiral
  needs the irregular asteroid well + two ships. Recorded by-eye as outstanding
  rather than overclaiming.

## What went wrong

- The regression shipped in the first place because the ORBIT-RCS test
  (20260718-151102) exercised only ONE convenient operating point (r=50,
  mu=1200 - a WEAK well where g=0.48 < rcs_accel), never the regime where the
  governing ratio (gravity vs RCS authority) flips. That task's own review even
  flagged "needs a playtest" for the feel, but the headless coverage gap was
  the deeper miss.

## What to improve next time

- When a behavior turns on a PHYSICAL RATIO (here local gravity vs RCS accel),
  the headless test must span the point where the ratio crosses 1, not just one
  convenient value. A single-regime test that passes proves nothing about the
  other regime - which is exactly where the user hit the bug.

## Action items

- [x] Bumped `diagnostic-first` (x11) and `fail-first-regression-ab` (x12) in
  LESSONS.md.
- [x] Added `test-across-the-ratio-boundary` to LESSONS.md.
- [ ] By-eye: confirm the two menu haulers hold their orbit in the running game
  (recorded in TASK.md; cannot be done headless).
