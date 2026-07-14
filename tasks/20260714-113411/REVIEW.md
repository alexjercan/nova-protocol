# Review: prototype references + component modifications + serde-default

- TASK: 20260714-113411
- BRANCH: modding/section-prototypes

Reviewed out-of-context with two independent fresh-eyes agents (correctness+tests;
design+behavior+serde) plus implementer re-derivation of the load-bearing claim (the
shakedown verb-withhold equivalence: old `verbs.goto/lock/orbit=false` == new
`basic_controller_section` all-true + `DisableVerb(Goto/Lock/Orbit)`, STOP kept -
confirmed identical). Both reviewers: sound, no BLOCKER/MAJOR. The DisableVerb
accumulation fix (one `SectionDisableVerb(Vec)` vs per-verb inserts) is correct and
guarded end-to-end by `goto_unlocks_at_the_first_objective`.

## Round 1

- VERDICT: REQUEST_CHANGES (MINOR test + honesty fixes; no BLOCKER/MAJOR)

- [ ] R1.1 (MINOR) crates/nova_scenario/src/objects/modification.rs - the
  accumulation fix (merge N DisableVerbs into one component) is only unit-tested with
  a SINGLE verb (`disable_verb_clears_the_verb_on_a_controller`), which would pass
  even under the last-write-wins bug. The multi-verb case is covered only incidentally
  by the shakedown e2e (checks GOTO alone). Add a unit test that calls `insert_all`
  with `[DisableVerb(Goto), DisableVerb(Lock), DisableVerb(Orbit)]` on a controller
  and asserts all three cleared while `stop` stays true - pins the exact fix at the
  module boundary.
  - Response: Added `multiple_disable_verbs_all_apply` - inserts 3 DisableVerbs on a
    controller, asserts goto/lock/orbit all cleared and stop granted. Fails under a
    last-write-wins regression (only orbit would clear). 6 modification tests pass.
- [x] R1.2 (MINOR) tasks/20260714-113411/TASK.md (step 2, header, SPIKE.md) - the
  starter set lists `SetMass(f32)` but it was NOT shipped (deferred: mass is not a
  directly-settable section component - `base_section` feeds it to avian as
  `ColliderDensity`, so overriding it post-spawn is non-trivial). Step 2 is checked
  `[x]` with no record of the descope. Amend the task to strike `SetMass` from the
  shipped set and note it (and why) as deferred.
  - Response: Step 2 amended - SHIPPED (DisableVerb/SetHealth/Rename) vs DEFERRED
    (SetMass, with the ColliderDensity reason). Done.
- [x] R1.3 (NIT) tasks/20260714-113411/TASK.md (step 4) - "serde(default) across the
  config tree" has no scoping caveat, but the impl correctly touched only
  Option/Vec/HashMap fields; f32/bool domain-defaults (`health: 100.0`) were
  deliberately deferred (a bare `f32` default of 0.0 is not the domain default and
  would lose information). Note the scope + deferral.
  - Response: Step 4 amended - scoped to Option/Vec/HashMap; f32/bool domain-defaults
    deferred with the reason. Done.
- [x] R1.4 (NIT) tasks/20260714-113411/TASK.md - the narrative "parity proves the
  lowered result is byte-identical to today's configs" overstates: after the re-port,
  `scenario_ron_parity` guards the new AUTHORED form (prototype refs) against its
  builder, not the resolved runtime result against the old inline config. Lowering
  correctness is covered by the shakedown e2e + `12_menu_newgame`, not parity. Correct
  the narrative.
  - Response: Narrative corrected - parity guards the authored form vs its builder;
    lowering/equivalence is covered by the e2e + 12_menu_newgame. Done.
- [x] R1.5 (NIT) crates/nova_scenario/src/objects/spaceship.rs - `insert_spaceship_sections`
  now hard-requires `Res<GameSections>`; a ship spawned before `register_sections`
  inserts it would panic (distinct from the graceful missing-prototype skip). Not
  reachable in prod/editor/tests today, but make `SpaceshipPlugin` self-satisfying:
  `app.init_resource::<GameSections>()` in `build` (it derives Default; register_sections
  overwrites it with the catalog later). Removes the footgun, esp. for Inline-only spawns.
  - Response: `SpaceshipPlugin::build` now `init_resource::<GameSections>()` (empty
    default; register_sections overwrites with the catalog). Self-satisfying. Done.

## Round 2

- VERDICT: APPROVE

All R1 findings resolved and verified. R1.1: multi-verb accumulation test added and
passing. R1.2/R1.3/R1.4: task doc reconciled (SetMass descope, serde scope, parity
narrative). R1.5: SpaceshipPlugin self-provides GameSections. No new problems
introduced (test + one-line plugin change + docs); `cargo test --workspace --no-run`
and the modification suite green. Branch approved.
