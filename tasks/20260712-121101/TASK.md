# Shakedown Run playtest round 2: gravity reach, bullet knockback, beat-scoped speed cap

- STATUS: OPEN
- PRIORITY: 50
- TAGS: v0.5.0,scenario,bug,feel

## Goal

Fix the user's second playtest round (2026-07-12): the planetoid's
gravity is felt inside the debris field; a single bullet hit sends ships
flying; the training speed cap should release after beacon 1. Verdicts
verbatim in Notes.

## Steps

- [ ] (finding 1) Move the planetoid out of gravity reach of the debris
      field: worst-seed SOI is 8 * 20 * ASTEROID_GEOMETRIC_FACTOR_MAX =
      960u, and the cluster currently sits ~650u from the planetoid -
      inside it on most seeds. New layout: PLANETOID_POS
      (1200, -100, -680) (cluster distance ~1004u), BEACON_3_POS moved
      with it at ~260u on the cluster-facing side (980, -69, -545). Pin
      the invariant in the geometry test: dist(DEBRIS_CENTER, planetoid)
      > 8 * nominal * FACTOR_MAX + margin, citing the exported consts.
      Update the layout comments (the SOI edge is now genuinely crossed
      on the beat-4 leg on every seed).
- [ ] (finding 2) Kill bullet knockback without touching damage: the bcs
      impact-damage observer computes damage from masses and velocities
      (integrity/plugin.rs:113-125) and never reads the solver contact -
      so make turret bullets `Sensor` colliders (no contact resolution,
      no shove, no restitution bounce) and add a despawn-on-first-hit
      observer (a sensor bullet would otherwise cross every section
      collider in its path, dealing damage per crossing; today's solid
      bullet bounces off the first). Torpedoes stay solid (blast + contact
      arming semantics unchanged). Physics test with delivery guards: a
      bullet into a floating hull leaves the hull velocity ~zero AND
      deals damage AND despawns; quantify the pre-fix knockback in the
      test comment (bullet momentum 0.1 * 100 = 10 into a ~4-mass ship
      = ~2.5 u/s per hit before restitution amplification).
- [ ] (finding 3) Beat-scoped speed cap: new
      `EventActionConfig::SetSpeedCap(SetSpeedCapActionConfig { id,
      cap: Option<f32> })` - resolves the scoped ship by EntityId and
      inserts/removes `FlightSpeedCap` (same scoped-only lookup rule as
      DespawnScenarioObject). The beat-1 -> 2 handler adds
      SetSpeedCap(player, None) so the governor releases at beacon 1;
      objective text mentions the governor in b1 ("training governor
      caps your speed") and its release in b2. Walk test asserts the cap
      component exists after boot and is gone after entering beacon 1.
- [ ] Docs (scenario-system.md action list; CHANGELOG entries for all
      three) and the full new-test set + check + fmt WITH --examples
      (the round-1 landing broke example builds because --tests does not
      cover them).

## Notes

- Playtest verdicts (user, 2026-07-12, round 2):
  1. "maybe the planetoid is too close, I can see the gravity effect in
     the debris field; move it further or somehow make it not affect it"
  2. "bullets hitting things (like the enemy) make it fly off like
     crazy, can we do some mass tweaks to make it more stable (doesn't
     make sense that 1 bullet sends you off like that)"
  3. "the rest of the scenario looks good; maybe after beacon 1 we
     disable the speed limitation"
- Verified before planning: only ships/torpedoes carry GravityAffected,
  so finding 1 is the PLAYER being pulled while weaving crates (SOI up
  to 960u vs ~650u separation); bullet knockback is solver contact
  resolution (mass 0.1 at 100 u/s, restitution 0.5) while damage is
  observer-computed - the sensor approach decouples them exactly, which
  beats the user's suggested mass tweak (mass scaling drags damage down
  with it since damage is kinetic).
- Tradeoff to surface in review: sensor bullets no longer physically
  push debris rocks around either. If playtest wants hit-shove juice
  back, add a small authored impulse in the despawn-on-hit observer
  later.
- Follows: 20260712-110730 (round 1, CLOSED, landed d6f4a8c).
