# Retro: Tag base storyline chapter-heads as Nova Protocol 1/2/3 + regen content

- TASK: 20260723-095909
- BRANCH: content/nova-protocol-campaign-tags
- REVIEW ROUNDS: 1 (APPROVE, out-of-context, 2 non-blocking NITs)

## What went well

- `edit-the-builder-not-the-generated-ron` applied cleanly: edited the three
  builders, ran `content -- gen`, committed the regenerated RON in the same
  change. The `content_ron_parity` guard confirmed builder<->RON parity, and
  the reviewer's independent re-gen came back with a clean tree.
- Ran the NARROW guards first (`content_ron_parity`, `content -- lint`) before
  the full `cargo test -p nova_assets`. Both green immediately, which framed
  the later full-suite failure as clearly outside the change's blast radius
  instead of a scary "my content change broke a test" moment.
- Followed the merge-red discipline the moment the full suite went red: checked
  the failing test against master FIRST (confirmed on-branch = master, ran it
  there) before touching anything. It failed identically -> inherited, filed as
  its own task (103523), not mis-blamed on this branch.

## What went wrong

- Nothing in the change itself. The only friction was the inherited
  `content_lint_gate` failure, which cost ~2 extra test runs to triage. Root
  cause is upstream (the 2026-07-22 ch4 ending rework removed a warn the test
  still expects) - not this task's to fix, correctly routed to a new task.

## What to improve next time

- For a content-regen task, lead the verify step with the parity guard + lint
  explicitly (the real proofs), and only then run the broad suite - a broad-
  suite red is much easier to reason about once the targeted guards are known
  green. This is what happened here; worth making the default order.

## Action items

- [x] Filed 20260723-103523: fix the stale content_lint_gate ch4
  "mutually exclusive" assertion (inherited failure). Committed to master.
- No new ledger lesson: the two applied lessons
  (`edit-the-builder-not-the-generated-ron`, the merge-red / check-source-first
  rule) are already promoted.
