# Shakedown beat sheet v2: one-line objectives, beacon 4, coast ring, derelict rehearsal

- STATUS: OPEN
- PRIORITY: 42
- TAGS: v0.5.0,scenario,tutorial,spike

## Goal

Playtest (2026-07-13): objectives carry too much text. Restructure the
Shakedown Run to the spike's beat sheet v2: ten one-line beats (ONE gesture
per objective, <= ~15 words), beacon 4 on the planetoid approach (the
waypoint/re-designation leg), the gravity-coast ring (zero-key scenic SOI
beat), the derelict live-fire rehearsal (combat lock + viewfinder + fire in
calm, before the scavenger), and the fight text collapsing to one line.
LOCK stays withheld through beats 1-3; the capability grant moves with its
lesson.

## Notes

- Spike: docs/spikes/20260713-140742-shakedown-beat-sheet-v2.md (the full
  beat sheet + design rules: one gesture per objective, failure-free new
  beats).
- Depends on: 20260713-140922 (OnLock event - beats 4, 6 gating, 9a).
- Open questions the plan must verify first: the derelict's body kind
  (inert sections-ship vs named rock), the invisible coast-ring area kind,
  beacon-4/ring/derelict geometry vs the worst-seed SOI (extend
  beat4_geometry_holds_across_the_derived_radius_range), OnLock leftover
  semantics in the scripted walk (staged double-designation pin).
- Emphasis re-pairing and the beat-walk rewrite come with the split.
- /plan before implementation.
