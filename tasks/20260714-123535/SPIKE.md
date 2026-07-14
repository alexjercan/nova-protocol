# Spike: drive controller verb availability from the disable-verb component, not config flags

- DATE: 20260714-123535
- STATUS: RECOMMENDED
- TAGS: spike, modding, refactor

## Question

After the section-modification work (113411), a controller's verb availability is
described in TWO places: `ControllerSectionConfig.verbs` (four bools in the config)
and `DisableVerb` modifications (components). Can we drop the config/bool
representation and drive verb availability from the disable-verb component alone -
"granted by default, withheld by a modification" - so the availability system queries
that component instead of a bool struct? A good answer says whether this is simpler,
whether it is feasible given the runtime `SetControllerVerb` action (which must keep
toggling verbs live), and what it touches.

## Context

Traced every reader/writer of `ControllerVerbs`
(`crates/nova_gameplay/src/sections/controller_section.rs:127`, a `{stop, goto, orbit,
lock: bool}` struct, `Default` = all true):

- **Built** from `ControllerSectionConfig.verbs` by `controller_section(...)`.
- **Seeded at spawn** by the `DisableVerb` modification (113411):
  `apply_section_disable_verb` clears verbs on `ControllerVerbs`
  (`objects/modification.rs:109`).
- **Toggled at runtime** by the `SetControllerVerb` scenario action
  (`actions.rs:489-526`, `verbs.set(verb, enabled)`).
- **Read** in ONE place: the flight/verb-availability gate
  (`input/player.rs:179`, `q_computer: (&ChildOf, Option<&ControllerVerbs>)`), which
  decides which verb hints/inputs are live. The HUD does not read it directly.

Two facts make the cleanup real, not cosmetic:

1. **`ControllerSectionConfig.verbs` is now VESTIGIAL.** Post-113411 the catalog's
   `basic_controller_section` has all verbs `true`, and every real withhold (shakedown
   player: GOTO/LOCK/ORBIT) is a `DisableVerb` MODIFICATION, not a config flag. Grep
   confirms no production config/RON sets any verb `false` (the only `false`s are in
   test fixtures). So the config field always carries the default and does nothing.
2. **The runtime state is already single-source.** At runtime everything writes the
   ONE `ControllerVerbs` component. The redundancy is between `ControllerVerbs` (a
   bool struct) and the `DisableVerb` component (a modification) - two shapes for the
   same "which verbs are withheld" fact.

Note: "no controller section = not flyable" is enforced by the gate's QUERY FILTER
(`With<ControllerSectionMarker> + With<PDController> + Without<SectionInactiveMarker>`),
NOT by `ControllerVerbs` presence - so dropping the component does not lose that gate.

## Options considered

- **A. Collapse to one withheld-verbs component (recommended, the user's idea).**
  Replace `ControllerVerbs {4 bools}` + `SectionDisableVerb(Vec<FlightVerb>)` with a
  single `WithheldVerbs(HashSet<FlightVerb>)` on the controller section (empty = all
  granted). The `DisableVerb` modification seeds it; `SetControllerVerb` toggles it
  (enable = remove, disable = insert; insert the component if absent); the gate reads
  it (`granted(v) = !withheld.contains(v)`). Drop `ControllerSectionConfig.verbs`
  entirely - withholding is authored ONLY as modifications. Pros: one representation,
  removes the vestigial config field, "the modification IS the state" (the user's
  model), extensible. Cons: a moderate refactor touching the gate, the action, the
  config, the catalog RON, and ~6 tests; behavior must be pinned (it is
  behavior-preserving).
- **B. Minimal: delete the vestigial `config.verbs`, keep `ControllerVerbs`.** Just
  remove the always-default config field; `DisableVerb` keeps writing
  `ControllerVerbs`; the gate is unchanged. Pros: smallest change, removes the dead
  field. Cons: keeps two component shapes (`ControllerVerbs` + `DisableVerb`) - does
  NOT deliver the "query the modification component" unification the user asked for.
- **C. Do nothing.** `config.verbs` stays vestigial - a standing smell that a future
  reader mistakes for a live knob. Cheap now, confusing later.

## Recommendation

**Option A** - collapse to a single `WithheldVerbs(HashSet<FlightVerb>)` component,
the live state that the `DisableVerb` modification seeds, `SetControllerVerb` toggles,
and the gate reads; drop `ControllerVerbs` (the bool struct) and the vestigial
`ControllerSectionConfig.verbs`. It is what the user described (query the modification
component, not a config field), it removes a genuinely-dead config field that 113411
left behind, and it collapses the two redundant shapes into one. Verbs are the one
"modification that is also live runtime state" (unlike SetHealth/Rename, which are
one-shot) - the spike names that explicitly so the design is not confused with the
one-shot modifications.

It beats B because B leaves the two-component redundancy (the actual thing the user
wants gone) and only removes the config field. It beats C because the vestigial field
is a real trap. The churn (the input gate + the runtime action + tests) is moderate
and fully behavior-preserving - guard it with the existing verb tests
(`SetControllerVerb` scoping, the DisableVerb single/multi/inert tests) + the shakedown
`goto_unlocks_at_the_first_objective` e2e + `12_menu_newgame`.

## Open questions

- **Component name/shape.** `WithheldVerbs(HashSet<FlightVerb>)` vs keeping the name
  `ControllerVerbs` but changing its representation to a set. Naming call at plan time;
  a set is cleaner than the `Vec` the current `SectionDisableVerb` uses (no dup logic).
- **STOP.** STOP is never withheld today; confirm the set model treats all four verbs
  uniformly (it does) and nothing special-cases STOP.
- **Interaction with the bundle/ship work.** None - this is section-controller-local;
  independent of 113418/134xxx.

## Next steps

- tatr 20260714-135642: refactor controller verb availability to a single
  `WithheldVerbs` component (drop `ControllerVerbs` bools + vestigial
  `ControllerSectionConfig.verbs`; DisableVerb seeds it, SetControllerVerb toggles it,
  the gate reads it). Behavior-preserving.
