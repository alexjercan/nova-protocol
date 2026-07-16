# Review: Cross-mod resource refs via `dep://<id>/<path>`

- TASK: 20260716-215423
- BRANCH: task-20260716-215423-cross-mod-refs

Reviewed with an out-of-context independent pass (fresh eyes on the diff) plus a
self re-verification of the load-bearing lint-refactor claim. The core design -
`dep://` classification, rewrite, base-rejection, declared-vs-available
distinction, and cross-domain (runtime / static lint / portal) consistency - is
correct as written. Findings are about test coverage and comment hygiene.

## Round 1

- VERDICT: REQUEST_CHANGES

- [x] R1.1 (MAJOR) crates/nova_assets/src/lib.rs (lint_walk::lint_bundle) - the
  static-lint domain ships with NO test that fails on revert. Two gaps: (a) the
  claimed double-report fix (content-membership loop hoisted out of the
  per-scenario loop) has no regression test, so re-nesting it would pass CI; and
  (b) no `dep://` ref is driven through `lint_bundle`/`lint_content_tree` at all,
  while the runtime and portal domains each got fresh `dep://` tests. Note the
  old nesting was worse than "double-report": a SECTION-ONLY bundle (zero
  scenarios) skipped the membership check entirely. Add a `lint_walk` test:
  a two-scenario bundle sharing one undeclared `self://` ref asserts EXACTLY one
  issue (pins the fix), plus a `dep://` cross-mod case (declared-ok /
  undeclared-resource / non-declared-dep / base).
  - Response: Added a `#[cfg(test)] mod tests` inside `lint_walk` (lib.rs) with
    four tests: `an_undeclared_self_ref_is_reported_once_for_a_multi_scenario_bundle`
    (asserts exactly 1 issue - fails as 2 if the loop is re-nested),
    `a_section_only_bundle_still_gets_its_refs_checked` (asserts 1 - fails as 0
    under the old nesting), `a_valid_dep_ref_across_the_walked_set_is_clean`, and
    `dep_ref_static_lint_flags_every_bad_case` (undeclared-resource /
    non-declared-dep / base). All pass; both regression tests fail on revert.
  - Verified: lib test count 59 -> 63, all green.
- [x] R1.2 (MINOR) crates/nova_assets/tests/mod_binary_resources.rs - every
  `dep://` test places the ref in the top-level `cubemap`. The whole point of the
  generic serde-value walk is catching `AssetRef` in NESTED fields (spawn
  actions, `render_mesh: Some(..)`, arrays). Add one `dep://` case with the ref
  buried in a nested `SpawnScenarioObject`/`Option` so the walk (and the portal's
  `ron::Value::Option(Some(..))` arm) is proven to reach it.
  - Response: Added `a_nested_dep_ref_is_rewritten` - the dep:// ref is an
    asteroid texture inside a `SpawnScenarioObject` action; asserts the merged
    scenario serializes `mods/art/textures/rock.png` and contains no `dep://`.
  - Verified: integration test count 6 -> 7, all green.
- [x] R1.3 (NIT) crates/nova_assets/src/mod_refs.rs (RefScope::violation, the
  dep "not available" arm) - a dependency that is installed and loaded but NOT
  enabled is absent from `by_id`, so it is reported "not available (not installed
  or not loaded)", which is misleading. Widen to include "or not enabled".
  - Response: Message widened to "not available (not installed, not loaded, or
    not enabled)".
  - Verified in the diff.
- [x] R1.4 (NIT) crates/nova_assets/src/lib.rs (the content-issues fold comment,
  "Fold in the undeclared-`self://`-resource findings gathered while flattening")
  - the loop now folds in both `self://` and `dep://` findings. Drop the
  `self://` qualifier from the comment.
  - Response: Comment updated to "resource-ref findings ... (undeclared
    `self://` and ungated `dep://<id>/` refs)".
  - Verified in the diff.
- [x] R1.5 (MINOR, ACCEPTED-AS-IS) crates/nova_assets/src/lib.rs
  (register_bundles) - `dep://` violations in SECTIONS are `error!`-logged but
  not gated (only `Content::Scenario` violations enter `ContentIssues`). This
  exactly mirrors the pre-existing `self://` behavior (the runtime gate is
  scenario-scoped by design, task 20260716-193949) and fails loudly at asset
  load, so it is not a regression this task introduces. Recording it as a known
  limitation; extending the gate to sections stays the gate's own follow-up.
  - Response: Accepted as-is - pre-existing scenario-scoped-gate behavior, not a
    regression. No change.
- [x] R1.6 (NIT, WON'T-FIX) crates/nova_portal_gen/src/lib.rs (collect_dep_refs)
  - the `dep://` split/label-strip is re-implemented on `ron::Value` rather than
  shared with `mod_refs::parse_leaf`. This is the SPIKE's deliberate "engine-free
  mirror" (the portal crate must not pull in bevy), the same split already made
  for `collect_self_refs`. Edge cases (`dep://art/`, `dep:///foo`, `dep://art`)
  were verified to classify identically today. Left as-is by design.
  - Response: Won't-fix by design (engine-free portal crate). No change.

## Round 2

- VERDICT: APPROVE

All Round 1 findings resolved or accepted with reasoning. Verified against the
new diff (commit e93648ef):

- R1.1: the four new `lint_walk` tests pass; the two regression tests
  (multi-scenario -> exactly 1 issue; section-only -> 1 issue) genuinely fail if
  the loop is re-nested. The `dep://` static-lint path is now covered.
- R1.2: `a_nested_dep_ref_is_rewritten` proves the serde walk reaches a deep
  `AssetRef`.
- R1.3 / R1.4: message and comment corrected.
- R1.5 / R1.6: accepted with reasoning (pre-existing behavior; deliberate
  engine-free mirror).

Full suites green: nova_assets lib 63, mod_binary_resources 7, content_lint_gate
2, nova_portal_gen 21; workspace `cargo check --all-targets` clean. No new issues
introduced by the Round 1 changes (tests + two doc-string tweaks). Approved.
