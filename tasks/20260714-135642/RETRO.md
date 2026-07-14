# Retro: collapse controller verbs to WithheldVerbs

- TASK: 20260714-135642
- BRANCH: refactor/withheld-verbs
- REVIEW ROUNDS: 1 (APPROVE)

Process only; what/why in TASK.md, design in the spike (123535).

## What went well

- **Spiking the refactor first (123535) made the /flow run clean.** The spike settled
  feasibility (the runtime `SetControllerVerb` still works), the design (one
  `WithheldVerbs` set), and the load-bearing observation (config.verbs is vestigial
  post-113411) BEFORE building, so there was no fork mid-flow - just plan -> build ->
  review -> land. Spiking a non-trivial refactor before flowing it paid off.
- **A cleanup that deletes more than it adds** (net -258/+250): removed `ControllerVerbs`,
  the `SectionDisableVerb` component + observer, and a dead config field, replaced by one
  component. Well-motivated (113411 had made the config field vestigial), fully
  behavior-preserving.
- **Applied the accumulated lessons and they held.** `git add -A` in the isolated worktree
  (master clean, no stale-lock); out-of-context review; behavior verified live
  (`12_menu_newgame` + `09_editor`) not just by unit tests; `cargo test --workspace
  --no-run`. The reviewer independently traced the riskiest change (a `required -> Option`
  query widening) and proved the newly-matchable controllers can't reach a player ship.

## What went wrong

- **R1.1: a test was adapted to pass rather than re-pinning its invariant.** When the
  refactor made `WithheldVerbs` entity-agnostic, `disable_verb_is_inert_on_a_hull` was
  mechanically changed from "component absent on a hull" to "present-but-unread" - which
  still passes but no longer pins the real guarantee ("no gate reads it on a hull",
  which lives in the readers' `With<ControllerSectionMarker>` filters). Root cause: when a
  refactor changes an invariant's MECHANISM, the test needs the invariant re-pinned on the
  new mechanism, not the old assertion massaged to be true. Caught by review, fixed by
  asserting the hull is not matched by a `ControllerSectionMarker` query.

## What to improve next time

- During a refactor that changes how an invariant is enforced, re-derive and re-pin the
  invariant in the test on the new mechanism - do not just tweak the old assertion until
  it passes. (A refactor variant of `pin-the-fix-at-its-boundary`.)

## Action items

- [x] Lessons ledger: extended `pin-the-fix-at-its-boundary` with the refactor variant.
- Modding family: all shipped/planned. Remaining ready-to-flow: the bundle family
  (134115 ship kind -> 134119 loader -> 134123 base-as-bundle -> 134127 mods+demo).
