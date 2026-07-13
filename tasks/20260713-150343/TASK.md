# Shakedown playtest fixes: bigger coast ring, longer derelict lock range, derelict-kill softlock

- STATUS: CLOSED
- PRIORITY: 50
- TAGS: v0.5.0,scenario,bugfix,playtest

## Outcome (CLOSED 2026-07-13)

Playtest round on beat sheet v2 (user, 2026-07-13; direct on master):

- **Softlock FIXED** ("I got stuck after destroying the derelict"): the
  kill handler was gated eq(beat 11) but the hulk is destructible from
  beat 9 - the player shot it before the combat-lock lesson ticked (root
  cause chained to the lock range below) and the death was consumed as a
  no-op. The handler is now a catch-all (lt 12) that completes
  B9/B10/B11 (absent completes are no-op removals), clears the RADAR
  emphasis defensively, and jumps to the fight - lessons complete by
  demonstration, never dead-end. Pinned by
  `an_early_derelict_kill_skips_to_the_fight` (kill at beat 10 without a
  lock -> B12 up, skipped lessons completed, emphasis retired).
- **Derelict lockable from afar**: AsteroidConfig gained a
  `lock_signature` override (None = radius, the old behavior; all 12
  construction sites updated); the derelict authors 15.0 -> 450u combat
  lock range (was the size-derived ~75u that caused the softlock chain).
- **Coast ring bigger**: 210 -> 300u. Enabled by a MECHANISM DISCOVERY:
  a spawned area DOES fire OnEnter for bodies already inside it - but
  ONLY with the full production bundle; during the discovery test a
  Collider without a RigidBody registered no contact pair at all,
  silently (three false hypotheses fell first: rig without a manual
  clock, rig poisoned by a message-less plugin panic, pair ordering -
  the A/B showed ordering was fine). Pinned in nova_scenario
  objects/area.rs; the old trigger-clearance pin is replaced by
  "nominal park outside the ring" (300 < 350) with spawn-inside as the
  safety net.

Verified: 17 nova_assets tests (incl. the new skip-path pin), 39
nova_scenario (incl. the new area pin), 471 nova_gameplay, fmt +
workspace check clean. Autopilots deferred (user's game instance
running).

## Notes

- Emphasis-pairing pin relaxed: clears may exceed sets (the catch-all's
  defensive clear); every set verb still has a downstream clear.
- The ring/park geometry: nominal park ~350u, ring 300u -> ~50u coast
  (was ~140u of dead air).
