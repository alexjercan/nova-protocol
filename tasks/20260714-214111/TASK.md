# nova_ui crate: shared theme + widgets; migrate nova_editor onto it

- STATUS: OPEN
- PRIORITY: 65
- TAGS: ui,v0.6.0

Umbrella: task 20260714-212139 (unify the whole game UI to the web-app theme).

## Goal

Create a new bevy-only `nova_ui` crate that holds ONE source of truth for the
game UI theme (palette + metrics) and the reusable widgets, then migrate
`nova_editor` to consume it (deleting the editor's private theme/widget copies).
This is the foundation the menu (20260714-214115) and HUD (20260714-214118)
restyles build on. Decisions (user, 20260714): a new `nova_ui` crate (not folded
into an existing crate); palette/metrics only, keep the current default font
(real fonts are a separate follow-up).

Done = `nova_ui` exists and exports the theme + widget helpers; `nova_editor`
uses `nova_ui` with its own `ui/theme.rs` + button infra deleted; the editor
looks and behaves identically (12 nova_editor tests + the `09_editor` autopilot
still green).

## Steps

- [ ] Create `crates/nova_ui/` (`Cargo.toml`: `name = "nova_ui"`, workspace
  version/edition/lints, `bevy = { version = "0.19.0" }`, NO nova deps) and add
  `"crates/nova_ui"` to the workspace `members` in the root `Cargo.toml`.
- [ ] `crates/nova_ui/src/theme.rs`: move the palette + metric consts verbatim
  from `crates/nova_editor/src/ui/theme.rs` (BG, PANEL, PANEL_RAISED, BORDER,
  BORDER_BRIGHT, CYAN, CYAN_BRIGHT, AMBER, TEXT, TEXT_MUTED, SELECTED_FILL,
  RADIUS, BORDER_W, ICON; drop the editor-only RAIL_W/DRAWER_W, keep those in the
  editor). Make them `pub`.
- [ ] `crates/nova_ui/src/widget.rs`: move the generic button infra from
  `crates/nova_editor/src/ui/widget.rs` and generalize the editor-specific names:
  `EditorButton` -> `ThemedButton`, `SelectedOption` -> `Selected`; keep
  `ButtonValue<T>`, `button_on_setting<T>`, the `button_on_interaction` colour
  observers, `on_add/remove_selected`, and the `button(text)` factory (rename to
  `themed_button`). Add `pub fn register(app: &mut App)` that wires the colour +
  selection observers. All `pub`.
- [ ] `crates/nova_ui/src/lib.rs`: `pub mod theme; pub mod widget;` + a `prelude`
  re-exporting the theme consts, `ThemedButton`, `Selected`, `ButtonValue`,
  `themed_button`, `button_on_setting`, and `register`. Also move the generic
  layout helpers `panel_header`/`separator` here (from the editor's
  `ui/drawer.rs` + `ui/mod.rs`) as `pub` widget helpers, since menu + HUD will
  reuse them.
- [ ] Add `nova_ui = { path = "../nova_ui" }` to `crates/nova_editor/Cargo.toml`.
- [ ] Migrate `nova_editor` onto `nova_ui`: delete `crates/nova_editor/src/ui/theme.rs`
  and the moved button infra from `ui/widget.rs`; re-point every `crate::ui::theme::*`
  to `nova_ui::theme::*` (or a local `use nova_ui::prelude::*`), `EditorButton` ->
  `ThemedButton`, `SelectedOption` -> `Selected`, `button` -> `themed_button`, and
  `crate::ui::widget::{button_on_setting,register}` -> `nova_ui`'s. Keep the
  editor-only `RAIL_W`/`DRAWER_W`, the component card/icon, rail categories,
  drawer, and section tooltip in `nova_editor` (they consume `nova_ui`).
- [ ] `cargo check --workspace --all-targets --features debug` clean; `cargo fmt`;
  `cargo test -p nova_editor` (12 pass) and `cargo test -p nova_ui` (move the
  scroll/selection-relevant unit tests that belong with the migrated widgets, if
  any); run the `09_editor` autopilot headless and confirm create-ship -> select
  card -> place section still works.

## Notes

- Relevant files: `crates/nova_editor/src/ui/{theme.rs,widget.rs,drawer.rs,mod.rs}`
  (the source of the moved code); root `Cargo.toml` (`members`); a minimal crate
  template is `crates/nova_info/Cargo.toml`.
- Dep graph (verified): `nova_gameplay` depends only on `nova_events`; `nova_menu`
  and `nova_editor` depend on `nova_gameplay`; `nova_assets` depends on
  `nova_gameplay`. A bevy-only `nova_ui` with NO nova deps can be depended on by
  menu/editor/gameplay without a cycle.
- `nova_info` is NOT a consumer - it is build-info only (`APP_VERSION`), no UI.
- The editor's `button_on_interaction` already references theme consts by name, so
  moving the consts + renaming the marker is mechanical. `button_on_setting<T>` is
  already generic (T = `SectionChoice` in the editor; the menu will use its own T).
- Keep the editor's behaviour identical - this task is a refactor + extraction, not
  a restyle of the editor (it is already on this palette). The proof is the editor
  tests + autopilot staying green.
- Depends on: nothing. Blocks: 20260714-214115 (menu), 20260714-214118 (HUD).
