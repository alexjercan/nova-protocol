# Signature-gated lock: long-range lock only acquires large objects

- STATUS: CLOSED
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

## Steps

- [x] `LockSignature(pub f32)` component (reflected) in input/targeting.rs
      - a radius-like magnitude the lock scanner "sees"; exported through
      the prelude so the scenario layer can author it.
- [x] Reflected `TargetingSettings` resource with the two knobs:
      `signature_range_per_unit` (lock range per signature unit, ~30) and
      `unsigned_lock_range` (the point-blank range for bodies with no
      signature at all - debris; ~15u). init_resource + register_type in
      SpaceshipTargetingPlugin.
- [x] Gate at candidate collection in update_spaceship_target_input (so
      the cone pick AND the heat-signature fallback both inherit it): a
      candidate's max lock range is TARGETING_MAX_RANGE for well bodies,
      ships, and committed torpedoes (ships deferred to 20260710-195953;
      torpedoes stay lockable for point defense), else
      signature * signature_range_per_unit, else unsigned_lock_range;
      drop candidates beyond their range from the cone origin.
- [x] Author signatures: asteroid_scenario_object inserts
      LockSignature(radius) (nova_scenario), so field rocks (1-3u) lock
      only within ~30-90u; the Gravity Rock is range-free via its well
      anyway. Debris/fragments/sections get nothing and fall to the
      point-blank default.
- [x] Tests (targeting truth-table style): small signed rock unlockable
      at range / lockable close; unsigned debris unlockable at 50u /
      lockable point-blank; well body and ship and committed torpedo
      lockable at long range (unchanged); asteroid bundle carries the
      signature (nova_scenario test).
- [x] fmt + check --workspace --examples + affected modules (input, hud
      consumers compile); document in tasks/20260710-195952/NOTES.md.

## Notes

- Planned 2026-07-10 (steps above). The gate lives in the candidate
  filter_map, not pick_target, so both pickers inherit it and the helpers
  stay untouched.
- The user's scale words ("20 km", "10 meters") map loosely onto world
  units - the point is the class gap, not the absolute numbers; pick
  thresholds relative to TARGETING_MAX_RANGE and playtest.
- Ships at long range: NOT settled - keep current ship lock behavior in
  this task; the sensors/minimap follow-up (20260710-195953) owns that
  question.
- Related: the aim-assist/turret/torpedo feeds consume the same lock;
  gating happens at candidate selection so they inherit it.

## Resolution

Shipped per plan: LockSignature + TargetingSettings (30/unit, 15u
unsigned), the gate at candidate collection (cone pick + fallback + all
lock consumers inherit), asteroids author signature = radius. Ships and
committed torpedoes deliberately keep full range (sensors task owns
ships; point defense needs torpedoes). 3 new + 2 adjusted targeting
tests, scenario assert; all affected modules green; fmt + check
--workspace --examples clean. Details: tasks/20260710-195952/NOTES.md.
