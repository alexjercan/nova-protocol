# AGENTS.md

Global `~/AGENTS.md` applies. This file defines project-specific instructions.

## Project

- Bevy 0.19 3D space shooter with native, WASM, editor, and scenario paths.
- Root assembly is `crates/nova_core/src/lib.rs` -> `AppBuilder`.
- Plugin order is Bevy -> input -> assets -> gameplay -> scenario -> UI -> debug.

## Workflow

- Work directly on `master` unless the user requests an isolated worktree.
- Use Tatr for requested tracked work. Keep one task for one request and its
  follow-up work.
- Give each requested task one scheduling tag: `backlog` at priority 0 or the
  current release tag.
- Keep proof, decisions, reviews, retrospectives, and research with the task.
- Use Sprout only when the user requests an isolated worktree.
- Stage explicit paths. Never leave the index staged across tool calls.
- Use local sources before network research.

## Conventions

- Prefer correct, simple, maintainable changes over compatibility machinery.
- Run Rust and Cargo through `nix develop --command ...` or inside the shell.
- Use the pinned nightly toolchain and `rustfmt.toml`.
- Use `#[expect(<lint>, reason = "...")]`, not bare `#[allow]`.
- Do not add workspace-wide pedantic, nursery, wildcard-import,
  redundant-pub-crate, needless-pass-by-value, or private-missing-doc lints.
- Put unit tests inline or in sibling `src/**/tests/`. Reserve `crates/*/tests/`
  for integration tests. Name tests as behavior statements.
- Do not share `CARGO_TARGET_DIR` across worktrees or exceed the job cap.
- Give exporting modules a `prelude` and export it from the crate root. Import
  through preludes, including inside the same crate.
- Name plugins `<Subsystem>Plugin` and system sets `<Subsystem>Systems`. State
  cross-plugin ordering explicitly.
- Build apps and examples with `AppBuilder`. Use seeded `bevy_rand` for gameplay.
- Author code and content in world units; one world unit is 10 m. Print every
  player- or creator-facing figure in meters, never as `u`.
- Write code that reads as its own documentation. Give public items a
  docstring. Comment inside a body only where the reason is not recoverable
  from the code; delete a comment that restates what the next line does.
- Keep module comments short. Explain ownership and constraints, not code or
  history.
- Run only affected checks. Do not run full workspace tests or Clippy unless
  requested. Inspect rendered and generated output when applicable.

## Changelog

`CHANGELOG.md` points here for these rules; they live nowhere else.

- Use the last RELEASE, not the last commit, as the changelog and documentation
  baseline.
- Keep one entry per released change, at most 200 characters once wrapped lines
  are joined.
- Collapse several pre-release revisions of one change into a single final
  entry. Omit bugs introduced and fixed inside the same release cycle.
- Group entries by subsystem. Mark format breaks with `**(breaking)**`.
- Re-read the whole `[Unreleased]` block after editing it more than once.
- Migration notes apply only to formats that shipped. Remove documentation for
  behavior that was removed before it ever shipped.
