# AGENTS.md

Global `~/AGENTS.md` applies.

## Project

- Bevy 0.19 3D space shooter with native, WASM, editor, and scenario paths.
- Root assembly: `crates/nova_core/src/lib.rs` -> `AppBuilder`.
- Plugin order: Bevy -> input -> assets -> gameplay -> scenario -> UI -> debug.

## Agent workflow

- Work directly on `master` unless the user requests an isolated worktree.
- Use tatr for tracked work. Create a task only when the user requests one.
- Use one task for one user request and its follow-up work. Create dependent
  tasks only when the user requests decomposition.
- Store task records under `tasks/`. Give each requested new task one scheduling
  tag: `backlog` at priority 0 or the current release tag.
- Keep proof-bearing DoD, NOTES, REVIEW, RETRO, and research spikes with the
  task. Inspect local sources before network research.
- Commit self-contained design explainers to the owning task. Do not leave them
  in a scratchpad or depend on external hosts.
- Put player and creator documentation under `web/src/`. Put developer
  documentation under `docs/`. Use `docs/keeping-docs-in-sync.md` as the routing
  map.
- Use an isolated worktree only when requested. Stage explicit paths and never
  leave the index staged across tool calls.

## Conventions

### Rust

- Run Rust and Cargo through `nix develop --command ...` or inside the shell.
- Use the pinned nightly toolchain and `rustfmt.toml`.
- Use `#[expect(<lint>, reason = "...")]`, not bare `#[allow]`.
  `nova_assets/src/portal/mod.rs` has the sole `missing_docs` exception.
- Do not enable workspace-wide pedantic, nursery, wildcard-import,
  redundant-pub-crate, needless-pass-by-value, or private-missing-doc lints.
- Put unit tests in inline `#[cfg(test)] mod tests`. Move large test modules to
  sibling `src/**/tests/`. Reserve `crates/*/tests/` for integration tests.
- Name tests as sentences that state behavior.
- Do not share `CARGO_TARGET_DIR` across worktrees or exceed the configured job
  cap.

### Modules and Bevy

- Give every exporting module a `prelude`. Export each module prelude from the
  crate root.
- Import through preludes, including within the same crate. Re-export by name
  when a glob can include an engine prelude.
- Name plugins `<Subsystem>Plugin` and system sets `<Subsystem>Systems`.
- State scheduling dependencies with `.before(...)` or `.after(...)`.
- Create a `SystemSet` only when another plugin needs an ordering handle.

### Nova behavior

- Reproduce each bug before its fix as an `examples/systems/` range.
- Every range assertion needs a nearby `nova_probe::probe_marker` with
  `outcome: <slug>` and the same slug in
  `crates/nova_probe_cli/tests/catalog_drift.rs`.
- File examples by audience:
  - `playable/`: a human can act through an affordance outside the
    `NOVA_AUTOPILOT` gate.
  - `systems/`: the probe asserts correctness.
  - `screenshots/`: the run produces documentation images.
- A free-fly camera is not a playable affordance. Playable examples may keep an
  autopilot, but must still work for a human or say they are autopilot-only.
- Use seeded `bevy_rand` for gameplay. Never use `rand::rng()`.
- Treat prototype, scenario, style, and asset IDs as runtime strings. Grep every
  renamed ID and run affected content.
- Put cross-crate IDs in the lowest crate already shared by all consumers. Do
  not add dependency edges for constants. Keep test and example IDs local.
- Build examples with `AppBuilder`, never hand-assembled `App::new()` and
  `DefaultPlugins`.
- Treat conversion to `AppBuilder` as behavioral work. It can add plugins,
  loading states, rendering, and races. Run the range with
  `probe run <name> --norender --correctness-only` and inspect every check.
- Edit Rust content builders, then run `content -- gen`. Never hand-edit
  `assets/base/**/*.content.ron`.

### Comments and documentation

- Start modules with at most three `//!` sentences: ownership, key constraint,
  and when to change the module.
- Document constraints and information declarations do not show. Do not narrate
  code or record history.
- Do not cite task artifacts in durable docs. `TODO(<task-id>)` is allowed for
  active tracked work.
- Explain a constant's value rather than repeating the value in prose.
- Use the last release, not the last commit, as the documentation and changelog
  baseline.
- Remove documentation for removed unshipped behavior. Migration notes apply
  only to formats that shipped.
- Ship code and invalidated docs in the same change. Route player experience to
  `/wiki/`, authored contracts to `/create/`, and mechanisms to the developer
  book.
- Leave a hole instead of speculative documentation for a mechanism being
  rewritten.

### Changelog and web

- Keep one changelog entry per released change, at most 200 characters after
  joining wrapped lines.
- Collapse multiple pre-release revisions into one final entry. Omit bugs
  introduced and fixed within the same release cycle.
- Re-read the full `[Unreleased]` block after editing it more than once.
- Group entries by subsystem. Mark format breaks with `**(breaking)**`.
- Keep static fallback prose inside every `data-widget` block.
- Source every documented game number from Rust and record its `file:line` in a
  comment.

## Verification

- Run only affected checks. Do not run full workspace tests or Clippy unless the
  user requests them; CI owns both.
- Open rendered and generated output. Exit status alone is not proof.

```bash
nix develop --command cargo check
nix develop --command cargo fmt --check
nix develop --command cargo test --lib -p <crate>
nix develop --command cargo run content lint
nix develop --command cargo run --features debug probe run <category>
cd web && npm run ci
```

Inspect probe `report.html` and `checks.json`. `SKIPPED` means unmeasured.
