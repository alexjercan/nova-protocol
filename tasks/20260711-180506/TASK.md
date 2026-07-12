# Starter New Game scenario: fun but gentle

- STATUS: OPEN
- PRIORITY: 40
- TAGS: v0.5.0,scenario,content,spike

## Goal

The scenario New Game actually drops you into: "Shakedown Run" (id
`shakedown_run`), a five-beat first-flight tutorial per the spike's beat
sheet - burn to beacon, freelook find, salvage sweep, GOTO/ORBIT hands-off,
then a pirate that spawned in the debris cluster as the finale. Legs a few
hundred meters, ships minimal (one turret each), conveyance is layer 0
(imperative text with [KEY] names, emissive blinking props, short
distances). Ends with New Game loading it instead of asteroid_field.

## Steps

- [ ] Verify TurretSectionConfig's damage/fire-rate fields in
      `crates/nova_assets/src/sections.rs` (better_turret_section, line
      ~59), then register a `light_turret_section` there: same model,
      noticeably lower damage and fire rate - the pirate's armament, so
      "gentle" is data. Also confirm which section healths make a weak
      pirate (reinforced_hull_section is 150; if nothing lighter exists,
      add a `light_hull_section` with ~60 health).
- [ ] New module `crates/nova_assets/src/scenario/shakedown.rs` (turn
      scenario.rs into a directory module, or add `mod shakedown` beside
      it - keep asteroid_field where it is): `pub fn shakedown_run(...)
      -> ScenarioConfig`. Layout constants at the top (positions derived
      from the beat distances): player spawn; beacon_1 ~350u ahead
      (+Z); beacon_2 ~300u further, ~120 degrees off the beacon_1
      approach axis; debris cluster (8-10 small asteroids, radius 1-3)
      just past beacon_2 with crate_1..crate_3 inside it; planetoid
      (nominal 20u, surface_gravity Some(6.0) - geometric radius runs
      80-91u, menu_ambience:39) ~700u out; beacon_3 at ~200u from the
      planetoid center (inside the SOI); orbit gate area = sphere around
      the planetoid, radius ~150u. All props clear of the planetoid's
      geometric radius (the menu_ambience penetration lesson).
- [ ] Player ship: SpaceshipController::Player, LMB turret mapping (copy
      asteroid_field:220), sections controller + reinforced_hull_front +
      basic_thruster + better_turret - one turret, no hull_back, no
      torpedo bay. Pirate: SpaceshipController::AI with patrol waypoints
      looping over the debris cluster, sections controller + light_hull +
      basic_thruster + light_turret; NOT spawned at OnStart - its
      SpawnScenarioObject action rides the salvage-complete event.
- [ ] Beat chain events (mirror asteroid_field's variable+filter idiom):
      OnStart spawns everything except the pirate, sets `beat` bookkeeping
      variables, pushes objective b1 "Your ship is drifting. Burn for
      Beacon 1 [W]". OnEnter(beacon_1, player) + beat guard -> complete
      b1, push b2 "Beacon 2 is off your beam. Hold [Alt] to look around
      and find it". OnEnter(beacon_2, player) -> complete b2, push b3
      "Recover 3 supply crates from the debris cluster [X] stops you".
      OnEnter(crate_N, player) x3 -> DespawnScenarioObject(crate_N),
      increment `crates_recovered`, complete+re-add b3 with the count
      ("... 1/3", "... 2/3" - single-frame rebuild, no flicker, verified
      in 20260712-093044 notes). When crates_recovered == 3: complete b3,
      push b4 "Lock Beacon 3 and press [G]. Then make orbit over the
      planetoid [O]", and SpawnScenarioObject(pirate) in the same action
      list (quiet - no objective mentions it yet). OnEnter(orbit_gate,
      player) + beat guard -> complete b4, push b5 "A scavenger is picking
      through the debris field you cleared. Drive it off [RMB] aims,
      [LMB] fires". OnDestroyed(pirate) -> complete b5, push final
      objective "Shakedown complete - the belt is yours" (no
      NextScenario; free flight). OnDestroyed(player) ->
      NextScenario(shakedown_run, linger: true).
- [ ] Register `shakedown_run` in register_scenario
      (crates/nova_assets/src/scenario.rs:11) and flip
      NEW_GAME_SCENARIO_ID to "shakedown_run"
      (crates/nova_menu/src/lib.rs:40); update the const's doc comment.
- [ ] Config-shape tests beside the scenario (pattern:
      menu_orbiter_is_an_ai_ship_directed_at_the_planetoid,
      scenario.rs:586): every area/beacon/crate id referenced by an event
      filter or despawn action is spawned by OnStart (catches typo'd ids,
      the whole script is strings); the pirate is NOT in the OnStart
      spawn set but IS in exactly one later action list; the pirate is AI
      with a patrol route over the debris cluster and carries exactly one
      turret (the light one); the player ship has exactly one turret and
      no torpedo section; beacon_3 sits inside the planetoid's SOI
      given the geometric-radius derivation (cite
      insert_asteroid_gravity_well for the SOI rule when writing it);
      player death routes back to shakedown_run with linger.
- [ ] Playtest with /run: boot -> New Game lands in shakedown_run, walk
      all five beats, confirm each objective advances, the counter text
      updates, the pirate stays passive until approached, death restarts.
      Screenshot the beacon chip and the b5 fight for the record.
- [ ] Update docs: CHANGELOG.md entry; note in docs/scenario-system.md
      that shakedown_run is the New Game scenario and the reference
      example of the beat-chain idiom.

## Notes

- Spike (design, beat sheet): docs/spikes/20260712-092926-starter-scenario.md
- Spike (parent direction): docs/spikes/20260711-180500-main-menu.md
- Parent task: 20260711-174915
- Depends on: 20260711-180426 (New Game wiring; CLOSED - swap the id)
- Depends on: 20260712-093044 (nav beacon + salvage crate objects +
  despawn action)
- Conveyance is layer 0 by design; task 20260712-093831 upgrades visuals
  later without touching the beat chain (targets are scenario entity ids).
- Objective text must use the exact key labels the hint cluster shows
  (hud/keybind_hints.rs) so panel and cluster corroborate.
- The orbit gate is an area approximation of "in orbit" (the event system
  cannot see autopilot state); if playtest says it feels wrong, that is a
  follow-up, not a redesign (spike open question).
- rand is used at config-build time (asteroid_field does the same); keep
  the debris scatter deterministic-enough by seeding positions from
  constants if test flakiness appears.
