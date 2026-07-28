# nova_probe: commit-keyed probe-runs folders and baseline discovery

- STATUS: CLOSED
- PRIORITY: 79
- TAGS: v0.9.0,tooling,probe,performance

## Story

As a developer running probe checks across branches and commits, I want new
captures to land under `probe-runs/<short-commit>/...` by default, so before and
after artifacts are reusable, comparable, and tied to the code that produced
them instead of ad hoc folders like `probe-runs/before`.

This task owns the nova_probe workflow fix only. It does not decide whether the
July 29 FPS regression is real or fixed; that investigation remains in task
20260729-002952 and can use this folder layout once it lands.

## Steps

- [x] Inspect the current `crates/nova_probe` run, render, report, and baseline
      path code and record the exact current output behavior in NOTES.md.
- [x] Design and implement a commit-keyed `probe-runs/<short-commit>/<example>/...`
      default output layout for captures. Preserve `--out` as an output base and
      enough run metadata to map the short directory to the full commit SHA.
- [x] Keep compatibility with existing `probe-runs/<example>` and
      `probe-runs/before` folders while migration is in progress; do not hand-edit
      historical artifacts.
- [x] Make `--baseline` auto-detect the nearest previous commit hash directory
      under `probe-runs/` when no explicit baseline path is passed. It should
      ignore non-hash folders, prefer the closest ancestor of HEAD when git
      history is available, and report clearly when no baseline is found.
- [x] Fix `probe report` so it works with multiple examples/items in one
      invocation, matching the aggregate shape used by `probe-all` instead of
      erroring on multi-item output.
- [x] Add focused tests for commit-keyed path selection, baseline discovery,
      old-folder compatibility, and multi-item render behavior.
- [x] Update the relevant probe docs/help text so the new default folder layout
      and baseline behavior are discoverable.

## Definition of Done

1. test: capture output defaults to `probe-runs/<short-commit>/<example>/...`
   while existing explicit `--out` paths still work.
2. test: baseline auto-discovery picks the closest existing ancestor commit
   directory and ignores non-hash compatibility folders like `before`.
3. test: `probe report` accepts and renders multiple examples/items in one
   command.
4. cmd: `nix develop --command cargo test -p nova_probe` passes.
5. docs: relevant nova_probe help/docs describe commit-keyed result folders and
   automatic baseline discovery.
6. notes: `tasks/<id>/NOTES.md` records what changed, why this layout was chosen,
   compatibility tradeoffs, bugs encountered, and a short self-reflection.

## Notes

- Split from task 20260729-002952 at user request.
- Depends on no code changes from the FPS investigation, but the investigation
  can consume this once it lands.
- Intended default folder shape: `probe-runs/<short-commit>/<example>/...`.
- Implemented `--out` as a storage base, so the actual default run root is
  always `<out-base>/<short-commit>/`; without `--out`, the base is
  `probe-runs`.
- Updated the stale `nova_probe render` wording to the live `probe report`
  command while implementing the intended multi-item re-render behavior.
- Change summary: single and multi native specs now share the aggregate-shaped
  driver; output and baseline roots are commit-keyed; manifests carry
  `full_git_sha`; docs and CLI help describe the new base-directory behavior.
- Alternatives considered: preserving the old single-run exact-output path would
  reduce churn but keep two code paths and make `--out probe-runs` ambiguous.
  Treating every spec as a vector, including one item, made the output model
  consistent and easier to reason about.
- Difficulties: the initial compatibility pass would have made explicit
  `--baseline probe-runs/before` unusable. The resolver now only falls back to
  legacy non-hash roots for explicit baselines; auto-discovery still ignores
  them.
- Verification: `nix develop --command cargo test -p nova_probe` passed, with
  focused tests for output roots, baseline discovery, compatibility roots, and
  multi-dir report re-rendering.
- Self-reflection: start by collapsing special cases when an existing aggregate
  path can naturally represent the one-item case; keeping the old branch split
  in the first draft created unnecessary design tension.

## Flow State

- FLOW STEP: DONE
- PLAN STATUS: APPROVED
