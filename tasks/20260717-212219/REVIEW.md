# Review: unify content tooling into one `content` CLI

- TASK: 20260717-212219
- BRANCH: refactor/content-cli

## Round 1

- VERDICT: APPROVE (one MINOR doc-comment miss, non-blocking; implementer
  addressing it - Round 2 verifies)

Review basis: shared-session risk mitigated with an out-of-context review
pass (fresh-context agent over the raw diff + spec), its load-bearing
claims re-verified in-session. Independent verifications performed:

- Behavior preservation: recovered the three deleted bin bodies from
  `git show master:...` and diffed line-by-line against the new
  subcommands. gen (path build, content_files loop, panic messages),
  lint (None/Some branches, resolve_target, no-mod error, errors counter,
  both exit codes), audit (audit_content_tree, partition_findings,
  shipped_acks, active/acked/stale loops, `errors > 0 || !stale.is_empty()`
  stale-ack-fails-CI, exit code) are VERBATIM - only the self-identifying
  output prefix changed (`content_lint:` -> `content lint:`, etc.).
- Tools run (bare, real exit codes): gen -> no *.content.ron drift
  (byte-identical); lint -> clean 1 warn, exit 0; audit -> 11/0/0/2 acked,
  exit 0; lint --target the-ledger -> clean exit 0; lint --target
  no_such_mod -> exit 1 (matches old FAILURE); bogus subcommand -> exit 2
  (clap usage); extra arg -> exit 2. clap is a superset of the old raw
  `--target X` parser (also accepts `--target=X`), no regression.
- Gate tests: content_lint_gate 1/1, balance_audit_gate 2/2,
  content_ron_parity 2/2 (its REGEN message now names the new command).
- Doc sweep completeness: no leftover `--bin gen_content|content_lint|
  balance_audit` command invocations anywhere in web/ or docs/design/;
  all remaining bare mentions outside tasks/ + LESSONS.md are the
  deliberately-left conceptual ones per NOTES.
- Cargo: clap 4.5.48 in nova_assets/Cargo.toml, Cargo.lock line in the
  same commit; check --workspace --all-targets clean (only the
  pre-existing proc-macro-error2 note). `content` bin name is
  collision-free; clap structure matches nova_meta_gen's idiom; no
  orphaned [[bin]] tables or dead code.

- [x] R1.1 (MINOR) crates/nova_assets/tests/balance_audit_gate.rs:2 - the
  module doc still names the deleted artifact: "the `balance_audit` bin
  runs". Its sibling content_lint_gate.rs was updated but this one was
  missed (the final sweep grepped `--bin <old>` COMMAND invocations, and
  this is a bare "the X bin" prose mention with no `cargo run`). Fix:
  "the `content` CLI's `audit` subcommand runs" (matching the
  content_lint_gate.rs wording).
  - Response: fixed - balance_audit_gate.rs:2 now reads "the `content`
    CLI's `audit` subcommand runs". A follow-up sweep for the same class
    (`grep "the \`X\` bin"` prose across live files, not just `--bin`
    command invocations) caught one TWIN the round missed: balance.rs:36
    ("the `balance_audit` bin prints the full table"), also fixed. NOTES
    bucket-2 list updated with both. Remaining `docs/design/*.md`
    references are left per the stated design-doc-is-history policy.

### Merge-integration note (for landing, not a task defect)

The branch is behind master: master gained a `bevy_common_systems`
v0.19.0 -> v0.19.1 bump (+ a docs commit) after this branch was cut. Merge
master into the branch and re-verify before squash-landing, so the CLI
change rides on top of the v0.19.1 fix rather than appearing to revert it.

## Round 2

- VERDICT: APPROVE

R1.1 verified fixed: balance_audit_gate.rs:2 now names the `content` CLI's
`audit` subcommand. The implementer's same-class sweep also caught and
fixed a twin the round missed (balance.rs:36). Re-swept live files
(*.rs/*.md/*.toml outside tasks/ + LESSONS.md) for both `--bin <old>`
command invocations AND "the `<old>` bin" prose: none remain except the
deliberately-left conceptual mentions and design-doc records per NOTES.
No behavior change in this round (doc comments only); the bin, gate tests,
and CLI behavior are unchanged from Round 1's green verification.
