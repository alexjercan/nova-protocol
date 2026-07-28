# Retro: NOVA OS ship app - minimize the blip overlay

- TASK: 20260728-125514
- BRANCH: feature/ship-blip-minimize
- REVIEW ROUNDS: 1 (out-of-context APPROVE, 2 NITs) + R2 in-session verify

See TASK.md Work Log for what changed and DECISION.md for the supersede; this file
is process only.

## What went well

- Compounding visibly paid off: last cycle's `dead-code-hides-under-cfg-test-reader`
  lesson was applied proactively - I ran a NON-test `cargo check` before declaring
  done (this task REMOVED helpers + struct fields, the exact dead-code risk), and it
  was clean. The reviewer independently confirmed it. The lesson turned a likely
  round-2 finding into a non-event.
- The DECISION.md supersede chain was done right this time (bidirectional link +
  scoped annotation on the old record); the reviewer verified both directions.
- Confirmed the two look-forks with the owner (all-labels-with-contrast + status
  dot) BEFORE building, so the visual came back right the first time - no rework,
  APPROVE on round 1.

## What went wrong

- R1.2 (NIT): I deleted `integrity_bar_and_ammo_pips_track_live_data` wholesale
  because it tested the removed `bar_fraction`/`ammo_pips`, but it also carried the
  ONLY assertion for a still-live edge (`integrity()`/`status()` "unknown health
  reads nominal"), which now drives the status dot + panel. Root cause: deleted the
  test by its subject, not by auditing each assertion for surviving coverage.
- R1.1 (NIT): the non-selected blip border used `PHOSPHOR.with_alpha(0)` in one
  place and `AMBER.with_alpha(0)` in another - both invisible, but an inconsistency
  from editing the two sites separately.

## What to improve next time

- When deleting a test because it covered removed code, read each assertion first
  and re-home any that still pin surviving behavior - a test is a bag of
  assertions, not a single unit tied to one symbol.

## Action items

- [x] Ledger: added `deleting-a-test-salvage-live-assertions`.
- Manual acceptance (owner playtest) open - listed in REVIEW.md R2 (readability +
  amber-dot-at-a-glance). This closes the blip-overlay feedback batch; the recenter
  task (`20260728-125510`) remains.
