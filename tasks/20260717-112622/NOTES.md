# AI line-of-sight fire gate - design record

Task 20260717-112622, spike tasks/20260717-111808/SPIKE.md (Option B).

## What shipped

`ai_line_of_fire_blocked` in `crates/nova_gameplay/src/input/ai.rs`: an
avian `SpatialQuery::cast_ray_predicate` from a firing position toward the
aim point, with two consumer gates:

- `on_projectile_input` (turrets): runs LAST, only for a shot that passed
  the target/burst/range/alignment gates, so only a shot that would
  otherwise fire pays for the ray. Point defense bypasses the gate.
- `update_torpedo_section_input` (torpedo bays): one ray per ship per frame
  (anchor to anchor), ANDed into the existing launch envelope.

## Decisions and why

- **The ray runs muzzle -> LEADED aim point, not muzzle -> target.** The
  bullet flies to the lead solution; judging occlusion on the raw target
  bearing would wrongly hold fire on crossing targets (the lead offset can
  clear cover the direct bearing does not, and vice versa). A hit that
  resolves to the TARGET's own body is a landing shot, not occlusion.
- **Sensors are transparent to the ray** for the same reason they are
  transparent to rounds: `despawn_bullet_on_hit` skips sensor pairs (the
  R1.1 beacon lesson). A beacon ring or blast shell must not read as cover.
- **The shooter's own colliders are transparent**: the muzzle sits on the
  hull; excluding by `ColliderOf.body == shooter` beats an
  excluded-entities set rebuilt per frame.
- **Unattributable tangible hits fail closed** (counted as blocked): a held
  burst costs a beat; firing through a wall breaks the mechanic's promise.
- **Point defense is exempt**: inbound ordnance hunts THIS ship, its line
  closes by itself, and a wasted round beats a held trigger with a torpedo
  ten meters out. Also keeps PD deterministic under the ordnance's own
  blast sensors crossing the line.
- **Torpedo gate is deliberately conservative**: torpedoes PN-navigate and
  could curve around a rock's edge, but the straight anchor-to-anchor ray
  only ever DELAYS a launch (cadence does not reset on a held launch), it
  never spends a torpedo on cover. Documented in the system doc.
- **No target-drop on occlusion** (out of scope by design): the AI keeps
  tracking, keeps orbiting (AI_ORBIT_SPEED floor keeps it moving), so it
  regains the angle by flying, which reads smarter, not dumber.
- **Symmetry**: the gate lives in AI input only; the player may still
  hold the trigger into a rock (their rounds expend on it, as before).

## Alternatives considered

- Collision-layer masks instead of a predicate: the repo does not author
  `CollisionLayers` anywhere yet; introducing a layer taxonomy for one gate
  is a bigger refactor with mod-facing surface. Predicate + `ColliderOf`
  does the same job with zero content changes.
- Gating target ACQUISITION on LOS: rejected (spike constraint - reads as
  dumber AI, and losing the target drops the orbit that regains the angle).
- Raycast in FixedUpdate next to physics: the whole AI input chain runs in
  Update today; moving one system would split the chain for at most one
  tick of tree staleness, which for a fire gate is a one-frame-late hold
  or release at a cover edge.

## Test rig notes (the exact rig, for the next session)

- `line_of_fire_tests` uses `integrity::test_support::integrity_physics_app`
  (MinimalPlugins + TransformPlugin + AssetPlugin + MeshPlugin +
  PhysicsPlugins, zero gravity, manual 1/60 clock) + `settle()` so avian
  builds REAL collider trees and `ColliderOf` links; the gate is then run
  via `run_system_once(on_projectile_input)` /
  `(update_torpedo_section_input)` and asserted on
  `TurretSectionInput`/`TorpedoSectionInput`.
- Delivery guards: both blocking tests re-assert FIRE after despawning the
  rock in the same test, so a broken rig cannot pass as a working gate.
- The pre-existing bare-`World` rigs gained
  `world.init_resource::<ColliderTrees>()` (empty trees = no occluders);
  `ColliderTrees` is NOT in avian's prelude - import
  `avian3d::collider_tree::ColliderTrees`.
- Sabotage A/B (fix committed first, a45dc244): replacing the ray body with
  `false` turned exactly `cover_between_muzzle_and_target_holds_fire` and
  `cover_holds_the_torpedo_launch` red (2 failed, 3 passed); restore via
  `git checkout`, all 5 green again. The three green-under-sabotage tests
  assert fire-happens, which the gate tests cover from the blocking side.

## Difficulties

- avian's `cast_ray_predicate` doc comment says the ray travels "until the
  predicate returns false", but the implementation SKIPS predicate-false
  colliders and keeps traversing (system_param.rs:176, the
  `return Scalar::MAX` arm). Trust the source, not the doc line.
- `TorpedoSectionConfigHelper` has a private field; production spawns bays
  via the `torpedo_section(config)` bundle helper - the rig now does too
  (reuse-production-helpers-in-tests, again).

## Post-review addenda (Round 1, all findings addressed on APPROVE)

- R1.1: the torpedo-side ray is now memoized behind the cheap per-bay gates
  (cadence + envelope first, ray last, at most one per ship per frame,
  zero while no bay could launch anyway).
- R1.2: CHANGELOG/wiki reworded - the motion that regains the angle is the
  PRE-EXISTING standoff orbit, not new occlusion-driven repositioning; the
  docs now say exactly that. A leashed or orbit-directive ship parked with
  cover dead on the bearing can legitimately hold fire until geometry
  changes; accepted as designed (it still tracks, and the player moving is
  the geometry changing).
- R1.3: own in-flight torpedoes are TANGIBLE bodies, so a launcher's turret
  holds fire while its own just-launched torpedo crosses the muzzle line.
  Accepted and now documented: the hold is transient (the torpedo
  accelerates away on its own guidance) and shooting one's own ordnance in
  the back would be strictly worse. Revisit only if playtests show visible
  trigger flicker (make own-`ProjectileOwner` ordnance transparent then).
