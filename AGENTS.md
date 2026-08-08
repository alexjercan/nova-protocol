# AGENTS.md

Repository guidelines. Global `~/AGENTS.md` applies.

## Repository

Nova Protocol is a Bevy 0.19 3D space shooter with a modular ship editor,
native and WASM simulation, and scenario-driven combat.

- Root crate: CLI entry and `nova_core` re-export.
- Assembly: `crates/nova_core/src/lib.rs` -> `AppBuilder`.
- Order: Bevy -> input -> assets -> gameplay -> scenario -> editor/menu -> debug.
- States: `GameStates::{Loading, MainMenu, Playing}` and
  `GameAssetsStates::{Boot, Loading, Processing, Loaded}`.
- Scenario setup usually uses `OnEnter(GameAssetsStates::Loaded)`.
- Crate ownership: each `crates/*/src/lib.rs` module document.
- Rust style: `CONVENTIONS.md`.

Use crate preludes. Export new public items from the owning prelude.

## Commands

Run Rust and Cargo commands through `nix develop --command ...`, or enter
`nix develop` first. Do not share `CARGO_TARGET_DIR` across worktrees.

```sh
cargo run
cargo run --features dev
cargo run --example scenario_grammar
cargo check
cargo fmt
cargo test --lib -p <crate>
cargo run -p nova_authoring --bin content -- gen
cargo run -p nova_authoring --bin content -- lint
cargo run -p nova_probe_cli -- run <category>
scripts/serve-web.sh
scripts/preview-web.sh
```

- `debug`: debug tooling. `dev`: alias for `debug`.
- `--norender` and `--debugdump` require `debug`.
- Fresh clone: run `scripts/setup-hooks.sh`.

## Testing

- Prefer App-driven tests and `NOVA_AUTOPILOT` examples.
- Bugs: reproduce in the current tree first. Record fail-first proof in `TASK.md`.
- Features: add player-path harness coverage. Unit tests do not replace it.
- Extend a known-good rig with production scheduling, defaults, and config.
- Example catalog: root `Cargo.toml`; categories: `examples/`.
- Gameplay changes: `cargo run -p nova_probe_cli -- run <example>`.
- Inspect `report.html` and `checks.json`. `SKIPPED` means unmeasured.
- Touched crate: `cargo test --lib -p <crate>`. Use `--test <name>` for one
  integration test. Bare `-p nova_assets` links all integration test binaries.
- Do not run full `cargo test` or `cargo clippy` unless asked. CI owns both.
- Requested full suite:
  `env -u DISPLAY -u WAYLAND_DISPLAY cargo test --workspace --features debug`.
- Windowed suite: `cargo run -p nova_probe_cli -- run --all`.
- Do not raise `-j` above the `.cargo/config.toml` cap.

Probe details: `.claude/skills/probe/SKILL.md` and the dev wiki Performance page.

## Repository rules

- `nova_events` is the scenario and modding event vocabulary. In-code gameplay
  wiring can use observers and direct calls.
- `assets/base/**/*.content.ron` is generated. Edit Rust builders, run
  `content -- gen`, and commit both. Do not edit generated RON.
- Enable `#![warn(missing_docs)]` only on a fully documented crate.
- Keep `cargo doc --workspace --no-deps` warning-free.
- Verify prose against shipped behavior.
- Open rendered and generated output. Exit status alone is not proof.
- Use measured runtime values and derive rig invariants.
- Advertised config and UI require a verified producer, consumer, and
  preconditions.

## Shared checkout

- Worktrees: use the `sprout` skill. Do not create them by hand.
- Before each main-checkout commit: `git branch --show-current`.
- Stage explicit paths only. Never use `git add -A`.
- Do not leave the index staged across tool calls.
- Concurrent reads: `git show HEAD:<path>`.
- Background code edits: isolated sprout worktree.
- Squash landing: one command chain with `pwd`, branch check, squash merge, and
  commit.

## Agent workflow

- Tracker: `tatr`; records under `tasks/`; `/flow` drives the full cycle.
- Examples: declared location -> `examples/` or `scripts/` -> task folder -> ask
  once and cache in the task.
- Domain docs: `web/src/wiki/`; routing map:
  `web/src/wiki/dev/keeping-docs-in-sync.md`.
- Research: use `/understand`; keep `SPIKE.md` in the task folder; inspect the
  current tree before network research.
- Checks: proof-bearing DoD (`test:`, `cmd:`, `manual:`); gate with `tatr check`.
- Knowledge: `/home/alex/personal/agent-knowledge`; project=nova-protocol;
  tags=rust,bevy,game,protocol. Advisory only; failed writes stay in `RETRO.md`.

Each new task has one scheduling tag: `backlog` at priority 0, or the current
`vX.Y.Z` after release-priority review. Topical tags are additional.

Task records: `TASK.md` for story, steps, DoD, and notes; `SPIKE.md` for
research; `REVIEW.md` for review; `RETRO.md` for reflection; `NOTES.md` for the
design or fix record.

## Documentation

Ship code and invalidated docs together. Use
`web/src/wiki/dev/keeping-docs-in-sync.md` for the dependency map.

- User behavior: `CHANGELOG.md`, affected player wiki, and tutorial if
  first-flight behavior changes.
- Internals or formats: affected `web/src/wiki/dev/*.md`; mark format breaks
  `(breaking)`.
- Feature release: changelog and one `web/src/news/` post.
- Patch release: parent feature post under `## Point releases`.
- New or renamed wiki page: `web/webpack.config.js` and `web/src/wiki-pages.ts`.
- New news post: `NEWS_POSTS` and the `web/src/news.html` card.
- New visual feature: `.figure` placeholder and caption.
- Website check: `cd web && npm run ci`.

Keep changelog lines short and subsystem-grouped. State behavior and the key
name. Put rationale and examples in news.

`docs/` is temporary scratch. Before release, move durable material to the wiki
and retain only `docs/README.md`.

Version: root `Cargo.toml` -> `workspace.package.version`. Release procedure:
`web/src/wiki/dev/development.md`.
