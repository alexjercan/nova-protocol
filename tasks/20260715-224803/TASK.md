# Make Gauntlet Run a playable sequential slalom race

- STATUS: OPEN
- PRIORITY: 60
- TAGS: modding, scenario, gameplay, web

## Goal

Turn the `gauntlet_run` portal mod (`webmods/gauntlet/gauntlet.content.ron`)
from four static beacons into an actually-playable time-order slalom: the
player spawns in a controllable ship, is guided gate-by-gate through
START -> GATE 1 -> GATE 2 -> FINISH, must thread the gates in order, and gets a
completion objective at the finish. Pure data-driven RON, no engine changes.

## Steps

- [ ] In `gauntlet_run`'s `OnStart`, spawn a player ship before the beacons:
      a `SpawnScenarioObject` with `kind: Spaceship((controller: Player((
      input_mapping: {"turret": [Mouse(Left), Gamepad(RightTrigger2)]},
      speed_cap: Some(25.0), infinite_ammo: true)), sections: [...]))` at
      `position: (0,0,0)`. Copy the four-section layout (controller / hull_front
      / thruster / turret with `source: Prototype(...)`) verbatim from the
      player block in `assets/base/scenarios/shakedown_run.content.ron`, but do
      NOT copy its `DisableVerb(Goto/Lock/Orbit)` modifications - the racer
      should have GOTO/lock enabled from the start. Keep the ship id
      `player_spaceship` (the id scenario systems key off; see the
      `on_player_spaceship_spawned` observer in
      `crates/nova_scenario/src/loader.rs`).
- [ ] Keep the four existing beacons (START, GATE 1, GATE 2, FINISH) with their
      `area_radius: Some(...)` values - that radius spawns the real OnEnter
      trigger area (bridge proven in `crates/nova_scenario/src/objects/area.rs`).
      Keep each beacon's `lock_signature: None`.
- [ ] Add ordering state: in `OnStart`, `VariableSet(key:"gate", Number(1.0))`,
      `Objective((id:"gate_1", message:"Thread GATE 1."))`,
      `ObjectiveMarkerAttach((target_id:"gauntlet_gate_1", label:"GATE 1"))`,
      `HintEmphasisSet((verb: Goto))`. Model the objective+marker+variable shape
      on shakedown's beacon handlers.
- [ ] Add an `OnEnter` event for GATE 1: `filters: [Entity((id:
      Some("gauntlet_gate_1"), other_id: Some("player_spaceship"))),
      Expression((Equal(Term(Factor(Name("gate"))), Term(Factor(Literal(
      Number(1.0)))))))]`; actions: `ObjectiveComplete((id:"gate_1"))`,
      `VariableSet(gate = 2.0)`, `ObjectiveMarkerDetach(gauntlet_gate_1)`,
      `ObjectiveMarkerAttach(gauntlet_gate_2, "GATE 2")`,
      `Objective((id:"gate_2", message:"Thread GATE 2."))`. The `gate == 1` guard
      makes gates strictly sequential (entering a later gate early does nothing).
      Copy the OnEnter + Entity-filter + Expression-gate grammar from shakedown.
- [ ] Add the analogous `OnEnter` for GATE 2 (`gate == 2`): complete `gate_2`,
      set `gate = 3`, detach GATE 2 marker, attach FINISH marker, add objective
      `finish` "Cross the FINISH gate."
- [ ] Add the FINISH `OnEnter` (`id: "gauntlet_finish"`, `gate == 3`): complete
      `finish`, detach the FINISH marker, `HintEmphasisClear((verb: Goto))`, add
      a terminal `Objective((id:"course_done", message:"Course complete - nice
      flying."))`. Leave it terminal (no `NextScenario`: the gauntlet is a
      standalone portal mod with no successor).
- [ ] Update `webmods/gauntlet/gauntlet.bundle.ron` meta + the scenario
      `description` if wording no longer matches. Keep author/version.
- [ ] Add a production-faithful behavior test proving the gate bridge advances
      the race, NOT a hand-fired-event walk (lessons `scripted-walks-skip-the-
      bridges`, `production-faithful-rigs`). Model it on
      `an_area_spawned_around_a_body_fires_on_enter` in
      `crates/nova_scenario/src/objects/area.rs`: real `PhysicsPlugins`,
      `ScenarioAreaPlugin`, `GameEventsPlugin`; spawn a player body, spawn the
      gate area, step frames, assert `gate` advances 1 -> 2, and that entering
      gate 2's area while `gate == 1` does NOT advance.
- [ ] Keep the portal gate green: `cargo test -p nova_assets --test
      webmods_validation` (every webmods bundle still loads recursively).

## Notes

- Relevant files: `webmods/gauntlet/gauntlet.content.ron` (edit),
  `webmods/gauntlet/gauntlet.bundle.ron` (meta),
  `assets/base/scenarios/shakedown_run.content.ron` (reference grammar),
  `crates/nova_scenario/src/objects/area.rs` (OnEnter test rig),
  `crates/nova_assets/tests/webmods_validation.rs` (load gate).
- Gauntlet is a PORTAL mod: published by `nova_portal_gen`, NOT in `assets/`,
  validated by loading (no content_ron_parity builder). The RON is the single
  source of truth - edit it directly.
- Gates already carry `area_radius`, so no `CreateScenarioArea` needed.
- Verify-first: confirm the section Prototype ids (`basic_controller_section`,
  `reinforced_hull_section`, `basic_thruster_section`, turret prototype) exist
  in `assets/base/sections/base.content.ron` before relying on them; gauntlet
  depends on `base` implicitly.
- Assumption: no timer/scoring in v1 (kept "simple but playable"); a lap timer
  via OnUpdate is a possible follow-up (continuous-time support unverified).
