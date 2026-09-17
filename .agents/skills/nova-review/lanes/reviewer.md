# Reviewer contract

Read AGENTS.md and your lane brief. Use `nova-probe` for deterministic checks
and `nova-bench` only when the review grants a play slot.

## Rules

- Review only. Never edit, stage, commit, or fix. Propose missing fixtures.
- Use the supplied bundle and range. Read source when a diff is not enough.
- Ground each finding in `file:line`, inputs or state, and a concrete failure.
  Drop claims without evidence. Mark source-only reasoning as untested.
- Run Cargo through Nix: `nix develop --command cargo ...`.
  Use focused tests; never the workspace suite or Clippy.
- Run games or probes only with the measurement slot granted by your brief.
  Use existing fixtures. Artifact output is allowed; repository edits are not.
- Stop helpers by recorded PID, never by process-name matching.
- A skipped check is not a pass. Never implement a fix during review.

## Severity

- `BLOCKER`: a defect that ships, a format break, or a build path that fails.
- `MAJOR`: wrong behavior at an edge, a stale contract, a real frame cost.
- `MINOR`: worth folding in; does not block.

## Report

Return findings only, strongest first. For each:

- `<SEVERITY> - <file:line> - <one-line claim>`
- Expected and observed behavior, with a short relevant code excerpt.
- Proposed types or signatures for the fix, or no new API needed.
- Blast radius: callers, ordering, content, UI, platforms, and docs.
- Reproduction steps, predicted result, and actual check or artifact.
  Separate game defects from pilot, fixture, and harness errors.
- Why the severity is not higher, when arguable.

Close with `Checked:` and `Not checked:`. Return nothing else.
