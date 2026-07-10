# Signature-gated lock: long-range lock only acquires large objects

- TASK: 20260710-195952
- MODULE: crates/nova_gameplay/src/input/targeting.rs (+ asteroid
  authoring in nova_scenario)

## What was built

The scanner-wave lock model (user request 2026-07-10): a candidate's
maximum lock range now depends on what the scanner "sees".

- Full-range classes (up to TARGETING_MAX_RANGE): gravity well bodies
  (the big chunky boy locks from across the field) and ships
  (deliberately unchanged - the sensors/minimap task 20260710-195953 owns
  that question). Committed torpedoes get their own generous
  `torpedo_lock_range` (2500u - small but hot: covers every real
  point-defense engagement without being visible across the map; review
  R1.3 pulled them out of the full-range exemption to keep the scanner
  fiction honest).
- Everything else is gated by a new `LockSignature(f32)` component, a
  radius-like magnitude: lock range = signature *
  `TargetingSettings::signature_range_per_unit` (default 30/unit).
  Asteroids author it from their radius, so 1-3u field rocks lock only
  within 30-90u; the signed range floors at the unsigned debris range so
  an authored signature can never make a body stealthier than none.
- Bodies with no signature at all - battle debris, fragments - fall to
  `TargetingSettings::unsigned_lock_range` (default 15u): point-blank
  only, which kills the annoying mid-fight debris locks.

The gate sits at candidate collection in update_spaceship_target_input,
so the aim cone pick, the heat-signature auto-acquire fallback, and every
downstream consumer of the lock (torpedo commit, turret feed, GOTO
designation, HUD reticle) inherit it with zero changes.

- The incumbent lock holds to `range_hysteresis` (1.15x) beyond its
  gate, so a body at its boundary cannot strobe the lock (and reset the
  1.5s focus dwell) as the ship drifts; fresh acquisition uses the plain
  gate.

## Decisions

- Component + settings over hardcoded classes: the scenario layer authors
  what the scanner sees; the two knobs live in a reflected
  TargetingSettings resource per the settings-tree convention.
- The user's absolute scale words ("20 km", "10 m") were mapped to class
  gaps relative to the existing ranges, per the task notes; defaults are
  playtest knobs.
- Two pre-existing tests spawned bare dynamic bodies as generic lockables;
  under the new model that means debris, so they were updated to carry
  signatures - a deliberate reflection of the new semantics, not a
  weakening (the new truth-table tests pin both sides of the gate).

## Verification

- 6 new targeting tests (signed rock far/near, unsigned debris far/
  point-blank, all three intrinsic classes at 5000u) + 25 existing
  targeting tests green; asteroid bundle test asserts the authored
  signature; input module (114) green; fmt + check --workspace
  --examples clean. Full suite and clippy on CI.

## Difficulties

None; the collection-point gate meant no picker or consumer changed.

## Self-reflection

- The candidate filter_map keeps absorbing policy (dynamic-or-well, now
  signatures) cleanly, but it is on its way to being a policy function of
  its own; if one more rule lands there, extract a pure
  `lock_candidate(...)` helper with a truth-table test.
