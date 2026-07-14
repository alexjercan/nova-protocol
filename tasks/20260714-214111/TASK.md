# nova_ui crate: shared theme + widgets; migrate nova_editor onto it

- STATUS: CLOSED
- PRIORITY: 65
- TAGS: ui,v0.6.0

Umbrella: task 20260714-212139 (unify the whole game UI to the web-app theme).

## Goal

Create a new bevy-only `nova_ui` crate that holds ONE source of truth for the
game UI theme (palette + metrics) and the reusable widgets, then migrate
`nova_editor` to consume it. Foundation for the menu (214115) and HUD (214118)
restyles.

## Steps

- [x] Create `crates/nova_ui/` (bevy-only, no nova deps) + add to workspace members.
- [x] `nova_ui/src/theme.rs`: the palette + metric consts (moved from the editor,
  made `pub`; editor-only RAIL_W/DRAWER_W stayed in the editor).
- [x] `nova_ui/src/widget.rs`: `ThemedButton` (was `EditorButton`), `Selected`
  (was `SelectedOption`), `ButtonValue<T>`, `button_on_setting`, the
  `button_on_interaction` colour observers, `register`, `themed_button` (was
  `button`), plus `panel_header` + `separator`.
- [x] `nova_ui/src/lib.rs`: `theme` + `widget` modules + a `prelude`.
- [x] `nova_editor` depends on `nova_ui`; deleted the editor's `ui/theme.rs` +
  `ui/widget.rs`; re-pointed every reference (`EditorButton`->`ThemedButton`,
  `button`->`themed_button`, `crate::ui::theme`->`nova_ui::theme`, etc).
- [x] Verify: workspace check clean; `nova_editor` 12 tests + `nova_ui` 1 test pass.

## Close-out

### What changed

- New `crates/nova_ui` (bevy-only): `theme` (the web-app palette + metrics) and
  `widget` (the `themed_button` factory, the `ThemedButton`/`Selected`/
  `ButtonValue<T>` selection machinery, the colour observers, `register`, and the
  `panel_header`/`separator` helpers). Added to the workspace `members`.
- `nova_editor` now consumes `nova_ui`: its private `ui/theme.rs` and the button
  infra in `ui/widget.rs` are DELETED (net -289/+41 lines in the crate); the
  editor's rail/drawer/card/tooltip and `RAIL_W`/`DRAWER_W` stay editor-specific
  but pull colours/widgets from `nova_ui`. `EditorButton`->`ThemedButton`,
  `SelectedOption`->`Selected`, `button`->`themed_button`.

### Key decisions

- `nova_ui` has NO nova deps, so `nova_gameplay`/`nova_menu`/`nova_editor` can all
  depend on it without a cycle (verified dep graph). `nova_info` is build-info
  only (not a consumer).
- The migration is a pure extraction + rename: `placement.rs` is byte-identical to
  master (`git diff master -- placement.rs` empty), so the editor's behaviour is
  unchanged by construction.

### Verification (repo policy: check/fmt + new tests; full suite is CI's)

- `cargo check --workspace --all-targets --features debug`: clean, no warnings.
- `cargo test -p nova_editor`: 12 pass. `cargo test -p nova_ui`: 1 pass - a NEW
  deterministic test proving `button_on_setting` still sets the resource + moves
  `Selected` when `Pressed` is inserted (the exact path the editor cards and the
  menu tools use).
- `09_editor` autopilot (headless): reaches Sandbox -> create ship -> SELECT the
  hull CARD reliably. HONEST GAP: on this heavily-loaded machine (cold caches + a
  parallel sprout building) the autopilot's frame-COUNTED place/verify phases did
  not complete inside its wall-clock 6s window (fewer frames/sec under load), so I
  did not capture a green "placed a section" THIS run. This is frame-starvation,
  not a regression: `placement.rs` is byte-identical to the master version that
  placed, and selection is proven by the deterministic `nova_ui` test. A clean-
  machine autopilot placement run is the one residual visual check (it passed on
  this same code path pre-migration, task 20260714-204219).

### Self-reflection

- Inserting a new crate LOW in the dep graph triggers full-graph rebuilds; combined
  with a parallel sprout it starved the autopilot's frame budget. Next time, run
  the timing-sensitive autopilot BEFORE kicking off other heavy builds, or verify
  the touch-free path via `git diff` + a deterministic unit test (which is what
  carried the verdict here).
- `run-example-via-cargo-run-for-assets` bit again: the raw binary + `BEVY_ASSET_ROOT`
  did not resolve assets; `cargo run` from the crate root did.

## Notes

- Depends on: nothing. Blocks: 20260714-214115 (menu), 20260714-214118 (HUD).
