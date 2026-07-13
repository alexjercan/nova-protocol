# Editor: visible + editable section keybinds

Task: tasks/20260712-163912.

## What shipped

Two additions to the ship editor (nova_editor/lib.rs), on top of the existing
hold-key-while-placing bind path (which is untouched):

1. **Keybind labels.** Every bindable section (thruster / turret / torpedo)
   shows a small gold UI chip with its current key. A `SectionKeybindLabel`
   entity is reconciled one-per-bound-section by `sync_section_keybind_labels`,
   and `position_section_keybind_labels` projects each section's world position
   to the viewport with the editor camera (`Camera::world_to_viewport`) each
   frame, setting the chip's screen position and text.
2. **Click-to-rebind.** Clicking a bindable section while no placement tool is
   selected (`SectionChoice::None` - the select/edit mode) arms a rebind
   (`EditorRebind.target`); the label shows "press key"; the next key press
   (`apply_section_rebind`) becomes the section's binding, Escape cancels. The
   new keyboard binding replaces the old keyboard binding on both the live
   `Spaceship*InputBinding` component AND `PlayerSpaceshipConfig::inputs` (what
   the scenario reads on hand-off), preserving any non-keyboard (gamepad) bind.

A `binding_label(&[Binding]) -> String` helper (nova_gameplay, reusing the
private `keyboard_label`) formats a binding for the chip: keyboard keys short
(`KeyW` -> "W"), mouse as "LMB"/"RMB"/"MMB".

## Decisions and alternatives

- **Editor-local viewport projection vs the gameplay `screen_indicator`.** The
  HUD's `screen_indicator` looked like the obvious reuse, but its
  `ScreenIndicatorCamera` is attached only to the spaceship *chase* camera
  (hud/mod.rs `add_screen_indicator_camera`), which the editor does not use (it
  runs a WASD camera). Reusing it would have meant tagging the editor camera and
  pulling the HUD layer machinery into the editor. A ~15-line editor-local
  `world_to_viewport` projection is self-contained and has no cross-crate
  coupling. (Reuse is good - but only when the substrate actually fits; here it
  didn't.)
- **Rebind entered from `SectionChoice::None`** (select/edit mode) rather than a
  new palette tool. That arm did nothing before; clicking a bound section there
  is a natural "select this to edit it". A dedicated "Select" palette button
  could be added later for discoverability, but is not needed for the feature.
- **Keyboard-only rebind.** The chip shows the primary (keyboard/mouse) bind, and
  rebinding captures a keyboard key. Gamepad rebinding is out of scope; the
  placement path's gamepad bind (if any) is preserved through a rebind.

## Verification

- `binding_label` unit test (KeyW -> "W", Left -> "LMB", gamepad-only -> "").
- `keybind_labels_reconcile_to_one_per_bound_section`: a label is spawned for a
  bound section (and NOT for an unbound `SectionMarker`), is idempotent, and is
  despawned when the section is removed.
- `rebind_replaces_the_keyboard_bind_on_component_and_config`: arming a thruster
  and pressing a key updates both the component and `player_config.inputs`,
  replaces the old keyboard bind, preserves the gamepad bind, and consumes the
  arm.
- `rebind_escape_cancels_without_changing_the_bind`.

`cargo check --workspace --all-targets` clean; nova_editor 8/8, nova_gameplay
binding_label green; `cargo fmt` clean.

Not unit-tested: `position_section_keybind_labels`' actual pixel projection -
`Camera::world_to_viewport` needs a rendered viewport, impractical headless. It
is a thin wrapper over the bevy API; the label's text/reconcile logic (the part
with real branching) is covered.

## Self-reflection

- Checking `screen_indicator`'s camera assignment BEFORE planning the label
  saved a wrong turn - the reuse would have compiled but silently shown nothing
  in the editor (no `ScreenIndicatorCamera`). Verifying the substrate's
  preconditions, not just its API, is the lesson.
- Follow-ups if playtest wants them: a "Select" palette button for
  discoverability; gamepad rebind; a highlight on the armed section.
