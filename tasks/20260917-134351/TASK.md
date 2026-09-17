# Delete compatibility machinery and low-value verification

- STATUS: OPEN
- PRIORITY: 100
- TAGS: v0.14.0, refactor, verification

## User notes

- Treat Nova as the sole consumer of its internal crates and unshipped formats.
- Prefer breaking replacement and compiler-assisted refactors.
- Delete obsolete paths instead of adding compatibility layers.
- Reject missing required content instead of inventing fallback values.
- Keep runtime recovery only when it is intentional game behavior.
- Remove comments that narrate code or use vague agent language.
- Keep comments only for ownership, constraints, reasons, or explicit debt.
- Remove tests that pin prose, incidental paths, inventories, or implementation.
- Prefer unit tests for pure logic and validation.
- Prefer asserted examples, probe runs, bench play, and inspected frames for game
  behavior, player flows, and visuals.

## Delivery

1. Inventory compatibility adapters, implicit defaults, stale branches, weak
   comments, task citations, and low-value tests. Record each candidate and its
   consumers before deletion.
2. Separate invalid-authoring fallbacks from deliberate runtime behavior. Do not
   delete a runtime fallback only because its variable or comment says fallback.
3. Clean one subsystem at a time. Change the intended interface first, use
   compiler errors to find callers, update owned content, and delete dead code.
4. Replace weak verification only when the behavior still needs protection. Use
   the cheapest proof that observes the real failure mode.
5. Run affected checks only. Inspect generated content and rendered output when
   the claim depends on them.
6. Keep behavior changes, proof, decisions, and remaining limits in this task.

## Execution protocol

Use this file as the work queue. Do not create separate Tatr tasks.

The main agent is the orchestrator. Process one unchecked item at a time in the
listed order:

1. Create one Sprout worktree for the item with
   `sprout new compatibility-cleanup-<NN> --task 20260917-134351`.
2. Start exactly one parallel worker in the path printed by Sprout. The worker
   uses `nova-implement`, reads the current code, and returns the implementation
   gate before editing.
3. The orchestrator checks the gate against this task. It may approve work that
   follows an interface and behavior already decided here. It must stop and ask
   the user about a new interface, name, default, precedence, owner, error rule,
   or permanent test.
4. After approval, let the same worker implement and verify the item. Do not
   start another item in parallel.
5. The orchestrator reviews the diff and the actual proof output. Reject broad
   cleanup, compatibility replacements, unrequested tests, and claims based
   only on exit status.
6. Run `sprout sync compatibility-cleanup-<NN>` and repeat affected verification
   after synchronization.
7. In the worktree, mark the item complete and add a short completion record
   with the changed paths, observed proof, and any retained limit.
8. From the main checkout, land and remove it with
   `sprout land compatibility-cleanup-<NN> --remove -m "<subject>"`.
9. Confirm the landed commit and clean state before starting the next item.

A worker must not edit the parent task outside its own current item. A failed or
blocked item stays unchecked. Record the blocker below it and stop the queue.

## Subtasks

- [ ] **01 - Delete retired nova-probe CLI verb shims.**
  Owner: `crates/nova_probe_cli/src/native/cli.rs:302-416,796-820`.
  Delete the hidden `Trace`, `Sweep`, `Web`, and `Profile` variants,
  `retired_alias`, and tests for their custom retirement errors. Clap then owns
  the unknown-command failure. Verify focused CLI parser behavior and help.

- [ ] **02 - Delete the legacy probe baseline-root fallback.**
  Owner: `crates/nova_probe_cli/src/native/paths.rs:78-91,191-210`.
  Caller: `crates/nova_probe_cli/src/native/sweep.rs:79`.
  Remove `allow_compat_root` and support for old non-hash `probe-runs` roots.
  Verify explicit and automatic resolution both require a commit directory.

- [ ] **03 - Require the current probe-run manifest.**
  Owner: `crates/nova_probe_cli/src/evaluation/manifest.rs:57-119,153-181`.
  Stop synthesizing missing `started_unix`, `full_git_sha`, `armed` fields, and
  pass booleans. Delete the legacy-manifest proof. Verify an incomplete manifest
  fails with the missing field name and a current manifest round-trips.

- [ ] **04 - Delete frametime CSV v1 support.**
  Owners: `crates/nova_probe/src/stats.rs` and
  `crates/nova_probe_cli/src/report/mod.rs`.
  Delete the 11-column parser, `CSV_HEADER_V1`, its tests, and the report's
  directory-derived renderer fallback. Keep v2-v4 for item 05. Verify v1 is
  rejected and current rows retain renderer identity.

- [ ] **05 - Delete frametime CSV v2 and v3 support.**
  Owners: `crates/nova_probe/src/stats.rs` and current-schema consumers in
  `crates/nova_probe_cli`.
  Make v4 the only accepted header and row shape. Remove dead unknown-metadata,
  pre-profile, and pre-cluster paths. Verify v2 and v3 fail clearly and v4
  parsing and reporting retain their stable behavior.

- [ ] **06 - Delete unused `enabled_by_default` mod machinery.**
  Owner: `crates/nova_mod_format/src/lib.rs:125-152`.
  Runtime consumer: `crates/nova_assets/src/mod_set.rs:261-288`.
  Update all constructors, fixtures, generated or authored content, modding
  docs, and the v0.14.0 changelog. A fresh install enables only the base mod;
  the removed catalog key must fail as unknown. Verify the fresh-install mod
  set and content lint.

- [ ] **07 - Require an explicit scatter-ring center.**
  Owner: `crates/nova_scenario/src/actions/spawn.rs:214-222`.
  Delete `serde(default)` and the omitted-center compatibility proof. Update all
  `ScatterRegion::Ring` builders in the editor, lint fixtures, examples, and
  base authoring. Regenerate base RON. Verify omitted `center` fails to
  deserialize, then run generated-content parity and content lint.

- [ ] **08 - Resolve `RenderMeshTransform` compatibility defaults.**
  Owner: `crates/nova_ship/src/sections/base_section.rs:277-309`.
  Existing compatibility proof:
  `crates/nova_ship/src/sections/turret_section/config.rs:354-392`.
  Classify omitted `position`, `rotation`, and `scale` as valid partial
  authoring or obsolete format compatibility. Recommendation: preserve partial
  transforms as intentional authoring, but delete pre-field compatibility prose
  and tests that protect no shipped format. This item has an explicit decision
  stop if implementation would make any field required.

- [ ] **09 - Remove task citations from production sources.**
  Scope: production Rust and Cargo files under `crates/**` that cite a completed
  task. Preserve concrete reasons, constraints, owners, and failure rules;
  delete historical narration. Keep an active `TODO(<task-id>)` only when it
  states the problem and removal rule. Verify the citation search is empty for
  production sources and run affected formatting and compile checks.

- [ ] **10 - Remove task citations from system examples.**
  Scope: `examples/systems/**`. Preserve fixture contracts, assertions, and
  failure conditions without task provenance. Delete comments that only narrate
  implementation history. Verify the citation search is empty for this scope
  and run the example catalog check plus checks for directly changed examples.

- [ ] **11 - Remove task citations from playable, screenshot, and asset fixtures.**
  Scope: affected files under `examples/playable/**`,
  `examples/screenshots/**`, and `assets/**`. Preserve visual acceptance rules
  without historical references. Edit Rust builders and regenerate owned base
  content instead of hand-editing generated RON. Verify the citation search,
  example catalog, generated-content parity, and any affected visual proof.

- [ ] **12 - Reconcile cleanup documentation and close the epic.**
  After items 01-11, rerun the compatibility, task-citation, stale-comment, and
  weak-test inventories. Update only invalidated docs and describe actual
  breaking changes in the v0.14.0 changelog. Record deliberately retained
  runtime fallbacks and shipped persistence migrations. Compare the result to
  every `Done when` condition before closing this task.

## Done when

- Required internal and content data fails clearly when absent or unknown.
- Removed internal APIs have no compatibility wrappers or dual paths.
- Remaining fallbacks document valid runtime behavior.
- Remaining comments state a concrete reason, constraint, owner, or debt.
- Remaining tests protect stable behavior rather than implementation trivia.
- Affected gameplay flows pass their asserted example, probe, or bench evidence.
- Invalidated docs and the v0.14.0 changelog describe the resulting behavior.
