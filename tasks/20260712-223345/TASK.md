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

- [ ] Write docs/2026-07-12-travel-combat-locks.md: the shipped model as the
      current source of truth - TravelLock/CombatLock components on the
      ship root, travel auto-cast + sticky + scroll cycling, seed-on-raise,
      combat enemy ordering, fire gating, unlock key, HostileContacts,
      SHIFT+SCROLL components, HUD language. Written against the code as
      landed, not the spike's intentions; record where implementation
      diverged.
- [ ] docs/spikes/20260711-163800-multi-target-cycle.md: SUPERSEDED banner
      (torpedo exclusion overturned by 20260712-212742; CTRL+scroll cycle
      replaced by view-routed SCROLL; resource state replaced by
      components), pointing at the new design doc.
- [ ] docs/spikes/20260712-203235-lock-stickiness-and-inset-scope.md:
      banner on the stickiness half (B5 ship-only stickiness -> per-slot
      stickiness); the inset-scope half still stands.
- [ ] docs/spikes/20260712-215256-combat-travel-lock-separation.md and
      docs/spikes/20260712-215733-unified-target-computer.md: verify their
      SUPERSEDED/addendum notes match what actually shipped; append Fix
      record entries (what shipped, task pointers).
- [ ] docs/spikes/20260712-222610-travel-combat-lock-slots.md: append Fix
      record entries per landed task; resolve its open questions with the
      playtest verdicts known by then.
- [ ] docs/2026-07-09-component-lock.md: fix cone range 2000 m -> 20 km
      (`TARGETING_MAX_RANGE`, targeting.rs:119), "nearest AI ship" fallback
      -> hostile-relation gate, and the scroll gesture change; re-read the
      rest against the code.
- [ ] docs/2026-07-10-signature-lock.md: verify against the code (expected
      accurate; range gates unchanged by the slot split).
- [ ] Sweep README/CHANGELOG for targeting/input behavior claims the new
      model invalidates (CHANGELOG entries are dated records - only fix
      forward-looking text).

## Notes

- Spike: docs/spikes/20260712-222610-travel-combat-lock-slots.md.
- Depends on: 20260712-223034, 20260712-231141, 20260712-223035, 20260712-223036 (write
  docs LAST, against shipped code).
- Replaces closed task 20260712-215958 (was scoped to the superseded
  unified-single-lock model).
