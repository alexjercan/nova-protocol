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
`12_menu_newgame`. Independent of the bundle/ship work. `spike`-gated (design in the
SPIKE); plan first.
