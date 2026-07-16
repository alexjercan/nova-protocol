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
