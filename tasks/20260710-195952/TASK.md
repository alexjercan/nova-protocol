# Signature-gated lock: long-range lock only acquires large objects

- STATUS: OPEN
- PRIORITY: 65
- TAGS: v0.5.0, targeting, input, ux

## Goal

User request (2026-07-10): think of the lock as a scanner wave - at long
range only large/important objects return a signature, small ones do not.
Locking the asteroid scene from very far away should acquire only the big
chunky well body (and possibly ships - deferred, see 20260710-195953);
small field rocks and especially battle debris must only become lockable
at very close range. This kills the annoying mid-fight debris locks.

Direction: give the targeting engine (input/targeting.rs,
update_spaceship_target_input) a signature model - each candidate gets a
signature size (asteroid radius / GravityWell body_radius / ship class /
tiny default for debris and small rocks), and its maximum lock range
scales with it (big threshold between classes, per the user: well bodies
lockable from the full TARGETING_MAX_RANGE and beyond-current ranges,
debris only within a stone's throw). The cone pick and the heat-signature
auto-acquire fallback both respect it. Tunables in one reflected tree.

## Notes

- /plan owns the steps when picked up. Likely a pure
  `lock_range_for(signature)` helper + a signature component or derivation,
  with tests mirroring the existing targeting truth-table style.
- The user's scale words ("20 km", "10 meters") map loosely onto world
  units - the point is the class gap, not the absolute numbers; pick
  thresholds relative to TARGETING_MAX_RANGE and playtest.
- Ships at long range: NOT settled - keep current ship lock behavior in
  this task; the sensors/minimap follow-up (20260710-195953) owns that
  question.
- Related: the aim-assist/turret/torpedo feeds consume the same lock;
  gating happens at candidate selection so they inherit it.
