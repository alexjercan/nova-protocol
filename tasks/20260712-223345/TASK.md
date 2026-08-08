# Reconcile targeting docs with the two-slot lock model

- STATUS: CLOSED
- PRIORITY: 45
- TAGS: v0.5.0, targeting, docs, spike, wontdo

## Goal

Bring the targeting docs in line with the shipped two-slot model (spike
20260712-222610, rounds 1-4 final) once tasks 20260712-223034/231141/
223035/223036 land. Retros are
dated records and are NOT rewritten; design docs and spikes get corrected
or superseded in place.

## Steps

- [x] Write tasks/20260712-223345/NOTES.md: the shipped model as the
      current source of truth - TravelLock/CombatLock components on the
      ship root, travel auto-cast + sticky + scroll cycling, seed-on-raise,
      combat enemy ordering, fire gating, unlock key, HostileContacts,
      SHIFT+SCROLL components, HUD language. Written against the code as
      landed, not the spike's intentions; record where implementation
      diverged.
- [x] tasks/20260711-163800/SPIKE.md: SUPERSEDED banner
      (torpedo exclusion overturned by 20260712-212742; CTRL+scroll cycle
      replaced by view-routed SCROLL; resource state replaced by
      components), pointing at the new design doc.
- [x] tasks/20260712-203235/SPIKE.md:
      banner on the stickiness half (B5 ship-only stickiness -> per-slot
      stickiness); the inset-scope half still stands.
- [x] tasks/20260712-215256/SPIKE.md and
      tasks/20260712-215733/SPIKE.md: verify their
      SUPERSEDED/addendum notes match what actually shipped; append Fix
      record entries (what shipped, task pointers).
- [x] tasks/20260712-222610/SPIKE.md: append Fix
      record entries per landed task; resolve its open questions with the
      playtest verdicts known by then.
- [x] tasks/20260709-192358/NOTES.md: fix cone range 2000 m -> 20 km
      (`TARGETING_MAX_RANGE`, targeting.rs:119), "nearest AI ship" fallback
      -> hostile-relation gate, and the scroll gesture change; re-read the
      rest against the code.
- [x] tasks/20260710-195952/NOTES.md: verify against the code (expected
      accurate; range gates unchanged by the slot split).
- [x] Sweep README/CHANGELOG for targeting/input behavior claims the new
      model invalidates (CHANGELOG entries are dated records - only fix
      forward-looking text).

## Notes

- Spike: tasks/20260712-222610/SPIKE.md.
- Depends on: 20260712-223034, 20260712-231141, 20260712-223035, 20260712-223036 (write
  docs LAST, against shipped code).
- Replaces closed task 20260712-215958 (was scoped to the superseded
  unified-single-lock model).
