# Reviewer contract

Every lane obeys this. Read it with your lane brief.

## Rules

- You review. You never edit, stage, commit, or fix. Findings only.
- Work from the bundle paths you were given. Do not re-derive the range.
- Read the tree when the diff is not enough. Read a whole file rather than
  guess from a hunk.
- Ground every finding: `file:line`, and a concrete failure scenario - the
  inputs or state, then the wrong result. Drop what you cannot ground. A
  plausible smell is not a finding.
- Read `AGENTS.md` at the repository root. It is the house authority.
- Run Cargo through the Nix development shell:
  `nix develop --command cargo ...`.
- Never run the workspace test suite or Clippy. The suite exhausts memory on
  this machine and CI owns both. Use `cargo test -p <crate> --lib <filter>`.
- Do not run a rendered example or a measuring probe unless your brief grants
  the measurement slot.
- Stop a helper process by its recorded PID. Never match processes by name.
- Say what you did not check. A skip is not a pass.

## Severity

- `BLOCKER`: a defect that ships, a format break, or a build path that fails.
- `MAJOR`: wrong behavior at an edge, a stale contract, a real frame cost.
- `MINOR`: worth folding in; does not block.

## Report

Return findings only, strongest first. For each:

- `<SEVERITY> - <file:line> - <one-line claim>`
- The failure scenario.
- The actionable change.
- Why it is not higher, when the severity is arguable.

Close with `Checked:` and `Not checked:`. Return nothing else: your text is the
review, not a message to a person.
