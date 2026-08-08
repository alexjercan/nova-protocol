# Unify content tooling bins into one 'content' CLI with subcommands (gen/lint/audit)

- STATUS: CLOSED
- PRIORITY: 35
- TAGS: v0.7.0, tooling, refactor, cli

User request: one CLI for content-related tasks with flags/subcommands
instead of three separate bins. Scope FIXED by the user to the three
`nova_assets` content bins; `nova_meta_gen` and `nova_portal_gen` stay as
their own crates/bins (build/deploy infra, wired into Trunk + the deploy
workflow).

## The three bins today (all thin `main`s over library functions)

- `crates/nova_assets/src/bin/gen_content.rs` (31 lines): writes the
  builder-backed `assets/base/**/*.content.ron` via
  `nova_assets::scenario_generation::content_files()`. No args.
- `crates/nova_assets/src/bin/content_lint.rs` (71 lines): walks the
  content tree (`lint_walk::lint_content_tree`) or one mod
  (`--target <dir-or-id>` -> `resolve_target` + `lint_target`); prints
  findings, exits non-zero on any Error.
- `crates/nova_assets/src/bin/balance_audit.rs` (68 lines): the balance
  audit (`balance::audit_content_tree`, `partition_findings`,
  `shipped_acks`); prints sheets + graded findings, exits non-zero on any
  Error or stale ack.

## Verified at plan time

- CI does NOT invoke any of the three bins: `.github/**` and `Trunk.toml`
  have zero references. The gates are the TESTS `content_lint_gate`,
  `balance_audit_gate`, `content_ron_parity`, which call the library
  functions directly - so replacing the bins does not touch CI behavior.
- `clap` (4.5.48, derive) is declared per-crate (root package +
  nova_meta_gen), not as a workspace dep; nova_assets adds it directly
  like nova_meta_gen does.
- Doc references to the old `cargo run ... --bin <x>` commands (usage, to
  rewrite): scenario-system.md (5), guide-author-scenario.md (4),
  modding-ron.md (2), guide-make-a-mod.md (2). `LESSONS.md` (3) and
  the `docs/design/*.md` records are HISTORY (they record what a past
  task ran) - do NOT rewrite those; they are correct as of their date.
- `keeping-docs-in-sync.md` names no specific content bin, but its
  App-assembly/tooling rows are where a "content tooling is one CLI now"
  note belongs.

## Design

- One bin `crates/nova_assets/src/bin/content.rs`, clap-derive, name
  `content`, invoked `cargo run -p nova_assets --bin content -- <sub>`.
  Reads as "content gen", "content lint", "content audit".
- Subcommands:
  - `gen` - gen_content's body (no args).
  - `lint [--target <dir-or-id>]` - content_lint's body; keep the
    `--target` flag semantics verbatim.
  - `audit` - balance_audit's body (no args).
- Shared exit-code handling: each subcommand returns `ExitCode`; main
  dispatches. Keep the existing human-readable output lines (nothing
  asserts on them, but they are the dev-facing UX - keep them intact,
  only the tool-name prefix may change to "content lint:" etc.).
- Delete the three old bin files. Nothing depends on them by name.

## Steps

- [x] Add `clap = { version = "4.5.48", features = ["derive"] }` to
  `crates/nova_assets/Cargo.toml` [dependencies] (mirror nova_meta_gen).
- [x] Create `crates/nova_assets/src/bin/content.rs`: clap `Parser` +
  `Subcommand` enum { Gen, Lint { target: Option<String> }, Audit };
  three dispatch fns lifted verbatim from the old bins' bodies; module
  doc comment covering all three (mirror the deleted docs) with the new
  invocations.
- [x] Delete `gen_content.rs`, `content_lint.rs`, `balance_audit.rs`.
- [x] Grep the repo for `--bin gen_content|content_lint|balance_audit`
  and rewrite every USAGE reference to the new subcommand form (wiki
  pages listed above; leave LESSONS.md and design-doc HISTORY as-is,
  confirming each hit is history not instruction).
- [x] keeping-docs-in-sync.md: one line noting content tooling is a
  single `content` CLI (so a future bin change updates the right pages).
- [x] CHANGELOG.md: Internals & Tooling line (dev-facing command change).
- [x] Verify: `cargo run -p nova_assets --bin content -- gen` (regen,
  `git status` clean = byte-identical), `-- lint` (clean + 1 warn),
  `-- lint --target the-ledger`, `-- audit` (11/0/0/2 acked); the three
  gate tests still green; `cargo check --workspace --all-targets`; fmt
  last. A bad subcommand and `-- lint --help` print usage.
</content>

## Close-out (verification)

- `content -- gen`: wrote all base content files; `git status` shows NO
  `.content.ron` drift (only Cargo.lock from clap + the bin delete/add) -
  byte-identical to the old gen_content output.
- `content -- lint`: clean (1 warning, the pre-existing ledger_ch4 auditor
  dual-spawn); `content -- lint --target the-ledger`: same one warning,
  exit 0. `content -- audit`: 11 combat scenarios, 0 errors, 0 warnings,
  2 acked - identical to the old balance_audit.
- Error paths: unknown subcommand exit 2 (clap usage), bad `--target`
  exit 1 (matches old content_lint FAILURE), clean lint exit 0.
- Gate tests green: content_lint_gate 1/1, balance_audit_gate 2/2,
  content_ron_parity 2/2 (its REGEN message now names the new command).
- `cargo check --workspace --all-targets` clean (only the pre-existing
  proc-macro-error2 future-incompat note); fmt run last.
- CI unaffected: it invokes the gate TESTS, never the bins (verified
  `.github/**` + Trunk.toml carry zero bin references).
