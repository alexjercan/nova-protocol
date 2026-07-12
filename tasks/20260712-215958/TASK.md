# Reconcile targeting docs: supersession banners + stale-claim fixes

- STATUS: OPEN
- PRIORITY: 45
- TAGS: v0.5.0, targeting, docs, spike

## Goal

Bring the targeting docs in line with the unified target computer model
(spike 20260712-215733) once tasks 20260712-215957 and 20260712-215402 land.
Retros are dated records and are NOT rewritten; design docs and spikes get
corrected or superseded in place:

- docs/spikes/20260711-163800-multi-target-cycle.md: SUPERSEDED banner
  (torpedo exclusion overturned by 20260712-212742; ships-only resource-based
  cycle replaced by the unified component list).
- docs/spikes/20260712-203235-lock-stickiness-and-inset-scope.md: banner on
  the stickiness half (B5 ship-only stickiness -> universal stickiness); the
  inset-scope half still stands.
- docs/spikes/20260712-215256-combat-travel-lock-separation.md: note that A1
  (non-sticky nav entries) was superseded by the sticky unified list (user
  steer 2026-07-12); Part B/C directions stay future work.
- docs/2026-07-09-component-lock.md: cone range 2000 m -> 20 km
  (`TARGETING_MAX_RANGE`, targeting.rs:119); "nearest AI ship" fallback ->
  hostile-relation gate.
- docs/2026-07-10-signature-lock.md: verify, expected accurate.
- Add/refresh a design doc describing the shipped unified model (list
  membership, ranking, auto-pick policy, stickiness, cycle) as the current
  source of truth, and append the fix records to both parent spikes.

## Notes

- Spike: docs/spikes/20260712-215733-unified-target-computer.md.
- Ordering: last of the three (docs must describe what actually shipped).
