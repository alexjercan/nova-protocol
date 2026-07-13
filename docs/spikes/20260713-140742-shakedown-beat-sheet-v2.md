# Spike: Shakedown beat sheet v2 - smaller objectives, more beacons, more fun

- DATE: 20260713-140742
- STATUS: RECOMMENDED
- TAGS: spike, scenario, tutorial, ux

## Question

Playtest (user, 2026-07-13, after the radar-era rework landed): "too much
text in the objectives; split them into smaller ones, add more beacons,
make it a little longer but less stressful and more fun; add 2-3 more
interesting things." What beat structure delivers one-lesson objectives,
and what (if anything) does the scenario event vocabulary need to support
it? A good answer is a concrete beat sheet plus the minimal engine work,
ready for /plan.

## Context (verified against the code, 2026-07-13)

- The pain is real and measurable: OBJ_B4 is four sentences teaching THREE
  gestures (radar hold, [G], [O]); OBJ_B5 is three sentences teaching three
  more (raise, combat lock, fire). Beats 1-3 are already one-lesson-ish.
- The event vocabulary (nova_scenario events.rs:12): OnStart, OnDestroyed,
  OnUpdate (+ variable filters), OnEnter, OnOrbit. Actions: objectives,
  markers, emphasis, spawn/despawn, SetSpeedCap, SetControllerVerb,
  NextScenario. There is NO "player locked X" event - which is exactly the
  completion signal a split radar lesson needs to TICK when the lock lands
  (the satisfying Arma-style confirmation the contextual cluster already
  gives for key availability).
- Precedent for a new game-state event: `track_orbit_holds`
  (loader.rs:195-245) watches Autopilot state, resolves the well's scenario
  `EntityId`, and fires `OnOrbitEvent { id, other_id, other_type_name }`.
  An OnLock bridge is the same shape over `TravelLock`/`CombatLock` change
  detection. nova_editor does not enumerate event variants (verified) - the
  surface is nova_events + nova_scenario + tests.
- Beat-4 geometry is pinned against the worst planetoid seed
  (beat4_geometry_holds_across_the_derived_radius_range); any new beacon or
  area near the planetoid must extend that pin, not eyeball it.
- Beacons are cheap content: `beacon(id, label, pos)` + `mark()`; the
  scenario also spawns asteroids with authored radius/health/gravity - a
  "derelict" practice target can be authored today (an asteroid IS
  InsetZoomable via the scenario layer, so the viewfinder works on it).

## Options considered

- **A. Text-trim only.** Rewrite the fat objectives into shorter sentences,
  same five beats. Cheapest; fails the ask (no "longer", no new content,
  beat 4 still teaches three gestures in one breath).
- **B. Spatial-only split.** More beacons; every micro-objective advances
  on existing events (OnEnter / OnOrbit / OnDestroyed). No engine work.
  Weakness: the lock lessons cannot COMPLETE on the lock itself - "lock
  BEACON 3" only ticks when you ARRIVE, so the one moment the radar
  tutorial most wants to reward (the lock landing) stays silent.
- **C (recommended). B plus ONE new event: OnLock.** A loader bridge
  (the OnOrbit shape) fires when a player lock lands on a scenario object:
  info `{ id, other_id, other_type_name, combat: bool }` - one event, the
  slot as a field, filterable like OnEnter (an `eq_bool`-style filter or a
  dedicated filter arm; /plan decides the encoding, both exist as shapes).
  Three beats consume it immediately (travel-lock lesson, re-designation
  leg, combat-lock rehearsal), which is what justifies the vocabulary
  growth.
- **D. Full vocabulary expansion** (OnManeuverEngaged, OnWeaponsRaised,
  OnSafetyChanged...). Rejected: each is a bridge + tests, each has one
  hypothetical consumer; [G] confirmation falls out of OnEnter (the GOTO
  leg ends somewhere) and raise-confirmation falls out of OnLock(combat)
  (you cannot combat-lock without being raised).
- **Do nothing** - rejected by playtest.

## The beat sheet v2 (the recommendation, C)

Design rules: ONE gesture per objective (a W/X pair counts as one lesson);
one line of text each (target <= 15 words); every new beat is failure-free
(no timers, no damage outside the rehearsed fight - "less stressful");
gestures rehearse once in calm before they matter in the fight.

1. **Burn** (OnEnter beacon_1, as today): "Hold [W] and burn for BEACON 1.
   Tap [X] to stop." (governor parenthetical DROPPED - it announces itself
   at release).
2. **Look** (OnEnter beacon_2, as today): "Governor released. BEACON 2 is
   off your beam - hold [Alt] and look around."
3. **Salvage** (tally, as today): "Recover 3 supply crates from the debris
   cluster." (+ existing 1/3, 2/3 tallies).
4. **First lock** (NEW, OnLock travel beacon_3): "Targeting computer
   online. Hold [CTRL] on BEACON 3 until the lock sticks." Ticks the
   INSTANT the white lock lands. RADAR emphasized.
5. **Hands off** (OnEnter beacon_3): "Locked. Press [G] - let the computer
   fly." GOTO emphasized; ticks on arrival.
6. **Waypoint run** (NEW BEACON 4 on the planetoid approach; OnEnter
   beacon_4): "New waypoint: BEACON 4. Lock it, press [G] again." One-line
   rehearsal; teaches that GOTO captures the lock you have at [G] (the
   re-designation semantics, currently untaught). Interesting-thing #1.
7. **Gravity coast** (NEW ring area inside the planetoid's grip; OnEnter
   coast_ring): "You are in the planetoid's pull. Cut the burn and coast."
   Zero keys, scenic, teaches SOI awareness by FEEL. Interesting-thing #2.
   Geometry must extend the worst-seed SOI pin.
8. **Orbit** (OnOrbit planetoid, as today): "Press [O] and hold the
   orbit."
9. **Live-fire rehearsal** (NEW derelict target near the field; two
   micro-beats): 9a (OnLock combat derelict): "A derelict hulk drifts
   ahead. Hold [RMB], keep [CTRL] on it - watch the viewfinder." Ticks
   when the RED lock lands. 9b (OnDestroyed derelict): "Locked on. Open
   fire - [LMB]." A calm target that shoots nothing back.
   Interesting-thing #3.
10. **The scavenger** (OnDestroyed pirate): "A scavenger is picking
    through your debris field. Drive it off." ONE line - every gesture was
    rehearsed; the fight is the exam, not the lesson.
11. **Stand down** (epilogue, as today): "Shakedown complete. Tap [CTRL]
    to stand down your locks - the belt is yours."

Net: 5 beats -> 10, beacons 3 -> 4 (+ a derelict + a coast ring), every
objective one line, the fight text shrinks from three sentences to one.
The capability grant moves with its lesson (beat 4); LOCK stays withheld
through beats 1-3 exactly as landed today.

## Open questions

- The derelict's body: an inert ship silhouette (sections, no controller)
  would look best in the viewfinder but may need a "no controller" spawn
  path; a big named rock works today. /plan verifies which is cheap.
- The coast ring: OnEnter needs an area - a beacon has `area_radius`, but
  the ring should be INVISIBLE (no chip). Check whether an area-only
  scenario object exists; if not, an unlabeled/unmarked beacon variant or
  a new minimal Area object kind is the fallback.
- OnLock firing semantics: first lock only, or every retarget? (The live
  radar retargets under the sweep - the bridge should fire on lock CHANGE
  onto the filtered id, once per acquisition, or beat 6 self-completes
  from beat 4's leftover lock. The scripted walk must pin the staged
  double-designation.)
- Does the waypoint run read as busywork? (Playtest; it is one line and
  ~20 s.)
- Beacon 4 / ring / derelict positions vs the worst-seed SOI (extend the
  geometry pin).

## Next steps

Direction-level tasks this spike seeded, for /plan to break into steps:

- tatr 20260713-140922: OnLock scenario event - loader bridge from the
  lock components (OnOrbit shape), combat/travel discrimination,
  filterable by target id.
- tatr 20260713-140929: shakedown beat sheet v2 - the content above
  (split objectives, beacon 4, coast ring, derelict rehearsal, one-line
  text budget, emphasis re-pairing, walk-test rewrite, geometry pin
  extension). Depends on the OnLock event.

## Fix record

(appended by the implementing tasks as they land)
