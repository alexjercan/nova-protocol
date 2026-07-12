# Review: Shakedown Run playtest round 2

- TASK: 20260712-121101
- BRANCH: fix/shakedown-playtest-2 (commit 0985470 vs master)

## Round 1

- VERDICT: REQUEST_CHANGES (fresh-context agent review)

- [x] R1.1 (BLOCKER) turret_section.rs despawn_bullet_on_hit - trigger
  volumes silently ate bullets: scenario areas/beacon spheres are Sensor
  colliders with events enabled, avian raises CollisionStart for
  sensor-sensor pairs, and the observer despawned any bullet in the
  pair. Pirate patrol waypoint 1 sits inside beacon 2's 70u trigger:
  un-hittable there, rounds vanish with zero feedback.
  - Response: fixed - the observer skips the pair when the OTHER
    collider is itself a Sensor (a trigger/blast volume is two
    intangibles crossing); covered by
    bullets_ignore_trigger_volumes_and_stop_at_event_less_solids.
- [x] R1.2 (MAJOR) bullets tunneled through invulnerable planetoids: no
  Health node means bcs never enables collision events on the body, the
  bullet had none either, so bullet-vs-planetoid raised no event at all
  - no despawn, no solidity, rounds crossed solid cover.
  - Response: fixed - bullets carry their own CollisionEventsEnabled;
    with the R1.1 sensor-skip, areas still do not expend rounds. Same
    test covers the event-less-solid stop.
- [x] R1.3 (MINOR) geometry margins razor-thin (cluster 3.6u of slack,
  crate_3 zero margin).
  - Response: fixed - planetoid pushed to (1240,-105,-700) (cluster
    ~1048u vs 1000 threshold), beacon 3 moved with it (1019,-74,-566,
    still 260u out), and the per-crate assert gained the +40 margin.
- [x] R1.4 (NIT) missing test for the trigger/tunnel interplay; comment
  overclaimed the pre-fix behavior.
  - Response: fixed - the new blind-spot test covers both cases; the
    observer comment rewritten (tangible-contact semantics, both review
    findings cited).

Reviewer verified clean: all prior geometry invariants at the new
positions (beacon 3 at 260u: inside half-smallest-SOI, outside widest
ring, clears surface; pirate spawn/patrol 1014-1041u, outside worst
SOI); damage-before-despawn ordering (deferred command flush);
ProjectileHooks own-ship filtering independent of sensor status; point
defense intact (torpedo sections carry Health, so events fire; bullets
still kill torpedoes); TempEntity cleanup for misses; muzzle/audio
effects unaffected; SetSpeedCap lookup + walk-test lifecycle real;
checks green incl. --examples; no non-ASCII; TASK.md honest about the
debris-shove tradeoff.

## Round 2

- VERDICT: APPROVE

All four fixes verified in the files: pair-orientation logic correct
(bullet-vs-bullet leaves both alive; body-less collider pairs skipped);
bullet-carried events close the tunnel without re-opening the trigger
eat; geometry re-derived with real margin (cluster 1048.5u vs 1000,
worst crate slack 23.6u, beacon 3 at 260.3u inside all bands); the
blind-spot test's two phases each fail against their respective pre-fix
code (timing walked frame by frame).
