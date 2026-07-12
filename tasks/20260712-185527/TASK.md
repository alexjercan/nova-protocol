# Editor: make the section palette panel scrollable

- STATUS: CLOSED
- PRIORITY: 55
- TAGS: v0.5.0,editor,ux

## Goal

The editor "Menu Container" panel (nova_editor/lib.rs `setup_editor_scene`) has a
fixed height (80%) and its content (title, two Create buttons, the section
palette - now Select/Rebind + one button per section + Delete - and Play) has
grown past it, so lower entries are cut off / spill. Make the panel scroll
vertically with the mouse wheel.

## Steps

- [x] Make the "Menu Container" Node scrollable: add
  `overflow: Overflow::scroll_y()`, a `ScrollPosition::default()`, and a marker
  `EditorScrollPanel`.
- [x] Add a system `scroll_editor_panel` (Editor state) that reads
  `MessageReader<MouseWheel>` and adjusts the panel's `ScrollPosition.y`
  (MouseScrollUnit::Line * a line-height ~20px, Pixel as-is), clamped `>= 0`.
  The editor's WASD camera does not use the wheel (verified: bcs helpers/wasd.rs
  has no wheel/zoom), so there is no scroll/zoom conflict; gameplay's wheel
  bindings are not present in the editor state.
- [x] Verify `cargo check --workspace --all-targets` + `cargo test -p nova_editor`
  + `cargo fmt`. Add a small test that a wheel message moves the panel's
  ScrollPosition (run the system with a MouseWheel message queued, assert y
  changed; and that it clamps at 0 scrolling up from rest). CHANGELOG line.

## Notes

- `Overflow`/`ScrollPosition` are bevy_ui (prelude); `MouseWheel`/
  `MouseScrollUnit` from `bevy::input::mouse`. Bevy 0.19 does NOT auto-scroll on
  wheel - a system must update `ScrollPosition` (bevy ui scroll example pattern).
- Scoped to the whole container (title..Play all scroll together) - simplest and
  matches "panel scrollable". Hover-gating the scroll to the panel area is a
  possible refinement but unnecessary since the editor wheel is otherwise unused.
- Relevant: nova_editor/lib.rs `setup_editor_scene` Menu Container (~440), the
  editor_plugin Update systems block.

## Implementation record

Made the "Menu Container" scrollable: `overflow: Overflow::scroll_y()` +
`ScrollPosition` + an `EditorScrollPanel` marker, and a `scroll_editor_panel`
system (Editor state) that reads `MessageReader<MouseWheel>` and drives the
panel's `ScrollPosition.y` (Line * 20px, Pixel as-is), clamped >= 0 at the top
(bevy clamps the bottom visually). Test `wheel_scrolls_the_editor_panel_and_
clamps_at_the_top` (fresh world per case so the re-run MessageReader does not
re-read a stale message).

Verify: cargo check --workspace --all-targets clean; nova_editor 9/9; cargo fmt
clean.
