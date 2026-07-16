# Review: Remove deep mod-content behavior tests from core CI

- TASK: 20260716-155830
- BRANCH: refactor/drop-mod-content-tests

## Round 1

- VERDICT: APPROVE

Verified with fresh eyes rather than trusting the close notes:

- The coverage-audit delta in TASK.md matches the code: the two bridge
  pins cited really exist (area.rs, asteroid.rs) and filters.rs really
  had zero tests before this branch - the deleted content tests were the
  only exercise of composition/fails-closed/increment semantics.
- Mutation evidence, run by this review: flipping the fails-closed arm
  to fail-open turned expression_filter_fails_closed_on_an_undefined_
  variable red while the composition test stayed green (correctly - its
  guard variable IS defined); restored, 3/3 green. The new pins can fail.
- Delivery probes ride inside each null assertion (the guarded action
  stayed inert WHILE the probe counted the dispatch), satisfying the
  delivery-guards-on-null-assertions rule in the same test.
- Sweep confirmed: the only non-historical reference to the deleted
  files was broadside_assault.rs's header, updated to state the
  base-vs-mod coverage policy; webmods/gauntlet README never cited the
  tests; no CI config names test targets.
- webmods_validation (the generic mod gate) green; check --all-targets
  and fmt green. Solo `-p nova_scenario` still does not compile
  (pre-existing feature-unification constraint, not this branch).

No findings.
