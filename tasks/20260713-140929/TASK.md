# Shakedown beat sheet v2: one-line objectives, beacon 4, coast ring, derelict rehearsal

- STATUS: OPEN
- PRIORITY: 42
- TAGS: v0.5.0,scenario,tutorial,spike

## Goal

Playtest (2026-07-13): objectives carry too much text. Restructure the
Shakedown Run to the spike's beat sheet v2: ~11 one-line beats (ONE gesture
per objective, <= ~15 words), beacon 4 (the waypoint/re-designation leg),
the gravity-coast ring (zero-key scenic SOI beat), the derelict live-fire
rehearsal before the scavenger, and the fight text collapsing to one line.
LOCK stays withheld through beats 1-3; the grant stays with its lesson.

## Steps

- [ ] Content constants (shakedown.rs): BEACON_4 position on the
      beacon-3 -> planetoid approach, the coast-ring center/radius, the
      derelict position + config. VERIFY-FIRST geometry: derive all three
      against the worst/min-seed SOI numbers inside
      `beat4_geometry_holds_across_the_derived_radius_range` (extend that
      test to pin: beacon 4 + derelict OUTSIDE the worst-seed SOI so
      nothing falls or fights gravity; the ring INSIDE the min-seed SOI so
      the coast is felt on every seed, and outside the widest orbit ring).
- [ ] The derelict is an ASTEROID-kind object named "Derelict Hulk"
      (radius ~2.0, health ~150, no gravity, not invulnerable) - zero new
      spawn paths; asteroids lock, zoom in the viewfinder, and die. The
      inert-ship silhouette is recorded as future polish, not built.
- [ ] The coast ring is a `CreateScenarioArea` action (EXISTS -
      actions.rs:35, used in nova_assets/scenario.rs:375): invisible
      trigger, id "coast_ring"; fires OnEnter like any area.
- [ ] Rewrite the event list to the spike's beats 1-11: new OBJ ids +
      VAR_BEAT values; lazily-spawned content (beacon 4 with its beat, the
      derelict with the rehearsal beat); the beat-4 handler keeps the LOCK
      grant; gates: OnTravelLock(beacon_3) for the first-lock beat,
      OnEnter(beacon_3) for hands-off, OnEnter(beacon_4) for the waypoint
      run, OnEnter(coast_ring) for the coast, OnOrbit for orbit,
      OnCombatLock(derelict) then OnDestroyed(derelict) for the rehearsal,
      OnDestroyed(pirate) for the fight; texts verbatim from the spike
      sheet.
- [ ] Marker hand-offs per leg (beacon 3 -> beacon 4 -> planetoid ->
      derelict -> scavenger) and emphasis pairs: RADAR set at the
      first-lock beat / cleared by its OnTravelLock handler; GOTO set at
      hands-off / cleared at the coast beat; RADAR set again at the
      rehearsal / cleared by its OnCombatLock handler. The pairing test
      pins the set/cleared sequences exactly.
- [ ] Rewrite the pinned tests: the beat-walk (a `lock(app, id, combat)`
      helper firing the new events like enter/orbit; the capability pins
      stay - withheld at boot, granted with its beat; a STALE-LOCK pin:
      firing OnTravelLock(beacon_3) again during the waypoint beat is a
      no-op - the beat guards own ordering); every_referenced_id + the
      marker hand-off test extended to the new legs; tally and
      death-restart tests unchanged.
- [ ] Docs: CHANGELOG entry; fix records in spike 20260713-140742 (both
      tasks).
- [ ] fmt + check; nova_assets suite; 03_scenario autopilot (defer if the
      user's game instance is running - contention flake documented in
      20260713-124000).

## Notes

- Spike: docs/spikes/20260713-140742-shakedown-beat-sheet-v2.md (beat
  sheet + design rules: one gesture per objective, failure-free new
  beats).
- Depends on: 20260713-140922 (OnTravelLock/OnCombatLock events).
- Positions on file for the derivation: PLANETOID_POS (1240,-105,-700),
  BEACON_3_POS (1019,-74,-566), DEBRIS_CENTER (350,20,-160), PIRATE_SPAWN
  (380,40,-100).
- The scavenger reveal moves from orbit completion to rehearsal
  completion - verify the reveal ordering reads well; the marker jump
  covers wayfinding back to the field.
