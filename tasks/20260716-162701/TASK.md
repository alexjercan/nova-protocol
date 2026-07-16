# Bug: enemy ship survives at 0 HP as an empty ghost after being shot down (Broadside, once)

- STATUS: OPEN
- PRIORITY: 95
- TAGS: v0.7.0,bug,integrity,scenario


## Report (user playtest, 2026-07-16, Broadside)

Shot down an enemy ship; it "survived" at 0 HP as an empty GHOST ship -
sections gone/dead but the ship still present (targetable, 0 HP readout).
Happened ONCE in the new Broadside scenario; the user could not reproduce
it. Enemy type not recorded (corvette or gunship).

## Why this is worse than cosmetic

Broadside's act machine gates on OnDestroyed: a ghost corvette never sets
its kill flag (gunship never spawns), and a ghost gunship never declares
Victory - either way the scenario SOFT-LOCKS unwinnable. The same class of
ghost would break shakedown's pirate beat and the arena mod's clear gate.

## Diagnostic leads (check, do not assume - diagnostic-first)

- Expected chain: section Health hits zero -> HealthZeroMarker -> integrity
  explode; root death -> IntegrityDestroyMarker -> recursive try_despawn +
  OnDestroyedEvent + debris (crates/nova_gameplay/src/integrity/explode.rs).
  A ghost = sections died without the ROOT death path firing.
- Suspicious edge already documented in code: integrity/glue.rs's test
  comments note EXACT damage vs OVERKILL semantics ("exact damage leaves
  the root alive, while overkill would zero it") - a hit landing exactly at
  the aggregate's remaining HP may leave a zero-HP-but-alive root. Check
  the >= vs > boundary in the root-death gate.
- Related CLOSED history: 20260706-174738 "Sections disable but never
  destroy; ship does not die at zero health" (v0.3.1) - re-read its fix and
  RETRO; this may be a survivor edge of the same family.
- Blast vs kinetic: the kill was player turret fire, but the gunship's
  torpedoes / blast damage may have co-hit; per-collider multi-hit
  (collisionstart-is-per-collider-pair) and damage propagation through
  ChildOf both touch the aggregate math.
- Not reproducible once: build the rig as a property/boundary test around
  the root-death threshold (exact-kill, simultaneous last-section deaths,
  blast+bullet same tick) rather than chasing the live repro; if the
  mechanism is refuted, close as falsification with the rig recorded
  (null-result-becomes-a-pin).

## Notes

- Priority 95 per the v0.7.0 plan's playtest-bug policy (soft-locks the
  flagship scenario when it fires).
- Needs /plan before /work (no Steps yet; the boundary-test rig above is
  the starting shape).

## Steps (planned 20260716, /flow)

- [ ] Record the verified mechanism in NOTES.md: HealthZeroMarker is inserted
      ONLY by bcs on_damage (health/mod.rs; zero writes from
      aggregate_ship_health in integrity/glue.rs carry no marker), bubbles
      are clamped to min(amount, section.current) and swallowed (amount=0.0)
      by already-zero/already-marked nodes, and the root recompute overwrites
      root.current with the section sum every frame - so any path where the
      sum reaches 0 without a qualifying bubble is a permanent unmarked ghost.
- [ ] Boundary rig in crates/nova_gameplay (integrity tests, reuse
      test_support.rs): a production-faithful multi-section ship driven
      through the candidate ghost paths, asserting root despawn +
      OnDestroyed within N frames for each: (a) exact-kill of the last
      section; (b) double-hit on the same section in one tick (per-collider
      multi-hit: second bubble swallowed); (c) fractional-resistance residue
      then kill; (d) last section REMOVED without the damage path (despawn/
      detach - the recompute-only zero); (e) direct-to-root damage
      interleaved with the recompute overwrite. Expect at least (d), likely
      others, to reproduce a live 0-HP root.
- [ ] Fix at the seam that owns the aggregate: aggregate_ship_health (or a
      sibling in the same set) inserts HealthZeroMarker on a ship root whose
      section sum is <= 0 with max > 0 (i.e. it HAD sections and they are
      all dead/gone) and which is not yet marked - making root death
      structural (no living sections) instead of dependent on the last
      bubble's arithmetic. Keep the bubble path; the structural check is the
      backstop. Pin at this boundary (unit-level), not only e2e.
- [ ] Fail-first A/B: commit the fix, then run the rig against the pre-fix
      code (revert/sabotage per commit-before-sabotage) and record which
      cases go red and their numbers in TASK.md.
- [ ] Keep ALL rig cases as regression tests, including the ones that never
      reproduced (they pin the non-behavior; null-result-becomes-a-pin).
- [ ] Sweep the consumers: the player root rides the same chain (Defeat
      overlay), shakedown pirate / broadside corvettes+gunship / arena gates
      all key on OnDestroyed - confirm the fix path fires OnDestroyed
      exactly once (no double-destroy from marker + bubble racing;
      count-gate lesson). Check the HUD health readout rounding while there:
      if a fractional residue can display as "0" on a живой ship, file it
      separately, do not widen.
- [ ] Verify: fmt/check --all-targets, new tests + integrity/glue suites,
      one live example-19 walk; CHANGELOG [Unreleased] Fixes line; close.

## Planning notes (verified in source, 2026-07-16)

- bcs health/mod.rs on_damage: applies min(amount, current), mutates
  damage.amount to the applied value for the ChildOf propagation, inserts
  HealthZeroMarker at <= 0, and EARLY-RETURNS with amount=0.0 on
  already-marked or already-zero nodes (swallowing the bubble).
- integrity/glue.rs aggregate_ship_health: recomputes root Health =
  sum(section children) every frame, ships only; its doc admits root death
  leans on the bubbled fatal hit reaching the root with a nonzero amount.
- integrity/explode.rs: IntegrityDestroyMarker -> fragments/despawn +
  OnDestroyed fire; meshless entities (root, sections) despawn directly.
- The ghost is at the JOINT of these: recompute-zero without marker.
