# Notes: unify content tooling into one `content` CLI

## What shipped

- One bin `crates/nova_assets/src/bin/content.rs` (clap derive), replacing
  `gen_content` / `content_lint` / `balance_audit`. Invocation:
  `cargo run -p nova_assets --bin content -- <gen|lint|audit>`, `lint`
  taking `--target <mod-dir-or-id>`.
- Each subcommand's body is lifted verbatim from the old bin; only the
  self-identifying output prefix changed (`content_lint:` -> `content lint:`,
  `balance_audit:` -> `content audit:`). No library code changed - the three
  bins were always thin `main`s over `scenario_generation::content_files`,
  `lint_walk::*`, and `balance::*`.
- `clap` added to `nova_assets` deps (mirrors nova_meta_gen). `Cargo.lock`
  updated in the same commit.

## Scope decision: which references to rewrite

The user fixed scope to the three `nova_assets` content bins;
`nova_meta_gen` and `nova_portal_gen` stay (build/deploy infra wired into
Trunk + the deploy workflow). Verified CI never invokes the three bins
(`.github/**`, `Trunk.toml` clean) - the gates are the TESTS
`content_lint_gate`, `balance_audit_gate`, `content_ron_parity`, which
call library functions, so this is a bin restructure + doc sweep with no
CI-behavior change.

Reference sweep, three buckets:

1. LIVE COMMAND INVOCATIONS (`cargo run ... --bin <old>`): all rewritten
   to the new subcommand form. These were broken by the rename. Fixed in
   guide-author-scenario.md, guide-make-a-mod.md, modding-ron.md, the
   CHANGELOG (both Unreleased lines), assets/base/sounds/README.md, and
   the `content_ron_parity` REGEN const (the message devs are told to run
   on drift).
2. PROSE NAMING THE DELETED ARTIFACT ("the `gen_content` bin", "the
   `content_lint` author CLI"): rewritten in lint.rs, lib.rs (x3),
   content_ron_parity.rs, content_lint_gate.rs, balance_audit_gate.rs,
   balance.rs, assets/mods/example/README.md. (balance_audit_gate.rs was a
   review R1.1 finding; balance.rs its twin, caught by the same-class
   sweep - grep "the `X` bin" prose, not just `--bin` invocations.)
3. BARE CONCEPTUAL MENTIONS ("content_lint warns on X", "authored via
   gen_content") in unrelated nova_scenario / nova_gameplay doc comments:
   LEFT AS-IS. Rationale: the CI gate test is still named
   `content_lint_gate`, and the library modules (`nova_scenario::lint`,
   `nova_assets::lint_walk`, `balance`) keep their names - so "content_lint"
   as a concept is not a ghost. These comments describe the lint's/
   generator's behavior, which is unchanged; only the manual invocation
   moved. Rewriting ~20 comments across two otherwise-untouched crates
   would be churn without correctness gain. (sweep-then-delete weighed:
   the deleted thing is the BIN, and every bin reference is fixed; the
   surviving concept keeps its name via the gate test.)

## keeping-docs-in-sync

Added a dependency-map row: "Content CLI: gen/lint/audit subcommands
(`nova_assets` bin `content`)" -> the three author/mod guides, so a future
subcommand change updates the right pages.

## Verification (see TASK.md close-out for the run lines)
</content>
