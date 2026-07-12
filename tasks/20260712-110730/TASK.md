# Shakedown Run playtest round 1 fixes

- STATUS: CLOSED
- PRIORITY: 50
- TAGS: v0.5.0,scenario,content,bug

## Goal

Fix the seven findings from the user's first visual playtest of Shakedown
Run (2026-07-12), two of which are progression blockers: GOTO parks the
ship outside the beacon trigger, and the orbit gate can be unreachable
(the ORBIT ring plans at max(park, current radius), so engaging from
beacon 3 at ~260u rings OUTSIDE the 200u gate). Verdicts recorded
verbatim in Notes.

## Steps

- [x] (finding 2, BLOCKER) Beacon triggers must contain the GOTO park
      point: GOTO stops at FlightSettings::arrival_standoff (50u,
      flight.rs:357) plus target BodyRadius (beacons have none), while
      BEACON_AREA_RADIUS is 40u - the autopilot parks 10u outside the
      objective. Raise BEACON_AREA_RADIUS to 70u and add a config-shape
      assertion citing FlightSettings::default().arrival_standoff
      (BEACON_AREA_RADIUS > arrival_standoff + margin) so a standoff
      retune cannot silently reopen the gap.
- [x] (finding 5, BLOCKER) Replace the orbit-gate AREA with an orbit-held
      event: new `EventConfig::OnOrbit` + `OnOrbitEvent` (nova_events)
      fired once per engagement when a ship has held an engaged
      `Autopilot { action: Orbit { well } }` for N continuous seconds
      (start N = 5.0): a nova_scenario system (gated scenario_is_live)
      tracks the hold per ship and fires with (id = well's scenario
      EntityId, other_id = ship's EntityId); disengage re-arms. The
      shakedown beat-4 handler filters OnOrbit(planetoid, player) instead
      of OnEnter(orbit_gate); delete the orbit_gate area. Rationale: the
      position gate is unfixable by sizing - the ORBIT verb rings at
      max(clearance * (body + margin), current radius) (flight.rs:1632
      parking path and the manual-engage equivalent), so any authored
      gate radius loses to a far-out engage.
- [x] (finding 4) The pirate spawns with beat 5, not beat 4: move its
      SpawnScenarioObject from the salvage-complete handler into the
      beat-4-complete (OnOrbit) handler's actions, alongside the b5
      objective. Beat 4 is now pirate-free, so DELETE the
      early-pirate-kill branch (pirate_dead variable, the two extra
      handlers) - killing it can only happen in beat 5. Update the walk
      tests (the alternate-ending test becomes: pirate destroyed in beat
      5 completes the run; assert the pirate does NOT exist during beat
      4).
- [x] (finding 7) Close the hull gap: both ships place the turret at
      z = -1.0 (was -2.0; asteroid_field filled -1 with hull_back, the
      minimal ships left a hole between controller at 0 and turret).
      Assert adjacency in the ships config test (every section within
      1.0 of another section's position).
- [x] (finding 1) Manual speed cap: add `speed_cap: Option<f32>` to
      PlayerControllerConfig (nova_scenario spaceship config); spawn
      inserts a `FlightSpeedCap(f32)` component on the ship root;
      flight.rs manual-burn path scales the commanded burn toward zero as
      velocity along the burn direction approaches the cap (soft cap;
      autopilot maneuvers ignore it - they plan their own decel; braking
      and turning are never blocked). Shakedown player ship: 25.0. VERIFY
      FIRST where intent.burn becomes thruster demand and cap there; a
      physics test proves a capped ship under full held burn levels off
      near the cap while an uncapped one keeps accelerating (delivery
      guard: the uncapped run must exceed the cap).
- [x] (finding 3) Objectives text: constrain the panel and shrink the
      lines before the big HUD rework - nova owns the panel spawn
      (hud/mod.rs:479): give it a fixed width (~280px) so text wraps, and
      add a small system (after ObjectivesPluginSystems::Sync) inserting
      TextFont::from_font_size(13.0) + left-justified TextLayout on
      Added<ObjectiveMarker> lines. bcs itself stays untouched (git dep).
- [x] (finding 6) Invulnerable designated bodies: add
      `invulnerable: bool` to AsteroidConfig; when set,
      insert_asteroid_collider gives the child the collider + density +
      visibility WITHOUT Health (no destructible_body), so nothing can
      kill the body or its well. Set for the shakedown planetoid AND the
      menu_ambience planetoid (its historical death-by-ring-rocks bug
      class disappears); asteroid_field keeps destructible rocks.
      Config-shape test: invulnerable body's child has Collider but no
      Health; a destructible one has both.
- [x] Update docs/scenario-system.md (OnOrbit event, invulnerable flag,
      beacon-standoff rule of thumb) and CHANGELOG (fixed + changed
      entries); re-run the full new-test set + check + fmt.

## Notes

- Playtest verdicts (user, 2026-07-12, first human run of shakedown):
  1. "limit the max speed in the beginning so you don't fly off into
     space"
  2. "GOTO should be somehow configurable such that for beacons it
     doesn't stop at 50 meters away and it completes the objective
     properly"
  3. "the text from objectives is really big, maybe wrap and smaller,
     before the big rework"
  4. "the enemy ship spawns too quickly, it should spawn after we get to
     beacon 3 and complete that objective"
  5. "I wasn't able to complete the beacon 3 objective - make it easier
     (e.g. if you do orbit for some time it completes)"
  6. "make the planetoid invincible somehow (extra health attribute or
     something)"
  7. "the spaceship is missing a section between the controller and the
     turret (empty gap) - hull or move the turret closer"
- Root causes verified in code before planning: standoff 50u vs trigger
  40u (finding 2); ring = max(park, current radius) makes any authored
  gate radius beatable (finding 5). Findings 2+5 explain each other:
  GOTO leaves you at 260u, O rings you at 260u, gate at 200u never
  fires.
- Fixes 2, 4, 5, 7 ride the scenario + event system; 1 and 6 are small
  gameplay/config features; 3 is nova-side styling only.
- Follows: 20260711-180506 (CLOSED; its un-ticked visual-playtest step
  is what produced these). Conveyance task 20260712-093831 stays
  separate.

## Close record

All seven findings fixed. AS EXECUTED deltas from the plan: the speed
cap's overshoot is the spool-down tail (bounded by accel/spool_down_rate;
the test derives its bound from the measured rig acceleration instead of
a magic constant, and Time<Virtual>'s 250ms max_delta bit the first test
rig); OnOrbit fires via an OrbitHold component tracker (one event per
engagement, disengage re-arms), tested through the real pipeline; the
early-pirate-kill branch and its pirate_dead variable are DELETED (the
pirate exists only from beat 5), replaced by a
stray-death-is-a-no-op-guard test; raising the beacon trigger to 70u
immediately tripped the crate-overlap invariant test from review R2.3
(the invariant did its job) - the debris cluster moved out 25u; the
objectives styling is nova-side only (panel Node override + an
Added<ObjectiveMarker> restyle system after the Sync set), bcs untouched;
both planetoids (shakedown AND menu ambience) are invulnerable.

New tests: orbit-hold per-window recurrence, invulnerable-no-health,
speed-cap level-off (physics-derived bound + uncapped delivery guard),
objective-line restyle-on-rebuild, and the reworked shakedown suite
(pirate-free beat 4, OnOrbit completion, stray-death guard,
standoff-containment and adjacency invariants).

Self-reflection: the crate-overlap trip proved the value of pinning
"unreachable" geometric invariants - my own fix would have shipped the
exact hole the reviewer had flagged as theoretical. And the
Time<Virtual> max_delta clamp is a new rig gotcha for the ledger.

Post-review round 1 (user playtest of the branch + agent review): the
objectives panel Node override PANICKED at runtime - a spawn bundle with
two Node components (the bcs panel's and nova's) is a duplicate-component
panic, not a last-wins override, and the styling test had spawned the
BARE panel so it never hit the production tuple (user-found BLOCKER,
R1.5). Fixed with insert-after-spawn (replace semantics) factored into
spawn_objectives_panel(), which the test now exercises. Agent MINORs
folded in: OnOrbit is now RECURRING per hold window (a single-shot event
consumed under a rejecting beat guard would soft-lock any scenario whose
beat advances mid-orbit - also defuses the early-orbit landmine), and an
unaddressable well no longer consumes the hold window.

Post-review round 2: both round-1 test claims were FALSE - the scripted
replacements had silently no-opped against fmt-reflowed bodies and were
reported as done without verification (reviewer R2.1/R2.2, both MAJOR).
The tests are now really fixed: the styling test goes through
spawn_objectives_panel() and asserts the override width; the orbit test
asserts window recurrence (1 -> 2 during one hold) and fresh-clock
re-engage (-> 3).
