# Retro: Cross-mod resource refs via `dep://<id>/<path>`

- TASK: 20260716-215423
- OUTCOME: landed (squash `911f7331`), review APPROVE round 2, full suite green.

## What was built

A second mod resource-ref scheme, `dep://<id>/<path>`, that resolves a DECLARED
dependency's shipped resources against that dependency's own folder - the
cross-mod analog of `self://`. Gated in all three domains (runtime merge, static
lint, portal generator), mirroring the `self://` machinery behind one unified
`RefScope`. Design decision recorded in SPIKE.md; a shipped art-pack dogfood was
seeded as task 20260716-231341.

## What went well

- **Mapping before coding.** An Explore agent + direct reads pinned the entire
  `self://` pipeline (mod_refs, register_bundles, lint_walk, portal_gen, deps
  resolver, tests, docs) before a line changed. The unified `RefScope` +
  `rewrite_refs`/`resource_ref_violations` fell out naturally because both
  schemes share the same generic serde-value walk; no per-field code.
- **The spike earned its tag.** Choosing `dep://` over the task's suggested
  `mod://` avoided a real footgun: `mods://` (with an `s`) is already a LIVE
  asset source, so `mod://` is one keystroke from silently loading real bytes.
  Rejecting `dep://base` (base uses bare paths) came from actually tracing base's
  `resource_base`, not from assumption.
- **Independent review paid off.** An out-of-context review pass (fresh eyes on
  the diff) flagged the one real gap - the lint domain shipped with no test that
  fails on revert. Verifying that finding surfaced a WORSE latent bug than the
  one I'd claimed: the old content-membership loop was nested in the per-scenario
  loop, so it both double-reported for multi-scenario bundles AND skipped
  section-only bundles entirely. The refactor fixed both; regression tests pin it.

## What went wrong / difficulties

- **Landed a warning.** An unused `use std::collections::HashSet` (the test only
  uses `HashSet` via `.collect()` type inference, never by name) rode into the
  squash commit and was caught only by the editor diagnostic AFTER landing,
  forcing a follow-up sprout. A warnings-surfaced build on the new test module
  before landing would have caught it.
- **Merge with a parallel task.** Task 20260716-215513 (example-mod
  consolidation) landed on master mid-task and renamed the `self://` test
  fixtures `variety` -> `example` while I had rewritten the same functions' API.
  The conflict resolved cleanly (take my API + master's naming), but it was a
  reminder to expect churn when a parallel task touches the same files.
- **Inherited red from master.** The merge surfaced TWO tests failing ON MASTER:
  215513 renamed the shipped `demo` mod to `example` but left
  `id_colliding_with_shipped_catalog_is_rejected` and
  `install_guards_reject_shadowing_and_double_install` asserting against the
  now-nonexistent `demo` shipped id. Confirmed pre-existing via `git show
  master:<file>`, then fixed the stale fixtures as merge integration so this
  branch could land green rather than spreading the red.

## What to improve next time

- Run a warnings-surfaced build (`cargo build`/`clippy`, not just filtered test
  output) on new or changed modules BEFORE the squash-land, not after.
- When a post-merge suite goes red, `git show master:<file>` the failing test
  first to decide whether your change caused it or you inherited it - cheap, and
  it turns "did I break this?" into a definite answer.
