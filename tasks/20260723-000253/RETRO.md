# Retro: SetAllegiance scenario action (20260723-000253)

## What went well

- Mirroring an existing action (`SetSpeedCap`) end to end - config struct,
  variant, dispatch, scoped-query apply, warn-on-missing-id, RON round-trip
  test - made a ~50-line engine addition land clean on the first pass. The
  `Allegiance` type was already reachable via the existing nova_gameplay
  prelude import, so no new dependency.
- The make-or-break semantics were verified IN SOURCE before building content on
  top: AI targeting reads `Allegiance` from live per-frame queries (nothing
  caches it at spawn), targeting already handles `Changed<Allegiance>` (a
  hostile lock target flipping non-hostile clears the lock), and combat-lock
  acquisition is stance-gated, not hostility-gated - so a Neutral ship can be
  painted and `OnCombatLock` fires. Runtime allegiance change is an anticipated
  engine semantic; the new action goes with the grain (the lock-dwell function
  even carries an explicit "stealth extension seam" comment).

## What went wrong / was tricky

- Review round 1 caught a real HIGH: the scenario LINT's `check_action` match
  enumerates every id-referencing action for dangling-target validation, and the
  new variant fell into the `_ => {}` catch-all - a typo'd SetAllegiance id
  would lint clean and silently no-op at runtime, with the ch3 rework about to
  author exactly this action. Fixed with the one-arm addition + a fail-first
  dangling-target lint test. (This is `lint-covers-types-not-variants` biting in
  its adjacent form: adding an enum variant must sweep every exhaustive
  enumeration of that enum, and the compiler only catches the ones without a
  catch-all arm.)
- The round-1 review process crashed mid-run (host process exit) and its state
  was lost. Recovery: re-verified the deep semantics directly, then relaunched a
  compact reviewer for the diff mechanics with the semantic findings passed in
  as given - no duplicated deep-dive, and the review trail stayed honest (the
  REVIEW.md notes the out-of-band verification).

## Lessons / what to do differently

- When ADDING a variant to an enum that has catch-all matches, grep every
  `match` on that enum for `_ =>` arms and audit each one (here: the lint's
  `check_action`); the compiler's exhaustiveness check only guards the matches
  without a catch-all. A dangling-id validator missing a new id-bearing action
  is the canonical instance.
- Before building content on a new engine primitive, verify its consumers read
  the affected state LIVE (queries) rather than caching at spawn - the whole
  feature rests on that, and it is cheaper to confirm in source than to debug in
  content.

## Follow-ups

- None blocking. The ch3 stealth rework (20260723-000320) consumes this action
  next.
