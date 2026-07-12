# Bullets affected by gravity wells

- STATUS: OPEN
- PRIORITY: 40
- TAGS: feature,gameplay,spike,v0.5.0

Make turret rounds (bullets) feel gravity wells, the same way ships and
torpedoes already do. Today only ship roots and torpedo projectiles opt into
`GravityAffected`; turret rounds and section debris deliberately skip it.

## Spike outcome (read first)

Spike: docs/spikes/20260712-112113-bullets-affected-by-gravity.md
(RECOMMENDED - conditional). It measured the curvature and settled the scope:

- Curvature is perceptible ONLY on close grazing passes near a strong well
  (~2-4u miss); at typical combat geometry (b >= 50u) it is sub-degree / ~1u -
  the original "imperceptible" call was mostly right.
- Correctness is subtle and may be a NET WIN. The turret already aims behind
  the target in a well today: the target is `GravityAffected`, so it is
  accelerating while the lead solver assumes constant velocity. A bullet and
  target near the same point share common-mode acceleration that largely
  cancels in the relative frame, so bullet gravity can REDUCE that existing
  miss (only first-order - degrades when they sit at very different radii).
- Decision: build **Option C1** - opt bullets in, add a measured perf guard,
  and OBSERVE net PDC accuracy in wells (don't assume it worsens). Bullets
  only; debris stays deferred. If the free common-mode cancellation is not
  enough in playtest, build the follow-up C2 (full gravity feedforward in the
  intercept solve for BOTH target and bullet) - user is willing to fund it.

## Steps (for /plan to expand)

- Insert `GravityAffected` on turret rounds at spawn: a third observer on
  `TurretBulletProjectileMarker`, mirroring `insert_gravity_affected_on_torpedo`
  in `crates/nova_gameplay/src/gravity.rs`. Round spawning lives in
  `crates/nova_gameplay/src/sections/turret_section.rs`.
- Measure the per-frame cost with a full PDC stream near a well: add a well to
  `examples/08_turret_range.rs` and check frame cost with ~500-2000 live
  rounds. Only then decide the perf guard's weight.
- Add the perf guard the measurement warrants: make out-of-SOI rounds cheap
  (skip the per-entity Vec/well loop or a coarse SOI broadphase). Bullets need
  no `DominantWell`/hysteresis, so a lighter path than the ship force loop is
  fine.
- During playtest, measure net PDC accuracy in a well before/after the change:
  baseline already aims behind the falling target, and bullet gravity may
  cancel some of that. Document the outcome; if it does not net out, that is
  the trigger for the C2 follow-up.
- Update the `GravityAffected` doc comment (it currently states rounds skip v1)
  so it stays accurate.

Out of scope: debris gravity; the gravity-aware turret intercept term (C2) -
create that as its own follow-up if a playtest asks for it.
