# Retro: unify content tooling into one `content` CLI

- TASK: 20260717-212219
- BRANCH: refactor/content-cli (landed as squash 1d0439a7)
- REVIEW ROUNDS: 2 (R1 APPROVE with one MINOR, R2 verified)

## What went well

- Scoping fork surfaced BEFORE building: the user said "content related
  things", which genuinely could have meant 3 bins or 5 (the two *_gen
  crates are content-adjacent). One AskUserQuestion settled it (just the 3
  nova_assets bins) instead of guessing and rebuilding. The survey that
  fed the question - every bin in the workspace, who invokes each - was
  the cheap part that made the question precise.
- The load-bearing risk was "did the merged bin change behavior?", and it
  was retired by construction: each subcommand body is the old bin's
  verbatim, and the out-of-context reviewer diffed them line-by-line
  against the recovered `master:` bins. Verbatim-lift + independent diff
  beats "rewrote it cleanly and it looks right".
- Verified the actual invocation by RUNNING the tool before writing any
  command examples into 6 docs (measure-before-writing) - the `-- lint
  --target X` form is not obvious from the old `-- --target X`, and a
  guessed doc would have shipped a wrong command.
- CI-safety was checked, not assumed: grepped `.github/**` + Trunk.toml to
  confirm CI runs the gate TESTS, never the bins - so deleting the bins
  was provably a no-op for the build.

## What went wrong

- R1.1: the doc sweep missed "the `balance_audit` bin runs" in
  balance_audit_gate.rs (and its twin in balance.rs). Root cause: the
  final sweep greped the COMMAND form (`--bin gen_content|content_lint|
  balance_audit`) but a bare artifact-name mention ("the X bin") has no
  `cargo run`, so it slipped the pattern. The out-of-context pass caught
  it; a same-class in-session sweep (`grep "the \`X\` bin"`) then caught
  the twin the reviewer hadn't flagged. This is `sweep-then-delete`
  exactly (bumped x11) - a rename needs BOTH the command grep and the
  bare-name grep.
- Landing raced a busy master twice: the `--is-ancestor` guard failed
  (correctly) once, then master moved AGAIN between re-merge and land. Not
  a defect - the guard did its job each time and the extra commits were
  task-metadata only - but it cost two extra merge/verify cycles. Tax of
  landing into a shared checkout with parallel sessions.

## What to improve next time

- When renaming a tool/bin/command, run TWO sweeps up front: the command
  form (`--bin X`, `run X`) and the artifact-name prose (`the \`X\` bin`,
  `\`X\` CLI`). Fold both into the plan's doc-sweep step so it is planned,
  not review-discovered.
- On a fast-moving master, expect the land guard to bounce; re-merge and
  retry immediately rather than treating the first failure as an anomaly.

## Action items

- [x] LESSONS.md: sweep-then-delete x11 (rename = command-grep AND
  bare-name-grep variant); out-of-context-review-pass x30.
- No follow-up code tasks: the two *_gen crates stay separate by explicit
  user scope; nothing else outstanding.
