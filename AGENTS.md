# AGENTS.md

Repository guidance. Global `~/AGENTS.md` applies.

## Project

- Bevy 0.19 3D space shooter with native, WASM, editor, and scenario paths.
- Root assembly: `crates/nova_core/src/lib.rs` -> `AppBuilder`.
- Plugin order: Bevy -> input -> assets -> gameplay -> scenario -> UI -> debug.
- Style and API rules: `CONVENTIONS.md`.

## Agent workflow

- Tracker/epics: tatr records under `tasks/`; one scheduling tag per new task:
  `backlog` at priority 0 or the current release tag.
- Examples/retention: use declared examples or scripts, then the task folder;
  ask once and cache the answer in the task.
- Domain docs: `web/src/wiki/`; use
  `web/src/wiki/dev/keeping-docs-in-sync.md` as the routing map.
- Research/network: inspect the current tree first; keep `SPIKE.md` with the
  task; use network research only when local sources are insufficient.
- Checks/records: use proof-bearing DoD and task-local NOTES, REVIEW, and RETRO;
  run only the affected checks below.
- Changelog: one commit-title entry per change, 200 characters hard max; the
  detail goes to `web/src/news/<version>.md`, the task folder, or `docs/`.

## Rules

- Run Rust and Cargo through `nix develop --command ...` or enter the shell.
- Do not share `CARGO_TARGET_DIR` across worktrees or exceed the configured job
  cap.
- Use crate preludes. Export new public items through the owning prelude.
- Edit Rust content builders, then run `content -- gen`; never hand-edit
  `assets/base/**/*.content.ron`.
- Reproduce bugs first, as an `examples/systems/` range (`CONVENTIONS.md`).
  Features need player-path harness coverage; unit tests do not replace it.
- Open rendered and generated output. Exit status alone is not proof.
- Do not run full workspace test or clippy unless requested. CI owns both.
- Use sprout for worktrees. Stage explicit paths. Never leave the index staged
  across tool calls.
- Ship code and invalidated docs together. Follow the docs routing map.

## Checks

```bash
nix develop --command cargo check
nix develop --command cargo fmt --check
nix develop --command cargo test --lib -p <crate>
nix develop --command cargo run content lint
nix develop --command cargo run --features debug probe run <category>
cd web && npm run ci
```

Inspect probe `report.html` and `checks.json`. `SKIPPED` means unmeasured.
