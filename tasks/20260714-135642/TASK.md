# Refactor: collapse controller verbs to a single WithheldVerbs component (drop bool struct + vestigial config.verbs)

- STATUS: OPEN
- PRIORITY: 34
- TAGS: v0.6.0,modding,refactor

Spike: tasks/20260714-123535/SPIKE.md

Goal: behavior-preserving cleanup. Replace `ControllerVerbs {stop,goto,orbit,lock:
bool}` + the `SectionDisableVerb(Vec<FlightVerb>)` modification component with ONE
`WithheldVerbs(HashSet<FlightVerb>)` on the controller section (empty = all granted).
The `DisableVerb` modification seeds it; the `SetControllerVerb` runtime action
toggles it (enable=remove, disable=insert, insert-if-absent); the verb-availability
gate (`input/player.rs`) reads it (`granted(v) = !withheld.contains(v)`). DROP the
vestigial `ControllerSectionConfig.verbs` (always all-true post-113411 - withholding
is authored only as modifications). Migrate the catalog RON (drop the `verbs` block)
and the ~6 verb tests. Pin with the SetControllerVerb scoping tests + the DisableVerb
single/multi/inert tests + shakedown `goto_unlocks_at_the_first_objective` +
`12_menu_newgame`. Independent of the bundle/ship work.

## Plan (20260714)

`WithheldVerbs` is runtime state the gate reads, so it lives in nova_gameplay (where
`ControllerVerbs` is). FlightVerb already derives Hash+Eq (HashSet-ready). Behavior-
preserving throughout.

Steps:
- [ ] 1. nova_gameplay: add `WithheldVerbs(HashSet<FlightVerb>)` (empty = all granted)
  with `granted(v)`/`withhold(v)`/`grant(v)` + register_type; REMOVE `ControllerVerbs`
  (the bool struct) and the `ControllerSectionConfig.verbs` field. `controller_section`
  no longer inserts a verbs component (default = no WithheldVerbs = all granted).
- [ ] 2. nova_gameplay: update the verb-availability gate (`input/player.rs`, the one
  reader) to `Option<&WithheldVerbs>` and `granted(v) = withheld.map_or(true, |w|
  !w.contains(v))`. Update any other `ControllerVerbs` reader (flight.rs, targeting.rs
  test fixtures).
- [ ] 3. nova_scenario: the `DisableVerb` modification inserts/populates `WithheldVerbs`
  DIRECTLY (drop the intermediate `SectionDisableVerb` component + its observer -
  DisableVerb becomes "insert the state component"; SetHealth/Rename keep observers).
  `SetControllerVerb` action mutates `WithheldVerbs` (get_mut or insert-if-absent;
  disable=insert verb, enable=remove verb). Update the modification + action tests
  (incl. the multi-verb accumulation test - now trivially a set).
- [ ] 4. nova_assets: drop the `verbs` block from `build_sections`
  (`basic_controller_section`); regenerate `base.sections.ron` (sections_ron_parity);
  update any test reading `ControllerVerbs`.
- [ ] 5. Verify: `cargo test --workspace --no-run`; nova_gameplay/nova_scenario/
  nova_assets tests; `12_menu_newgame` (shakedown still withholds GOTO/LOCK/ORBIT) +
  `09_editor` under `DISPLAY=:0 BCS_AUTOPILOT=1 --features debug`; parity green.
  Behavior must be identical to pre-refactor.
