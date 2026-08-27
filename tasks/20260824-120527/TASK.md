# Tabbed settings menu with keyboard and gamepad rebinding

- STATUS: CLOSED
- PRIORITY: 65
- TAGS: v0.12.0, ui, input

v0.12.0. The settings half split out of `20260714-001140` (which keeps
gamepad menu navigation + the mobile pad, backlog). Depends on the bindings
registry from `20260820-174148` phase 1. Research:
`tasks/20260815-231945/INPUT-AND-PROCESS.md` sections 2-3 and 5.

## Goal

Turn Settings into a real settings menu: a tabbed layout (Audio / Graphics /
Controls / Interface), rebinding for keyboard AND gamepad reading and
writing the bindings registry instead of the hand-authored display mirror,
and a window-mode option - the game does not ship fullscreen today.

## Window mode (owner add, 2026-08-24)

The window is created once at a fixed 1024x768 windowed
(nova_core/src/lib.rs:486-500 `window_plugin`); `WindowMode` is never set
and nothing in the tree touches fullscreen. Add to the Graphics tab:

- Windowed / Borderless fullscreen as a segmented row (same
  `ButtonValue<T>` pattern as the graphics preset). Borderless =
  `WindowMode::BorderlessFullscreen` on the current monitor; skip exclusive
  fullscreen unless it turns out free.
- Apply live by mutating the primary `Window` on change, and at startup from
  persistence - same `PersistedSettings` field flow as the rest of this
  task.
- Native only. The web build already fits the canvas
  (`fit_canvas_to_parent`, lib.rs:496); browser fullscreen needs a user
  gesture and is out of scope here - hide or fix the row on wasm.

## Reuse - all three flows are tested code already in tree

- Tabs: the mods screen pattern verbatim - `ModsActiveTab` resource
  (nova_menu/src/mods.rs:51-52), `on_mods_tab` (:226-249), visuals
  `segmented_container` / `segmented_option`
  (nova_ui/src/widget/segmented.rs:34-50).
- Rebind capture: `apply_section_rebind` (nova_editor/src/keybind.rs:197-279)
  - armed-target resource, capture next key or mouse press, Escape cancels,
  waits out the arming click, refuses conflicts, stays armed on refusal.
  Generalise from section entities to action names. Second copy at
  nova_os_ui/src/ship/rebind.rs. NEITHER captures gamepad buttons - the
  gamepad capture branch is the genuinely new piece.
- Persistence: `PersistedSettings` (nova_menu/src/settings_store.rs:16-36),
  RON key "settings", debounced save, exit flush. Add a serde-defaulted
  `bindings: map<action_name, bindings>` field; copy the partial-file test
  pattern (settings_store.rs:213-230). Apply loaded bindings by patching the
  rig's `Binding` child entities on rig spawn (read pattern:
  hints.rs:227-242).

## What this deletes

- The FLIGHT and TARGETING rows of the hand-authored mirror
  (nova_ship/src/input/reference.rs, TODO(20260710-231927) at :10) and their
  parity test (hints.rs:352-449): the Controls tab renders from the registry.
- `flight_rig_reserved_sources()` (hints.rs:164-195), the SECOND
  hand-maintained mirror: conflict checks must compute from the live
  registry, or they go stale on the first remap. `nova_hud/src/key_glyphs.rs`
  needs the same live source.

## Fixed rows

The raw system chords stay non-rebindable this release and are LISTED as
fixed: pause Esc/Start, HUD backquote/Select, NOVA OS Tab/RightThumb, comms
V/B, scenario advance Enter/DPadDown. (Inventory:
INPUT-AND-PROCESS.md section 5.)

## Done when

- Settings shows tabs; Controls lists every registry action with its
  keyboard and gamepad bindings, live.
- Rebind a flight key and a pad button; conflicts refused with a reason;
  bindings survive a restart; the rig is built from them.
- The mirror rows and parity test are deleted; reserved-source conflict
  checks are registry-derived.
- Works from both entry points (main menu overlay and pause overlay).
- Borderless fullscreen toggles live from the Graphics tab, survives a
  restart, and the wasm build shows no broken row.

## Proof (2026-08-27, commit b6075c4b)

Landed on `master` in `b6075c4b`, on top of `8e956a64` (one input
vocabulary), `f35c3dc3` (section rigs through `source_bindings`) and
`662957f5` (shadow actions, capture, refusal).

### Done-when, item by item

- **Tabs, live rows.** `SettingsTabKind` / `SettingsActiveTab` /
  `SettingsTabBody`; `refresh_settings_tab` runs on
  `settings_tab_dirty` (active tab, table, pending rebind, or a
  just-spawned body). Controls draws `InputBindings::rows()`, so a
  shadow action never gets a row of its own.
  Tests: `settings_panel_builds_one_tab_at_a_time`,
  `pressing_a_tab_swaps_the_body`, `the_controls_readout_follows_a_rebind`.
- **Key, pad, refusal, restart, rig.** `apply_settings_rebind` captures
  through `InputSources::captured_desk` / `captured_pad` - the pad half
  reads the `Gamepad` COMPONENT, which is the only place bevy 0.19 keeps
  digital state. Tests:
  `a_rebind_takes_the_next_key_and_the_table_follows`,
  `a_rebind_takes_a_pad_button`,
  `a_taken_key_is_refused_by_name_and_the_chip_stays_armed`,
  `escape_backs_out_of_an_armed_rebind`,
  `reset_defaults_puts_every_row_back`,
  `rebinding_a_gesture_moves_the_half_that_follows_it`,
  `an_armed_chip_does_not_eat_the_pointers_own_button`.
  Restart: `settings_store::tests::save_then_load_round_trips` and
  `a_moved_keybind_persists_and_an_older_store_still_loads`.
  Rig: `rebuild_flight_input_on_rebind` and
  `rebuild_player_input_on_rebind`, proved by
  `a_rebind_made_in_flight_reaches_the_live_rig`.
- **Mirrors gone, conflicts registry-derived.** `input/reference.rs` and
  its parity test are gone. `InputBindings::holder_in(context, source)`
  is the one live check; `nova_editor::keybind::binding_conflict` and
  `nova_os_ui::ship::rebind::reserved_conflict` both read it. Tests:
  `the_shared_key_warning_follows_a_rebind`,
  `reserved_flight_control_remains_blocked`.
  `flight_rig_reserved_sources()` SURVIVES on purpose, for
  `nova_authoring`'s content lint: a mod's ship is linted against what
  the game SHIPS with, not against one player's remap.
  `nova_hud/src/key_glyphs.rs` keeps a static path table (a preload
  collection cannot be dynamic) but its coverage test now derives the
  chrome keys from `hud_bindings()`, and the OBJECTIVES affordance reads
  `novaos_toggle` off the live table.
- **Both entry points.** `the_pause_overlay_settings_body_tabs_too`.
- **Window mode.** `WindowModeSetting` + `apply_window_mode`, native
  only; `the_window_row_drives_the_primary_window`. Persisted as a
  serde-defaulted field. `cargo check -p nova_menu --target
  wasm32-unknown-unknown` is clean - the row is cfg'd out and its
  helpers carry a wasm-only `expect(dead_code)`.

### Decisions

- **The pointer's own button is never captured.** Found live: an armed
  chip ate the next click of a driven walk and `main_drive` came out on
  Left Mouse - which then made the row that would undo it unclickable.
  Every other mouse button stays bindable.
- **Reset Defaults**, not in the Done-when, was added because a
  whole-column rebind with no way back is a trap.
- **A device column is a chip only if the action already holds a source
  there.** An action is MOVED, never given a button it never had, which
  leaves `rcs_aim` and `camera_rotate` as read-only text.
- **The label takes the row's leftover width** so both chip columns land
  at the same x. Sized by the label instead, the column read as ragged -
  seen in the first live capture.

### Verified live (Xvfb :92)

- `system_menu_boot` with `NOVA_AUTOPILOT=1`: clicked New Game, reached
  Playing, menu tore down, `cycle complete, no panic`.
- `screenshot_menu` walked Settings -> Controls -> armed chip ->
  Graphics. Read off the captures: the tab bar, group headers, aligned
  keyboard and pad columns, `PRESS A KEY` on the armed chip, and the
  Quality and Window rows.

### Skipped

- The workspace test suite and Clippy (standing instruction; the suite
  OOMs this box). Ran `-p` tests for `nova_menu` (94), `nova_input` (31),
  `nova_hud` (218), `nova_ship`, `nova_editor` and `nova_os_ui` filtered,
  plus `cargo fmt --all` and `cargo check --workspace --all-targets`.

### Found and fixed on the way

The menu test fixture was loading the developer's real
`~/.config/nova-protocol/settings.ron`. A keybind saved by playing the
game - or by one of this task's own screenshot runs - silently rewrote
the table the tests assert on. `support::app()` now points
`NOVA_CONFIG_ROOT` at a scratch dir.
