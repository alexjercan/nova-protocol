# Retro: full missing_docs rollout on nova_scenario + nova_gameplay

- TASK: 20260721-121316
- BRANCH: docs/missing-docs-tail (landed ffa68045)
- REVIEW ROUNDS: 1 (out-of-context APPROVE, no findings)

Process only; see TASK.md close-out for the per-crate counts.

## What went well

- The hard gate held: both large crates reached EXACTLY 0 missing_docs before
  `#![warn(missing_docs)]` stayed in, verified per-crate by the impl AND the
  reviewer (lesson `lint-enabled-crate-must-be-zero-of-that-warning`). The whole
  workspace is now missing_docs-clean with the lint on every crate.
- The impl found + fixed a real rustdoc footgun during verify: a `///` summary
  on a `pub mod` line that ALSO carries `//!` inner docs makes rustdoc resolve
  that module's `//!` intra-doc links against the PARENT scope (silently broke 6
  pre-existing links - invisible to missing_docs and the build; only `cargo doc`
  caught it). Fix: bare `pub mod` for `//!`-carrying modules.
- Review verified doc ACCURACY against code, not just cargo-doc-clean: the
  flagged `SetControllerVerb` verb set (taken from the wiki) was confirmed to
  MATCH the `FlightVerb` enum exactly; 6 more spot-checks passed.

## What went wrong

- Fan-out race: the impl split the sweep across 4 parallel sub-subagents on ONE
  shared worktree. Their concurrent `--force-warn missing_docs` builds raced, so
  a per-agent "count == 0" self-check was unreliable - one agent reported done
  with 40 items still undocumented in actions.rs. The impl caught it by
  re-verifying with a SETTLED single build (polled mtimes). Right recovery, but
  the race is a real cost of fanning builds on a shared tree.
- MY slip (orchestration): the `sprout land -m "..."` body used backticks around
  `pub mod`, and BACKTICKS IN A DOUBLE-QUOTED `-m` ARE COMMAND SUBSTITUTION -
  bash ran `pub mod` (-> "command not found" -> empty), eating the term and
  leaving "a  line". Amended the tip via `-F`. No side effect (the substituted
  token was a harmless non-command), but a backticked `git`/`sprout`/`tatr`
  phrase WOULD have executed.

## What to improve next time

- Never put backticks (or `$`) in a double-quoted `git`/`sprout` `-m "..."` -
  use `-F <file>` (heredoc with a quoted delimiter) or single quotes for any
  commit message containing backticks/shell metacharacters.
- When fanning build-verified work across parallel agents on one worktree,
  the acceptance count must come from ONE settled build after all writes
  quiesce, not each agent's concurrent self-check.

## Action items

- [x] LESSONS.md: added `commit-msg-backticks-are-command-substitution` and
  `parallel-builds-race-the-lint-count`.
- Completes the 20260525-133033 rustdoc strand: the workspace is fully
  missing_docs-clean.
