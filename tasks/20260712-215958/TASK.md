# Reconcile targeting docs: supersession banners + stale-claim fixes

- STATUS: CLOSED
- PRIORITY: 45
- TAGS: v0.5.0, targeting, docs, spike

## Goal

Bring the targeting docs in line with the unified target computer model
(spike 20260712-215733) once tasks 20260712-215957 and 20260712-215402 land.
Retros are dated records and are NOT rewritten; design docs and spikes get
corrected or superseded in place.

## Steps

- [ ] Write docs/2026-07-12-unified-target-computer.md: the shipped model as
      the current source of truth - component state on the ship root, list
      membership (wide cone, all classes), ranking, auto-pick policy
      (hostiles-first / nav-in-tight-cone / signature fallback), universal
      stickiness, CTRL+scroll cycle, HostileContacts for edge indicators.
      Written against the code as landed, not the spike's intentions.
- [ ] docs/spikes/20260711-163800-multi-target-cycle.md: SUPERSEDED banner at
      the top (torpedo exclusion overturned by task 20260712-212742;
      ships-only resource-based cycle replaced by the unified component
      list), pointing at the new design doc.
- [ ] docs/spikes/20260712-203235-lock-stickiness-and-inset-scope.md: banner
      on the stickiness half only (B5 ship-only stickiness -> universal
      stickiness); the inset-scope half still stands.
- [ ] docs/spikes/20260712-215256-combat-travel-lock-separation.md and
      docs/spikes/20260712-215733-unified-target-computer.md: append Fix
      record entries (what shipped, task pointers).
- [ ] docs/2026-07-09-component-lock.md: fix cone range 2000 m -> 20 km
      (`TARGETING_MAX_RANGE`, targeting.rs:119) and "nearest AI ship"
      fallback -> hostile-relation gate; re-read the rest against the code
      while there.
- [ ] docs/2026-07-10-signature-lock.md: verify against the code (expected
      accurate; the range-gate model did not change).
- [ ] Sweep README/CHANGELOG for targeting behavior claims that the unified
      model invalidates (CHANGELOG entries are dated records - only fix
      forward-looking text).

## Notes

- Spike: docs/spikes/20260712-215733-unified-target-computer.md.
- Depends on: 20260712-215957 and 20260712-215402 (docs must describe what
  actually shipped - write this LAST).
- The spike's "Docs found wrong or stale" section is the checklist source;
  if implementation diverged from the spike, the code wins and the design
  doc records the divergence.

## Closure (2026-07-12, superseded - no code shipped)

Superseded together with the unified-single-lock direction (spike
20260712-215733) it was written against. The docs-reconcile role continues
as task 20260712-223345 under the two-slot model (spike
docs/spikes/20260712-222610-travel-combat-lock-slots.md).
