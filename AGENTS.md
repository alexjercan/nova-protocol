# AGENTS.md
`~/AGENTS.md` applies. Root: `crates/nova_core/src/lib.rs` -> `AppBuilder`.
Plugin order: Bevy -> input -> assets -> gameplay -> scenario -> UI -> debug.

## Work
- Start with `nova-implement` unless the user requests another mode.
- Read code and real content before proposing changes. Use local sources first.
- State the end state, what dies, what may break, and what must fail loudly.
- Map one feature slice: owner, entry point, inputs, state, outputs, ordering,
  IDs, content, UI, docs, callers, and closest proof. Expand only on evidence.
- Stop for interfaces, names, defaults, precedence, ownership, and error policy.
  Show options, one consequence each, and a recommendation.
- Continue approved mechanical work. Do not ask only whether to continue.
- Do not edit while answering a question. At stops use `Delta`, `Verified`,
  `Next`; put one decision question on its own final line.
- Work on `master`. Use Sprout only when the user requests an isolated worktree.
- Stage explicit paths. Never leave the index staged across tool calls.

## Implementation gate
Before code edits, show exact paths and lines, existing types and functions,
proposed types, fields, functions, and signatures, a compact before/after call
graph, and the behavior or failure the proof will observe. Wait for approval
before adding a type, function, or test. Continue an approved design directly.

## Change policy
- Treat Nova as the sole consumer of internal crates and unshipped formats.
- Replace obsolete interfaces. Delete old paths, adapters, aliases, and tests.
- Do not add compatibility for unshipped behavior. Migrate shipped formats only.
- Require explicit content fields and known IDs; fail at lint, then load.
- Use `Option` only when absence has defined behavior. Keep a fallback only for
  a valid runtime condition; never use one to hide invalid authoring.
- Change the owning interface first. Use compiler errors to update every caller.
- Delete comments that repeat code or history. Explain a concrete reason,
  constraint, owner, failure, or debt. Avoid vague terms such as load-bearing.
- Use `TODO`, `FIXME`, `WTF`, or `XXX` only with the problem and removal rule.

## Evidence
- Report Claim, Evidence (`path:line`), Change, Blast radius, and Verification.
  Show proposed code and short graphs. Label unverified claims.
- Reproduce bugs before fixing them. Preserve before/after artifacts.
- Add permanent proof only for named, important, stable behavior or a reproduced
  failure. Do not add tests for coverage, prose, paths, inventories, or tests.
- Use unit tests for pure logic and validation; asserted examples for ECS;
  `nova-probe` for deterministic flows; `nova-bench` for player flows.
- Judge assertions, state, audits, logs, reports, and frames, not exit zero or
  pilot prose. Inspect rendered output for visual claims.
- Run affected checks only. Do not run workspace tests or Clippy unless asked.
- Never assert timing. Compare matched repeat sets to a named reference.

## Code and content
- Use `nix develop --command ...`, pinned nightly, and `rustfmt.toml`.
- Use `#[expect(<lint>, reason = "...")]`, not bare `#[allow]`. Document public
  APIs. Do not add workspace pedantic, nursery, wildcard-import,
  redundant-pub-crate, needless-pass-by-value, or private-missing-doc lints.
- Export module preludes from crate roots and import through them.
- Use `<Subsystem>Plugin` and `<Subsystem>Systems`; state cross-plugin ordering.
- Put unit tests inline or in `src/**/tests/`; use `crates/*/tests/` for
  integration tests. Name tests as behavior statements.
- Build apps with `AppBuilder`; seed gameplay through `bevy_rand`.
- Do not share `CARGO_TARGET_DIR` across worktrees or exceed the job cap.
- Author `nova_events` quantities in meters. Convert at named engine/grid edges.
  One engine unit is 10 m and one cell. Display meters.
- Edit Rust builders; regenerate, lint, and run content. Never hand-edit
  generated base RON. Search every runtime-ID consumer.
- Put shared IDs in the lowest shared crate. Keep fixture IDs local.
- Ship invalidated docs with code. Remove docs for removed unshipped behavior.
  Do not cite tasks in durable docs except an active `TODO(<task-id>)`.

## Changelog
- This section owns changelog policy. Other docs may point here but must not
  define conflicting rules.
- Add one entry per released change under `## [Unreleased]`.
- Base entries on the last release. Group them under the subsystem headings
  that the `CHANGELOG.md` banner names.
- Keep each entry at most 200 characters with its lines joined.
- Mark a format break `**(breaking)**`.
- Collapse pre-release revisions. Omit a bug that was introduced and fixed
  since the last release.
- Reread all `[Unreleased]` entries after several edits.
