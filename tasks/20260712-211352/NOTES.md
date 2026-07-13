# Notes: examples rework for testability (20260712-211352)

## Audit v2 (user-directed structure, 2026-07-13 - supersedes the round-1 table below)

The user redirected the rework after the round-1 checkpoint: too-simple
demos go, and the curriculum is SECTIONS -> SCENARIO -> EDITOR -> PLAYABLE,
everything testable. Final set:

| Example | From | Asserts |
|---|---|---|
| 01_controller_section | NEW | the PD tracks a rotating attitude command (0.18 rad lag measured) |
| 02_thruster_section | NEW (absorbs 02_thruster_shader's knob check) | full burn grows nose speed (0 -> 46 u/s) and the plume shader follows the throttle |
| 03_hull_section | NEW (absorbs 10_gameplay's destruction) | exact partial-damage drop, overkill destroys the section, root + controller survive |
| 04_turret_section | 08_turret_range | fired -> gate damaged (plus the aim/owner-pair guards it had) |
| 05_torpedo_section | 06_torpedo_range | fired -> armed -> detonated -> gate damaged |
| 06_torpedo_guidance | 07_torpedo_guidance | scenario-load + PN gizmos (unchanged) |
| 07_com_range | 11_com_range | unchanged, now in the smoke list |
| 08_scenario | 03_scenario + 01_scene | the event grammar live: OnStart seeds variables, type-filtered OnDestroyed tallies, expression filter advances the beat, OnUpdate promotes it |
| 09_editor | 09_editor | unchanged |
| 10_playable | NEW | real gestures through the live input pipeline: combat lock the prey (identity asserted), gun it down, travel-lock the beacon, G engages GOTO, the beacon's own trigger area sees the arrival - all read back from the scenario's variables |
| 11_hud_range | 12_hud_range | unchanged + the velocity-sphere tracking stage folded in from 05_directional |
| 12_menu_newgame | 13_menu_newgame | unchanged (boot pin) |

Deleted: 01_scene, 02_thruster_shader, 04_asteroids (tuning tool),
05_directional, 10_gameplay, 07b_slicer (round 1). Every probe carries a
completion backstop (fail loudly if the window closes before the script
finished - the vacuous-pass guard 11/12 pioneered).

## Bugs the new assertions flushed out (this task's thesis, proven)

- BOTH weapon ranges have been fire-dead since the weapons safety landed
  (20260713-082337): the safety denies a press on a cold ship, a held key
  never re-edges, and the old reach-Playing smoke could not see it. Fixed
  in the range scripts (raise stance, wait for WeaponsHot, then press) and
  in the controls docs; the outcome assertions now pin the whole chain.
- Firing a torpedo volley drifts the ship ~20 u/s with no engaged drive -
  found by 10_playable's timeline probe, filed as task 20260713-220512
  (10_playable ships with a turret loadout partly to stay robust to it).
- The radar pick is purely angular, so with the camera above the hull a
  FARTHER on-axis object outranks a near one on the same boresight -
  10_playable's collinear first geometry locked the beacon behind the
  prey. Documented in the example; possibly playtest-relevant (an enemy in
  front of a locked beacon may be hard to lock).
- The asteroid hierarchy trap from 20260713-150343 (id on the root, Health
  on the child) bit again in 08_scenario's first damage path - the probe
  now damages the health node the way real rounds do.

## Audit v1 (round 1, superseded)

## Audit (decision per example, 2026-07-13)

| Example | Lines | Decision | Reasoning |
|---|---|---|---|
| 01_scene | 74 | KEEP + harness | The one example that builds a `ScenarioConfig` in CODE (vs loading a named asset) - the modding teaching point. Harness is nearly free: `nova_autopilot` + `assert_scenario_loaded` (id `test_scenario`, 1 handler, 20 objects). |
| 02_thruster_shader | 330 | KEEP + harness | Unique shader-tuning tool (slider -> `ThrusterExhaustMaterial`). Behavior assertion: script drives `DemoWidgetStates.slider_value`, asserts the material's `thruster_input` follows - pins the knob wiring, not just boot. |
| 03_scenario | 43 | KEEP as-is | Canonical named-scenario smoke, already asserted; the curriculum's hello-world. |
| 04_asteroids | 337 | KEEP + harness | Unique asteroid/planet mesh-generation tuning tool. Assertion: mutate the gen knobs, assert the marked mesh regenerates. |
| 05_directional | 107 | KEEP + port + harness | Only hand-built (non-AppBuilder) app in the set - its own comment apologizes for duplicating status-bar wiring. Port onto AppBuilder, then assert the velocity sphere actually tracks the moving target's velocity. |
| 06_torpedo_range | 389 | KEEP + strengthen | Harnessed but only reach-Playing; its PURPOSE (arm/home/detonate) is unasserted. Add an outcome assertion (target takes blast damage inside the window). |
| 07_torpedo_guidance | 288 | KEEP as-is | Harnessed + scenario-load assertion; PN guidance internals are unit-tested, and a hit assertion would duplicate 06's. |
| 07b_slicer | 111 | REMOVE | Header says "TODO: move to bevy_common_systems as a small game" - its subject (mesh slicer) lives in bcs; its scaffold is a copy of 01's scenario helper. Nova's destruction pipeline is asserted in 10/11. Removal recorded for the bcs promotion backlog (20260706-151804). |
| 08_turret_range | 559 | KEEP + strengthen | Same as 06: fires but nothing asserts rounds connect. Add a target-damage assertion. |
| 09_editor | 275 | KEEP as-is | Editor flow harnessed (create ship + controller path). |
| 10_gameplay | 238 | KEEP as-is | Inline assembly + destruction, harnessed + scenario assertion. |
| 11_com_range | 412 | KEEP + join smoke | Richest physics assertions in the tree; excluded from `HARNESSED_EXAMPLES` only because the const's criterion was literally "wires `nova_autopilot`" and 11/12 use a custom staged AutopilotPlugin. They satisfy all three smoke assertions (exit 0, reached Playing, cycle complete). |
| 12_hud_range | 864 | KEEP + join smoke | Same as 11. |
| 13_menu_newgame | 136 | KEEP as-is | Shipped boot-flow pin (20260713-175352). |

Numbering: keep existing numbers (renumbering churns git history, docs and
muscle memory for zero behavior); 07b's removal leaves no gap in the NN
sequence proper.

## Why 11/12 were out of the smoke list

`HARNESSED_EXAMPLES`'s doc comment states the criterion: "The examples that
wire `nova_autopilot` (grep `nova_autopilot` in `examples/`)". 11/12 predate
no infrastructure and hit no blocker - they simply use
`AutopilotPlugin::new().hold(Loading, 8.0)` directly (they need a longer
window and staged scripts), so the grep criterion excluded them. The three
smoke assertions all hold for them: bcs's AutopilotPlugin logs "cycle
complete, no panic" on any clean cycle, and nova_debug logs "reached
Playing" whenever BCS_AUTOPILOT is set. Fix: add them to the const and
rewrite its criterion comment ("drives itself under BCS_AUTOPILOT").

## Bug-pin decisions for the two v0.5.2 fixes

- Teardown despawn race (20260712-115902): NO bespoke example pin. The unit
  pins run the real observer against the real command queue; an example pin
  would need a scenario transition with an autopilot engaged mid-unload,
  which no current example flow reaches deterministically inside its
  window. The class-level example coverage comes from task 20260713-203709
  (fail any harnessed example whose stderr contains "Encountered an error
  in command"), which covers every example at once - strictly more than a
  bespoke pin.
- Audio hum attenuation (20260711-183417): NO bespoke example pin. The App
  tests drive the production `compute_thruster_hum_volume` system itself;
  an example pin would re-run the same system in a slower rig, and no
  harnessed example has a distant burning ship inside its autopilot window
  (the menu ambience ship only burns after the 13_menu_newgame script has
  already clicked through - verified during that task's trace hunt).

## Removal sweep (07b_slicer)

To sweep before closing: docs/development.md example list, any docs/ or
comment references to "07b" / "slicer example", CHANGELOG mentions, CI
comments. The slicer machinery itself (bcs) is untouched; pointer recorded
for the promotions backlog task 20260706-151804.
