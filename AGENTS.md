# AGENTS.md

`~/AGENTS.md` applies. Root: `crates/nova_core/src/lib.rs` -> `AppBuilder`.
Plugin order: Bevy -> input -> assets -> gameplay -> scenario -> UI -> debug.

## Workflow

- Use Pair by default unless another mode is requested. Read code first.
  Continue approved plans and mechanical work; stop only at real decisions.
  Do not edit while answering a question. Use local sources before the network.
- Interfaces, names, defaults, and precedence are decisions. Show options,
  one consequence each, and a recommendation. Prefer code over abstract prose.
- At stops: `Delta`, `Verified`, `Next`. Put the question on its own final line.
- Work on `master`; use Sprout only for a requested isolated worktree.
- Use Tatr only for requested tracked work: one task per request and follow-up.
  Give it one scheduling tag: `backlog` at priority 0 or the current release.
  Keep proof, decisions, reviews, retrospectives, and research with the task.
- Stage explicit paths. Never leave the index staged across tool calls.

## Skills

Read `.agents/skills/<name>/SKILL.md`; do not wait for automatic discovery.
- `pair`: start unless opted out. `verify`: before change proposals/work/review.
- `probe`: examples, scenario probes, performance. `content`: content/IDs/RON.
- `docs`: docs, web, releases, or invalidated docs. `nova-review`: asked panel.
- `CLAUDE.md` imports this file; `.claude/skills/` links to `.agents/skills/`.
- Limits: AGENTS.md 80 lines; each SKILL.md 40, including metadata. All lines
  at most 80 characters. Cut repetition; do not split text to evade limits.

## Evidence

- Reports/proposals: Claim; Evidence (`path:line`, code); Change (proposed
  types, fields, signatures or none); Blast radius; Verification (steps/limits).
- Label unverified claims. Trace callers, crates, ordering, IDs, formats, UI,
  platforms, and docs. Use short code excerpts, not abstract prose.
- Reproduce before fixing; add an assertion that fails without the fix.
  Rerun afterward. Preserve before/after artifacts and state any blocked proof.
- Use a unit test if sufficient; otherwise propose examples/scenarios and an
  agent play of the flow. Run applicable flows; explain tests-only choices.
- Before play: fixture, revision, seed, budgets, pilot, output, actions,
  predicted results, pass/stop conditions. Easy scenes must keep the trigger.
- Judge assertions, state, audits, logs, and frames, not pilot prose or exit 0.
  Separate game, pilot, fixture, and harness errors. Unreached means untested.
- Run affected checks only; no full workspace tests or Clippy unless requested.
  Inspect rendered/generated output. Headless runs do not prove appearance.
- Register systems examples in `Cargo.toml`. Put `outcome: <slug>` markers by
  assertions; register each in `crates/nova_probe_cli/tests/catalog_drift.rs`.
- Never assert timing. Compare repeat sets against a named, matched reference.

## Conventions

- Prefer simple, correct, maintainable changes, not compatibility machinery.
- Require explicit content fields and known IDs; errors at lint, then load.
  Reserve `Option` for documented overrides that define what absence means.
- Rust/Cargo: `nix develop --command ...`, pinned nightly, `rustfmt.toml`.
- Use `#[expect(<lint>, reason = "...")]`, not bare `#[allow]`. Document public
  items. Comments explain ownership/constraints or reasons, not code or history.
- No workspace pedantic, nursery, wildcard-import, redundant-pub-crate,
  needless-pass-by-value, or private-missing-doc lints.
- Export module preludes from crate roots; import through them, even internally.
- Use `<Subsystem>Plugin` and `<Subsystem>Systems`; state cross-plugin ordering.
- Unit tests: inline or sibling `src/**/tests/`; integration: `crates/*/tests/`.
  Name tests as behavior statements.
- Build apps/examples with `AppBuilder`; seed gameplay through `bevy_rand`.
- Do not share `CARGO_TARGET_DIR` across worktrees or exceed the job cap.
- Author `nova_events` quantities in meters; convert at named engine/grid edges
  via `to_engine`/`from_engine`. 1 unit = 10 m = 1 cell. Display meters.
- Edit Rust builders, regenerate, lint, and run content. Never hand-edit
  generated `assets/base/**/*.content.ron`. Search runtime-ID renames/consumers.
- Shared IDs: lowest shared crate; no edge for a constant. Fixture IDs: local.
- Ship invalidated docs with code; read `docs/keeping-docs-in-sync.md`.
  No task citations in durable docs except active `TODO(<task-id>)`.

## Changelog

- Use the last RELEASE baseline. One entry per released change, <=200 joined
  characters, grouped by subsystem. Mark format breaks `**(breaking)**`.
- Collapse pre-release revisions; omit bugs introduced and fixed in that cycle.
  Reread all `[Unreleased]` after multiple edits. Migrate shipped formats only;
  remove docs for removed unshipped behavior.
