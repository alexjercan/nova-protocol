# Rework the examples to make more sense and to have better testing capabilities

- STATUS: CLOSED
- PRIORITY: 80
- TAGS: v0.5.2,refactor,examples,tests

## Goal

The examples were not created with testing in mind initially, so I think it
would be a good idea to re-work them and refactor them (can even remove some of
them and add new ones) to be more "testable". Maybe even to touch on some
corner cases. Also testing things that we know were bugs (when we hit a bug it
might be a good idea to write a "example test" that reproduces it and then fix
it). This could be a good learning thing to think about.

## Baseline (2026-07-13 plan pass)

13 examples, ~4.2k lines. Seven run in CI via tests/examples_smoke.rs
`HARNESSED_EXAMPLES` (03_scenario, 06_torpedo_range, 07_torpedo_guidance,
08_turret_range, 09_editor, 10_gameplay, 13_menu_newgame) asserting exit 0 +
"reached Playing" + "cycle complete"; 03/07/10 also `assert_scenario_loaded`.
11_com_range and 12_hud_range carry the RICHEST multi-stage behavior
assertions in the tree but are NOT in `HARNESSED_EXAMPLES` (custom 8 s
AutopilotPlugin timelines instead of `nova_autopilot()`). 01_scene,
02_thruster_shader, 04_asteroids, 05_directional, 07b_slicer have no harness
at all (07b is marked "TODO: move to bevy_common_systems").

## Steps (v2 - user-directed structure, 2026-07-13)

The user redirected the round-1 audit (which had kept/harnessed the old set
1:1): the curriculum is sections first, then scenario loading, then the
editor, then a playable scenario - and everything must be testable. Round-1
work that survives: the outcome-assertion pattern, 11/12 joining the smoke
list, the 07b removal, the docs/bug-pin sections. Round-1 work dropped: the
bespoke harnesses for 02_thruster_shader / 04_asteroids / 05_directional
(examples deleted; their assertion subjects fold into the new set).

- [x] Restructure moves: 08_turret_range -> 04_turret_section,
      06_torpedo_range -> 05_torpedo_section, 07_torpedo_guidance ->
      06_torpedo_guidance, 11_com_range -> 07_com_range, 03_scenario ->
      08_scenario, 12_hud_range -> 11_hud_range, 13_menu_newgame ->
      12_menu_newgame; deleted 01_scene, 02_thruster_shader, 04_asteroids,
      05_directional, 10_gameplay (07b already removed in round 1). All
      internal names/paths updated; HARNESSED_EXAMPLES names the final
      twelve.
- [x] Fix the fire-dead ranges (found by the new outcome assertions): the
      v0.5.0 weapons safety denies a press on a cold ship and a held key
      never re-edges, so both ranges fired NOTHING (headless and
      interactive) since 20260713-082337 landed. Scripts now raise the
      combat stance (RMB) and only press fire once WeaponsHot; controls
      docs teach the same gesture.
- [x] NEW 01_controller_section: PD attitude - command a rotation through
      the controller section's input seam, assert the hull converges.
- [x] NEW 02_thruster_section: burn -> velocity grows along ship forward;
      exhaust shader input follows throttle (absorbs the old shader-knob
      assertion).
- [x] NEW 03_hull_section: damage -> health drops -> disable -> destroy ->
      integrity graph reacts (absorbs 10_gameplay's destruction content).
- [x] 08_scenario: enrich from bare named-load to "variables and lots of
      things": scenario with variables, several event kinds, filters and
      actions; assert the load-level machinery (payload counts + variables
      present with expected initial values + a variable-gated handler
      fires). Absorbs 01_scene's programmatic-config content.
- [x] NEW 10_playable: play a scenario - GOTO a beacon, raise + fire at a
      target, complete an objective beat; assert the scenario reacted
      (variable/objective advanced).
- [x] 11_hud_range: fold in a velocity-sphere tracking stage (from the
      deleted 05_directional).
- [x] Update docs/development.md's curriculum list + ci.yaml's smoke-step
      comment (it names 03_scenario); sweep all old example names
      (sweep-then-delete).
- [x] Full check suite + the full smoke run locally (Xvfb) over the final
      set; CHANGELOG updated.

## Notes

- Depends on: 20260711-183417 and 20260712-115902 (bug fixes land first so
  this task can pin them). PRIORITY 90 -> 80 in the v0.5.2 plan pass to
  encode that order; the substance is unchanged.
- 20260710-143138 (CI taffy) depends on THIS task: re-enable the smoke gate
  over the final example set, not the pre-rework one.
- Key files: tests/examples_smoke.rs, crates/nova_debug/src/harness.rs
  (module docs describe the harness contract), examples/*.rs,
  docs/development.md.
- Removing an example must sweep every reference (sweep-then-delete):
  docs/, HARNESSED_EXAMPLES, CI comments, other examples' comments.


## Record (2026-07-13)

What changed: the examples are a four-block curriculum (sections ->
scenario -> editor -> playable), every one self-driving under BCS_AUTOPILOT
(ten of twelve with panic-on-failure behavior assertions + completion
backstops; 06/09 assert at load/reach-gameplay level); the CI smoke list
runs all twelve (full suite green locally in ~130 s under Xvfb). Round 1
harnessed the old set 1:1; the user redirected the structure at the
checkpoint and round 2 rebuilt it (moves, merges, five removals, three new
section examples, the scenario-language example, the playable). Full audit
v2, the four bugs the assertions flushed out, and per-decision reasoning in
NOTES.md; two follow-up tasks filed (20260713-220512 torpedo-volley ship
drift; 20260713-203709 was already filed by the teardown task).

Difficulties (the playable took five iterations to stabilize): the radar
pick is purely angular so the collinear beacon outranked the prey (fixed by
off-axis geometry + a lock-identity assert); wall-clock beat staging broke
under suite load twice (fixed by making every beat event-driven on the
game state it produces); and full area-arrival proved untestable in-suite
(llvmpipe throttles unfocused windows, so a multi-second flight leg gets
too few sim seconds) - the headless contract was rescoped to
"GOTO engaged and closing", documented in the example.

Self-reflection: the round-1 audit optimized for keeping the existing set
and got redirected - checkpointing the table BEFORE implementing round 1's
harnesses would have saved the three discarded ones (the checkpoint came
with work already done). The event-driven-beats principle should have been
the starting point, not the third fix: every timing flake in this task was
a wall-clock stage racing game state. On the win side: outcome assertions
immediately caught two real shipped bugs (fire-dead ranges, torpedo-volley
drift) - the task's thesis paid for itself on day one.
