# Retro: Bullet first-frame pop - easing seed at spawn

- TASK: 20260711-121839
- BRANCH: fix/bullet-spawn-render-pose (squash-merged as 6d059ae)
- REVIEW ROUNDS: 1 (APPROVE, no findings)

## What went well

- **The plan's verify-first steps paid for themselves twice.** Reading
  bevy_transform_interpolation 0.5.0's actual schedules (start at
  FixedFirst, end at FixedLast, ease only when both are Some) confirmed
  the mechanism in minutes AND killed the plan's primary fix direction
  (split-clock spawn) on paper: avian's same-tick writeback makes a
  render-pose spawn Transform unreachable by the first render, so the
  "fallback" was provably the whole fix. No prototype cycle wasted.
- **Fail-first A/B with a derived magnitude.** The regression was written
  to fail before the fix existed, and its pre-fix failure (2.09 u
  cross-stream) matched the back-of-envelope prediction (~2.3 u at
  150 u/s), which is what made the post-fix "exactly 0" trustworthy
  rather than merely green.

## What went wrong

- **The regression's first version had its own frame-composition bug**
  (rotated the muzzle's mount translation by the muzzle's own rotation),
  producing a constant 0.148 u phantom cross-stream offset that
  initially read as a residual bug in the fix. Root cause: the test
  hand-composed the expected pose instead of reusing the production
  composition (`local_pose_in_root` lives in the same module and was
  usable). The tell that unmasked it: the offset was CONSTANT across
  easing alpha - timing artifacts scale with alpha, rig-math errors do
  not.
- **A sed one-liner used to toggle the A/B restored both seed fields
  with the same value** (rotation seeded with a Vec3), requiring a
  manual fix-up. Toggling a fix off for A/B via text substitution is
  fragile; the branch already had the pre-fix state one commit away.

## What to improve next time

- When a test must compose an expected pose along a hierarchy, call the
  production composition helper instead of re-deriving it inline; if the
  helper is private, the test module can still reach it.
- A/B toggles run against the committed pre-fix state (stash/checkout),
  not against sed-edited source.
- Diagnostic heuristic worth keeping: an error invariant across the
  interpolation alpha implicates the rig's math, not the timing under
  test.

## Action items

- [x] Ledger updated: new `constant-offset-is-rig-math` and
      `ab-toggle-via-vcs-not-sed` entries; `reuse-production-helpers-in-tests`
      seeded at x1.
- [ ] None outstanding in code; the torpedo cycle (20260711-114640)
      inherits the review's note to reuse the easing-seed pattern.
