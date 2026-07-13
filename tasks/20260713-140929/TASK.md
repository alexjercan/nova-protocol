# Shakedown beat sheet v2: one-line objectives, beacon 4, coast ring, derelict rehearsal

- STATUS: CLOSED
- PRIORITY: 42
- TAGS: v0.5.0,scenario,tutorial,spike

## Outcome (CLOSED 2026-07-13)

Shipped per plan; the run is now 12 one-line beats. Notes:

- GEOMETRY RESHUFFLE the plan's verify-first step forced: beacon 3 (the
  FIRST lock target) moved OUTSIDE the largest SOI at (600,90,120) - the
  old inside-SOI spot became BEACON 4, scaled out to 300u so its trigger
  clears the coast ring (the already-inside trap, now pinned). The
  waypoint leg (~800u) exceeds the default beacon lock range (600u), so
  BeaconConfig gained a `lock_signature` override (nova_scenario, with its
  own unit test) and beacon 4 authors 30.0 (900u); the leg-vs-range pin
  lives in the geometry test.
- The break-away beat ([Z]) came free: OnExit(coast_ring) is an existing
  event - the ring serves both the coast (enter) and the break-away
  (exit), and the orbit always stays inside it (pinned).
- The coast ring spawns WITH its beat (never at start), so its OnEnter
  cannot fire before the beat arms.
- The derelict is an asteroid-kind hulk ("Derelict Hulk", r 2.5, hp 150)
  outside the largest SOI (a dynamic body inside would fall in - pinned).
- The walk covers all 12 beats through the real pipeline, including the
  stale-lock-echo no-op (a re-fired OnTravelLock(beacon_3) during beat 5
  does not move the beat) and the early-pirate-kill guard re-homed to the
  rehearsal beat.
- Emphasis sequences pinned exactly: set [RADAR, GOTO, RADAR], cleared
  [RADAR, GOTO, RADAR].

Verified: 16 nova_assets tests + nova_scenario beacon override test; fmt +
workspace check clean. The 03_scenario autopilot run is DEFERRED (the
user's game instance is running; contention flake documented in
20260713-124000) - the walk exercises the full event pipeline headlessly.

## Goal

Playtest (2026-07-13): objectives carry too much text. Restructure the
Shakedown Run to the spike's beat sheet v2: ~11 one-line beats (ONE gesture
per objective, <= ~15 words), beacon 4 (the waypoint/re-designation leg),
the gravity-coast ring (zero-key scenic SOI beat), the derelict live-fire
rehearsal before the scavenger, and the fight text collapsing to one line.
LOCK stays withheld through beats 1-3; the grant stays with its lesson.

## Steps

- [x] Content constants (shakedown.rs): BEACON_4 position on the
      beacon-3 -> planetoid approach, the coast-ring center/radius, the
      derelict position + config. VERIFY-FIRST geometry: derive all three
      against the worst/min-seed SOI numbers inside
      `beat4_geometry_holds_across_the_derived_radius_range` (extend that
      test to pin: beacon 4 + derelict OUTSIDE the worst-seed SOI so
      nothing falls or fights gravity; the ring INSIDE the min-seed SOI so
      the coast is felt on every seed, and outside the widest orbit ring).
- [x] The derelict is an ASTEROID-kind object named "Derelict Hulk"
      (radius ~2.0, health ~150, no gravity, not invulnerable) - zero new
      spawn paths; asteroids lock, zoom in the viewfinder, and die. The
      inert-ship silhouette is recorded as future polish, not built.
- [x] The coast ring is a `CreateScenarioArea` action (EXISTS -
      actions.rs:35, used in nova_assets/scenario.rs:375): invisible
      trigger, id "coast_ring"; fires OnEnter like any area.
- [x] Rewrite the event list to the spike's beats 1-11: new OBJ ids +
      VAR_BEAT values; lazily-spawned content (beacon 4 with its beat, the
      derelict with the rehearsal beat); the beat-4 handler keeps the LOCK
      grant; gates: OnTravelLock(beacon_3) for the first-lock beat,
      OnEnter(beacon_3) for hands-off, OnEnter(beacon_4) for the waypoint
      run, OnEnter(coast_ring) for the coast, OnOrbit for orbit,
      OnCombatLock(derelict) then OnDestroyed(derelict) for the rehearsal,
      OnDestroyed(pirate) for the fight; texts verbatim from the spike
      sheet.
- [x] Marker hand-offs per leg (beacon 3 -> beacon 4 -> planetoid ->
      derelict -> scavenger) and emphasis pairs: RADAR set at the
      first-lock beat / cleared by its OnTravelLock handler; GOTO set at
      hands-off / cleared at the coast beat; RADAR set again at the
      rehearsal / cleared by its OnCombatLock handler. The pairing test
      pins the set/cleared sequences exactly.
- [x] Rewrite the pinned tests: the beat-walk (a `lock(app, id, combat)`
      helper firing the new events like enter/orbit; the capability pins
      stay - withheld at boot, granted with its beat; a STALE-LOCK pin:
      firing OnTravelLock(beacon_3) again during the waypoint beat is a
      no-op - the beat guards own ordering); every_referenced_id + the
      marker hand-off test extended to the new legs; tally and
      death-restart tests unchanged.
- [x] Docs: CHANGELOG entry; fix records in spike 20260713-140742 (both
      tasks).
- [x] fmt + check; nova_assets suite; 03_scenario autopilot (defer if the
      user's game instance is running - contention flake documented in
      20260713-124000).

## Notes

- Spike: tasks/20260713-140742/SPIKE.md (beat
  sheet + design rules: one gesture per objective, failure-free new
  beats).
- Depends on: 20260713-140922 (OnTravelLock/OnCombatLock events).
- Positions on file for the derivation: PLANETOID_POS (1240,-105,-700),
  BEACON_3_POS (1019,-74,-566), DEBRIS_CENTER (350,20,-160), PIRATE_SPAWN
  (380,40,-100).
- The scavenger reveal moves from orbit completion to rehearsal
  completion - verify the reveal ordering reads well; the marker jump
  covers wayfinding back to the field.
