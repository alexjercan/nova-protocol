# AGENTS.md

Nova Protocol: Bevy 0.19 3D space shooter. Modular ship editor, native and
WASM simulation, scenario-driven combat.

Start here:

- Read this file.
- Use crate preludes. New public items require prelude exports.

## Code map

Root crate: CLI entry and `nova_core` re-export. Main assembly:
`crates/nova_core/src/lib.rs` -> `AppBuilder`.

Share is that crate's percentage of the 145,611 `crates/*/src` lines
(2026-08-08). It is here so the size distribution is visible before you plan
anything: `nova_gameplay` is over half the workspace on its own.

| Crate | Share | Handles |
| --- | --- | --- |
| `nova_gameplay` | 54% | Sections, integrity, input, HUD, targeting, flight, AI, camera, NOVA OS UI bridges, `GameStates`. |
| `nova_assets` | 12% | Assets, content builders, `content` CLI, bundle merge, mod cache/downloads, portal client, catalog builder. |
| `nova_scenario` | 10% | Actions, events, filters, variables, objects, content lint. |
| `nova_menu` | 6% | Main/pause menus, settings, mods, portal UI, scenario picker. |
| `nova_probe_cli` | 5% | Host half of the run harness: spawns runs, grades artifacts, renders reports. Never linked into a game binary. |
| `nova_ui` | 3% | Shared theme and widgets. Zero `nova_*` dependencies - keep it that way. |
| `nova_probe` | 2% | In-game half of the run harness: the capabilities an example wires, and the wire format they write. |
| `nova_autopilot` | 2% | Automation drivers and the run-completion protocol; `bevy`-only. |
| `nova_os` | 2% | Terminal model, shell, app runtime. No UI ownership. |
| `nova_editor` | 2% | Ship editor and play-test transition. |
| `nova_debug` | 1% | `debug`-gated inspector, wireframe, overlays, and the example test harness. |
| `nova_events` | <1% | Game event kinds and entity identity components. |
| `nova_core` | <1% | Plugin assembly. Start here. |
| `nova_mod_format` | <1% | Engine-free mod wire and serde types. |
| `nova_modding` | <1% | Bevy asset loaders for the RON mod format: `ContentAsset`, `BundleAsset`, `InstalledCatalog`. One file. Not bundle merge, downloads or the portal - those are `nova_assets`. |
| `nova_events_macros` | <1% | The `EventKind` derive; `nova_events` is its only consumer. |
| `nova_info` | <1% | `APP_VERSION` from `build.rs`. |
| `nova_meta_gen` | - | Web asset `.meta` generator under `tools/`; no game dependency. |

Assembly order: Bevy -> enhanced input -> assets -> gameplay -> scenario ->
editor/menu -> debug tooling. States: `GameStates::{Loading, MainMenu, Playing}`
and `GameAssetsStates::{Boot, Loading, Processing, Loaded}`.
Scenario setup usually hooks `OnEnter(GameAssetsStates::Loaded)`.

## Commands

NixOS: run every Rust/Cargo command through `nix develop --command ...`, or
enter `nix develop` first. Never share `CARGO_TARGET_DIR` across worktrees.
`sccache` provides safe cross-worktree reuse.

```sh
cargo run
cargo run --features dev
cargo run --example scenario_grammar
trunk serve
scripts/serve-web.sh    # live dev: site + game + portal, watched, proxied
scripts/preview-web.sh  # one-shot static build in the real deploy layout
cargo check
cargo fmt
cargo run -p nova_assets --bin content -- gen
cargo run -p nova_assets --bin content -- lint
cargo run -p nova_probe_cli -- run player_path
```

Features:

- `debug`: debug tooling.
- `dev`: alias for `debug`.
- `--norender`, `--debugdump`: require `debug`.

Fresh clone: run `scripts/setup-hooks.sh`. Pre-commit blocks Rust changes when
`cargo fmt --check` fails.

## Testing

- Harness-first. Prefer App-driven tests and `NOVA_AUTOPILOT` examples.
- Bugs: failing current-tree harness first; record fail-first numbers in `TASK.md`.
- Features: player-path harness coverage. Unit tests support, not replace, it.
- Rigs: production scheduling, defaults, configuration. Extend a known-good rig.
- Reference tests: `crates/nova_assets/tests/gauntlet_course.rs`,
  `crates/nova_assets/tests/ledger_ch2_encounter.rs`.
- Example catalog: root `Cargo.toml`; categories under `examples/`.
- Category run: `cargo run -p nova_probe_cli -- run <category>` (or `--all`).
- Touched tests: run `cargo test --lib -p <crate>`. No feature-unification workaround.
  `--lib` is load-bearing: bare `-p nova_assets` also links its 22 integration
  test binaries. Add `--test <name>` to reach one integration guard.
- Gameplay changes: run `cargo run -p nova_probe_cli -- run <example>`.
- Probe output: inspect `report.html` and `checks.json`; `SKIPPED` means unmeasured.
- Full `cargo test` and `cargo clippy`: do not run locally unless asked. CI owns both. State when skipped.
- If asked to run the full suite, use the CI-equivalent headless form:
  `env -u DISPLAY -u WAYLAND_DISPLAY cargo test --workspace --features debug`
  (nothing in the suite opens a window now that probe owns the run gate; test
  apps use `MinimalPlugins`, so no audio device is touched). The windowed half
  is `cargo run -p nova_probe_cli -- run --all`, which CI runs as its own step.
  Never raise `-j` past the `.cargo/config.toml`
  cap - concurrent rust-lld links are what OOMs the box.

Probe details: `.claude/skills/probe/SKILL.md` and the dev wiki Performance
section.

## Code rules

Rust house style is `CONVENTIONS.md` at the repo root. Read it before writing
code. Only what is not style lives here:

- Global `~/AGENTS.md` applies.
- ASCII punctuation only. No AI commit attribution.
- `nova_events` is the scenario/modding event vocabulary - the game-event kinds
  a RON scenario can react to. Adding one exposes it to modders. It is not a
  general "no direct coupling" mandate: in-code gameplay wiring uses observers
  and direct calls, and that is correct.
- Base `assets/base/**/*.content.ron`: generated. Edit Rust builders, run
  `content -- gen`, commit both. Never hand-edit generated RON.
- `#![warn(missing_docs)]`: enable only on a fully documented crate.
- Keep `cargo doc --workspace --no-deps` warning-free.

Paid rules:

- Prose from the final diff. Re-read every claim against shipped behavior.
- Reproduce stale bug briefs against the current tree before scoping a fix.
- Open rendered/generated output. Green exit codes do not prove useful output.
- Author against measured runtime values; derive rig invariants.
- Advertised config/UI requires verified producer, consumer, and preconditions.

## Shared checkout

- Worktrees: `sprout` skill only. No hand-created worktrees.
- Main checkout before every commit: `git branch --show-current`.
- Main checkout staging: explicit paths only. Never `git add -A`.
- Never leave the index staged across tool calls.
- Squash landing: one atomic `pwd && git branch --show-current && git merge --squash <branch> && git commit`.
- Concurrent work: read repository facts with `git show HEAD:<path>`.
- Background code edits: isolated sprout worktree. Main checkout only for task/ledger records.
- Helper processes: record PID; kill by PID. Never `pkill -f`.
- Piped checks: preserve exit codes with bare commands or `set -o pipefail`.
- Edited artifacts: re-read after tool success.

## Agent workflow

- Tracker/epics: `tatr` records under `tasks/`; `/flow` drives plan -> work -> review -> compound -> land.
- Examples/retention: declared location -> existing `examples/` or `scripts/` -> task folder -> ask once and cache in the task.
- Domain docs: durable reference under `web/src/wiki/`; routing map in `web/src/wiki/dev/keeping-docs-in-sync.md`.
- Research/network: use `/understand`; keep `SPIKE.md` in the task folder; verify current tree before external research.
- Checks/records: proof-bearing DoD (`test:`, `cmd:`, `manual:`); gate with `tatr check`.
- Knowledge: central repo `/home/alex/personal/agent-knowledge`; project=nova-protocol; tags=rust,bevy,game,protocol. Advisory only; failed writes stay in RETRO.

Task scheduling:

- Every new task: exactly one scheduling tag.
- Unscheduled: `backlog`, priority 0.
- Scheduled: current `vX.Y.Z`; inspect release priorities before slotting.
- Topical tags are additional.

Task records:

| File | Purpose |
| --- | --- |
| `TASK.md` | Story, Steps, DoD, Notes. |
| `SPIKE.md` | Research. |
| `REVIEW.md` | Review rounds and verdict. |
| `RETRO.md` | Retrospective. |
| `NOTES.md` | Design/fix record. |

`docs/`: ephemeral scratch only. Before release: move durable reference
material to the wiki, then clear everything under `docs/` except its `README.md`.
Plans and durable records stay out of `docs/`.

## Documentation

Code and invalidated docs ship in the same task. Full dependency map:
`web/src/wiki/dev/keeping-docs-in-sync.md`.

| Change | Required surfaces |
| --- | --- |
| User-visible behavior | `CHANGELOG.md`; affected player wiki; tutorial when first-flight changes. |
| Internals or formats | Affected `web/src/wiki/dev/*.md`; mark format breaks `(breaking)`. |
| Feature release | Changelog plus one `web/src/news/` post. |
| Patch release | Parent feature post `## Point releases`; no separate post. |
| New/renamed wiki page | `web/webpack.config.js`, `web/src/wiki-pages.ts`. |
| New news post | `NEWS_POSTS`, `web/src/news.html` card. |

CHANGELOG lines: short, subsystem-grouped, behavior plus key name. No rationale
or worked examples. News owns narrative. New visual feature: add `.figure`
placeholder and caption.

Version: root `Cargo.toml` -> `workspace.package.version`. Release procedure:
`web/src/wiki/dev/development.md`. Website verification: `cd web && npm run ci`.
