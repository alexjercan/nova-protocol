# Review: collapse controller verbs to WithheldVerbs

- TASK: 20260714-135642
- BRANCH: refactor/withheld-verbs

Out-of-context reviewer + implementer spot-checks of the two behavior-sensitive
changes (the `SetControllerVerb` absent-handling and the `targeting.rs` LOCK gate
required->Option), plus live `12_menu_newgame` + `09_editor` runs (both reached
Playing, exit 0). Behavior-preserving refactor.

## Round 1

- VERDICT: APPROVE

The reviewer independently re-derived the shakedown verb-withhold equivalence, confirmed
all four gates treat an absent `WithheldVerbs` as all-granted, traced the required->Option
widening (the only controllers that lacked `ControllerVerbs` on master - the editor
`preview_controller_section` and the standalone torpedo controller - are never children of
a `PlayerSpaceshipMarker` ship, so they cannot be newly matched in a behavior-visible way),
confirmed `SetControllerVerb` is net-identical, and found no leftover `ControllerVerbs`/
`SectionDisableVerb`/`config.verbs` references. Compiles clean workspace-wide.

- [x] R1.1 (MINOR) crates/nova_scenario/src/objects/modification.rs:227 - after the
  refactor `disable_verb_is_inert_on_a_hull` asserts the hull's `WithheldVerbs` is
  present-but-unread (the component is entity-agnostic now), which no longer pins the
  real inertness guarantee ("no gate reads it on a hull"). That guarantee lives in the
  readers' `With<ControllerSectionMarker>` filters, unpinned by this test.
  - Response: Added an assertion that the hull is NOT matched by a
    `With<ControllerSectionMarker>` query - pinning "a hull is not a controller section,
    so no verb gate reads its WithheldVerbs" explicitly rather than trusting the reader
    filters elsewhere. 6 modification tests pass.

No BLOCKER/MAJOR. Behavior verified identical (unit + shakedown e2e + two windowed
examples). Branch approved.
