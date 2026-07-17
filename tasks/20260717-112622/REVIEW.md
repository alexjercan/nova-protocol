# Review: AI line-of-sight fire gate

- TASK: 20260717-112622
- BRANCH: work/ai-los-fire-gate

## Round 1

- VERDICT: APPROVE

- [ ] R1.1 (MINOR) crates/nova_gameplay/src/input/ai.rs:1667 - The torpedo-side
  ray is cast eagerly for every engaged AI ship with a ship target, before the
  bay loop runs: a ship with ZERO torpedo bays, or with every bay still on its
  launch cadence / out of envelope, pays one raycast per frame for nothing.
  The system doc's "one ray per ship" is technically honest, but the task's
  own perf direction ("per firing turret, not per frame per turret if
  avoidable") argues for laziness here the same way the turret gate got it
  (ray last, only for a shot that would otherwise fire). Suggest memoizing
  per ship inside the bay loop (e.g. `let mut line_clear: Option<bool> =
  None;` filled only after `engaged && cooldown.is_finished() && envelope`
  pass), so ships that cannot launch this frame cast nothing.
  - Response: fixed - the ray is now a memoized closure evaluated LAST in
    the launch chain (cadence + envelope first), so a frame with no
    launchable bay casts nothing and multiple bays still share one ray.

- [ ] R1.2 (MINOR) CHANGELOG.md:17, web/src/wiki/combat-weapons.md:33 - Docs
  overclaim the maneuvering half: "holds fire and maneuvers for a clear
  angle" reads as occlusion-driven repositioning, but no code in this change
  repositions anything - the motion is the pre-existing standoff orbit,
  which runs whether or not the line is blocked (and a leashed or
  orbit-directive ship may never regain the angle at all; it just goes
  quiet). The task Goal is honest about this ("uses its existing
  approach/orbit machinery"), the player-facing text is not. Suggest
  rewording to something like "holds fire while its usual maneuvering brings
  the angle back" in both places, so the docs describe emergent behavior
  instead of implying a new reposition feature.
  - Response: fixed - both texts now attribute the motion to the normal
    attack orbit ("its normal attack orbit keeps it circling, so the
    pressure resumes only once that motion brings the angle back").

- [ ] R1.3 (NIT) crates/nova_gameplay/src/input/ai.rs:1425-1435 - The AI's
  own in-flight torpedoes are tangible dynamic bodies whose `ColliderOf`
  resolves to the torpedo root (neither shooter nor target), so a
  just-launched torpedo chasing the same target can intermittently read as
  cover and hold the launcher's turret fire until it clears the line. That
  is arguably the right outcome (pre-change, those rounds would expend on
  and could kill the ship's own ordnance), but the interaction is nowhere in
  NOTES.md's decision record. Suggest one line in NOTES.md documenting it as
  accepted (or, if playtests show it flickering badly, treating colliders
  whose `ProjectileOwner` is the shooter as transparent like sensors).
  - Response: documented in NOTES.md (post-review addenda): accepted as
    designed, transient by construction; the ProjectileOwner-transparency
    escape hatch is recorded there for if playtests show trigger flicker.

### Verification record (what this review re-derived, not trusted)

- avian 0.7 `cast_ray_predicate` semantics (claim a): read
  avian3d-0.7.0/src/spatial_query/system_param.rs:176-225 directly.
  Predicate-false makes the traversal callback `return Scalar::MAX` (skip
  this collider, keep traversing); a passing hit shrinks `max_distance =
  distance` so the closest passing hit wins. The crate's own doc line ("ray
  keeps travelling until the predicate returns false") is wrong; the code
  matches what the gate relies on. NOTES.md's account is accurate.
- ColliderOf body resolution (claim b): production ships spawn the root with
  `RigidBody::Dynamic` via `base_scenario_object`
  (crates/nova_scenario/src/actions.rs:932-948); sections are direct
  children carrying only a `Collider` (`base_section`,
  crates/nova_gameplay/src/sections/base_section.rs:62-74;
  `destructible_body` is Health + ColliderDensity + Visibility, no
  RigidBody). avian's ColliderOf (collider_hierarchy/mod.rs) attaches every
  collider to the nearest ancestor rigid body, with `ALLOW_SELF_REFERENTIAL
  = true` for collider-on-body entities. So every section collider resolves
  to the SHIP ROOT that AITarget stores: the target-hit check and the
  shooter exclusion are both sound, including the muzzle-inside-own-hull
  case under `solid = true`.
- Gate placement: read the final code, not the description - the turret ray
  runs only after target/burst, range and alignment gates pass
  (ai.rs:1480-1537), and `defending ||` short-circuits so PD never casts.
  No allocation in the hot path (`SpatialQueryFilter::default()` builds an
  empty EntityHashSet, which does not allocate until first insert; the
  predicate is a `&dyn Fn` closure over queries).
- Rig sweep: grepped the whole workspace - `run_system_once` of the two
  systems exists only in ai.rs; all four bare-World rig constructors got
  `ColliderTrees` (behavior_state_tests:2096, firing_world:3314,
  defended_world:3519, torpedo_world:4421); `line_of_fire_tests` uses the
  real physics app. No other crate or example references the systems or the
  AI input plugin directly, and `NovaGameplayPlugin` adds `PhysicsPlugins`
  itself (plugin.rs:36), so no App-based rig can panic on the new resource.
- Tests: `cargo test -p nova_gameplay input::ai::` run from the worktree:
  `test result: ok. 88 passed; 0 failed; 0 ignored; 0 measured; 453 filtered
  out; finished in 1.21s` - matches the close-out claim. The two blocking
  tests (turret + torpedo) go red with the gate deleted and carry same-test
  delivery guards (fire re-asserted after the rock despawns); the sabotage
  record in NOTES.md (2 red / 3 green under a neutered ray) is consistent
  with which tests assert the blocking side.
- `cargo fmt --check`: clean. `cargo check --workspace --all-targets`:
  green (one pre-existing future-incompat note in proc-macro-error2, a
  dependency, unrelated). Full suite left to CI per standing instruction.
- Edge cases walked: muzzle inside own collider (excluded via ColliderOf),
  target with no remaining colliders (fires unless a third body blocks),
  target despawned mid-frame (turret: anchor resolution fails, input false;
  torpedo: `q_ship_root.contains` filter drops it, no panic), sensors
  transparent (bullets are Sensor colliders, so own rounds in flight never
  block), degenerate aim-on-muzzle ray (Dir3::new Err -> not blocked).
- Constraint check: aim, lead and decision systems untouched; the player's
  trigger path has no gate (symmetry stays authored-side); PD exempt; no
  damage numbers changed anywhere in the diff.
