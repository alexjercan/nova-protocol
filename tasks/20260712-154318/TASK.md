# Lock reticle on beacons sizes to the trigger sensor - exclude sensors from ApparentSize

- STATUS: CLOSED
- PRIORITY: 40
- TAGS: v0.5.0,hud,polish,playtest

## Goal

Playtest feedback 2026-07-12 (same round as 20260712-152340): "beacons
have a really big target thingy - I would make it smaller." Diagnosis
(verified in code): locking a beacon puts the lock reticle on it
(torpedo_target.rs, ApparentSize { min_px: 32 }), and ApparentSize unions
the anchor subtree's ColliderAabbs - a beacon's ONLY collider is its
trigger sensor sphere (70u in shakedown, objects/beacon.rs:125), ~10x the
visible 2u orb. Same bug class as 20260712-093831 review R1.1 (the crate
bracket), on the remaining ApparentSize consumer. Fix it at the source:
sensors are invisible volumes and must not count as "apparent" size.

## Steps

- [x] In hud/screen_indicator.rs, exclude Sensor colliders from the
      ApparentSize AABB union (q_aabb becomes
      `Query<&ColliderAabb, Without<Sensor>>` for target_world_aabb and
      its caller). A sensor-only subtree then has no AABB and falls back
      to min_px - the intended small reticle. Document the rule on the
      ApparentSize variant.
- [x] Check the other ApparentSize consumers keep their behavior: ships
      (torpedo reticle, candidate brackets) measure hull-section solid
      colliders, unaffected; sweep for anchored entities mixing Sensor +
      solid colliders.
- [x] Tests: sensor-only anchor yields no AABB (locked-beacon case pins
      the min_px fallback) with a solid-collider delivery guard on the
      same rig; mixed subtree unions only the solid part; existing
      ship-shaped tests stay green.
- [x] Verify: cargo fmt + check --workspace --all-targets + the
      screen_indicator suite.

## Notes

- Playtest verdict recorded with the gold-text one in spike
  docs/spikes/20260712-140842-objective-conveyance-visuals.md.
- Same-class precedent: tasks/20260712-093831/REVIEW.md R1.1 (the crate
  bracket went to an authored WorldRadius; here the fix is the generic
  sensor exclusion, because the reticle must keep tracking SHIP hulls).
- Beacons stay lockable (LockSignature) - only the reticle SIZE changes.

## Close record

What changed: target_world_aabb and its callers query
`ColliderAabb Without<Sensor>` (screen_indicator.rs), the ApparentSize
variant doc states the rule, CHANGELOG entry. New test
target_world_aabb_ignores_sensor_colliders: sensor-only body -> None
(min_px fallback), mixed subtree unions only the solid part, and a
strip-the-Sensor delivery guard proves the query shape does the
excluding. Sweep: ships/asteroids anchor solid colliders (unchanged);
turret muzzle/bullet sensors are not ship children in anchored
subtrees, and excluding any such sensor from a mixed union is the
desired semantics anyway; the crate bracket already moved to authored
WorldRadius last cycle.

Alternatives considered: per-consumer overrides or an authored
WorldRadius on the reticle (rejected - the reticle must keep tracking
arbitrary lock targets, and the semantic bug is in the union itself).

Self-reflection: R1.1 of 20260712-093831 analyzed exactly this
exclusion and deferred it because "nothing else uses ApparentSize on a
sensor-only entity" - but beacons are deliberately lockable, so the
reticle-on-beacon path was reachable all along. When deferring a
generic fix because current consumers look safe, enumerate the
ANCHORABLE entities, not the consumer list.
