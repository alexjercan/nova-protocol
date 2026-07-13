# Editor: visible + editable section keybinds (v0.5.0)

- STATUS: CLOSED
- PRIORITY: 50
- TAGS: v0.5.0,feature

## Goal

Make editor keybinds easier to assign and discover. Today the only way to
bind a key to a section is to hold that key while plopping the section down
(`on_click_spaceship_section`, nova_editor/lib.rs:654). That is undiscoverable
and awkward. Build on top of it:

1. Show the current keybind on each placed section (in-editor label) so you can
   see at a glance which key triggers each section.
2. Clicking a placed section lets you change its keybind (rebind after
   placement) instead of delete-and-re-plop while holding a key.

The hold-key-while-placing flow stays as the fast path.

## Design decisions (interaction)

- **Label**: a per-section UI text chip, positioned each frame by projecting the
  section's world position to the viewport with the editor camera
  (`Camera::world_to_viewport`). Editor-local (NOT the gameplay `screen_indicator`,
  whose `ScreenIndicatorCamera` is only on the spaceship chase camera, absent in
  the editor). Shown only for bindable sections (thruster/turret/torpedo; hull and
  controller have no binding).
- **Rebind**: clicking a bindable section while NO placement tool is selected
  (`SectionChoice::None` - the natural "select/edit" mode) arms a rebind on that
  section; the NEXT key pressed becomes its binding; Escape cancels. The label
  shows a "press a key" prompt while armed. Only keyboard capture for now
  (matches the primary-key display); gamepad rebind is out of scope.

## Steps

- [x] Expose a `pub fn binding_label(bindings: &[Binding]) -> String` in
  nova_gameplay (input/player.rs, reusing the private `keyboard_label`; prelude
  -export). Return a short chip for the first bindable input: keyboard via
  `keyboard_label`, mouse buttons as "LMB"/"RMB"/"MMB", else "" . VERIFY the
  `Binding` variants (`Binding::Keyboard { key, .. }`, `Binding::Mouse`/
  MouseButton) against bevy_enhanced_input before matching. Unit test it.
- [x] Editor label component + reconcile: add `SectionKeybindLabel { section:
  Entity }` (a UI `Text` node) and a system `sync_section_keybind_labels`
  (Editor state) that spawns exactly one label per player-ship section carrying a
  binding component (`SpaceshipThrusterInputBinding` / `SpaceshipTurretInputBinding`
  / `SpaceshipTorpedoInputBinding`) and despawns labels whose section is gone -
  the reconcile shape used by ammo_readout's `sync_ammo_readouts`.
- [x] Editor label positioning + text: a system `position_section_keybind_labels`
  (Editor state, PostUpdate after transform propagation) that, per label, reads
  its section `&GlobalTransform`, projects with the editor camera
  (`q_camera: Query<(&Camera, &GlobalTransform), With<...editor cam...>>`,
  `world_to_viewport`), sets the UI node `left/top` (hide when off-screen / behind
  camera via `Node.display` or `Visibility`), and sets the text to
  `binding_label(binds)` - or the rebind prompt when this section is the armed
  target. VERIFY the editor camera marker to query (the `WASDCameraController` /
  `Camera3d` spawned at lib.rs:388).
- [x] Rebind arm on click: add resource `EditorRebind { target: Option<Entity> }`
  (init None; reset to None on OnEnter of each state, like `SectionChoice`). In
  `on_click_spaceship_section`'s `SectionChoice::None` arm (currently `{}`,
  lib.rs:689), if the clicked entity carries any of the three binding components,
  set `EditorRebind.target = Some(entity)`. (Leave Section/Delete arms unchanged -
  the hold-to-bind placement path is untouched.)
- [x] Rebind capture: a system `apply_section_rebind` (Editor state) - if
  `EditorRebind.target` is Some and a key was `just_pressed`: Escape -> clear
  (cancel); otherwise map the KeyCode to a `Binding`, write it to the section's
  binding component (match whichever of the three it has) AND
  `player_config.inputs`, then clear the target. Mirror the bind vec shape the
  placement path builds (keyboard binding; keep any existing gamepad bind if
  present, else just the key).
- [x] Tests (nova_editor + nova_gameplay): (1) `binding_label` maps Keyboard(KeyW)
  -> "W", Mouse(Left) -> "LMB", empty -> "". (2) `sync_section_keybind_labels`
  spawns one label for a bound section and despawns it when the section is
  removed. (3) `apply_section_rebind`: arm a thruster (has
  `SpaceshipThrusterInputBinding`), press a key, run the system, assert the
  component AND `player_config.inputs` now hold that key; a second test that
  Escape cancels (binding unchanged, target cleared). Use App/World rigs like the
  existing editor tests (lib.rs ~1180+); every assertion able to fail.
- [x] Docs: `docs/<date>-editor-section-keybinds.md` (the two features, the
  editor-local projection choice vs screen_indicator, the rebind interaction and
  why None-mode). CHANGELOG Unreleased line.

## Notes

- Keep the hold-to-bind placement path working as-is (do not touch the
  Section/Delete arms of the click handler).
- Bindable = thruster / turret / torpedo (the three `Spaceship*InputBinding`
  components, player.rs:905/993/1067). Hull and controller are not bound.
- The editor stores placement in `PlayerSpaceshipConfig` (lib.rs:170):
  `sections` and `inputs: HashMap<Entity, Vec<Binding>>`; a rebind must update the
  live binding component AND `inputs` (that map is what the scenario reads on
  hand-off).
- `SectionChoice` (lib.rs:515): None / Section(id) / Delete, set by palette
  buttons. None is effectively "select/edit". If discoverability needs it, a
  future "Select" palette button can set None explicitly - not required here.
- Relevant: nova_editor/lib.rs (`on_click_spaceship_section` :654, palette/scene
  `setup_editor_scene` :366, camera :388, `SectionChoice` :515,
  `PlayerSpaceshipConfig` :170, editor tests :1180+); nova_gameplay input/player.rs
  (`keyboard_label` :119, binding components :905/993/1067).
- Open question for /work: confirm `Camera::world_to_viewport` signature/return in
  this bevy version (Result vs Option) and the editor camera's component to query;
  the label positioning step is verify-first on those.

## Implementation record

Added `binding_label` (nova_gameplay, prelude) and, in nova_editor: an
`EditorRebind` resource, `SectionKeybindLabel` UI chips reconciled by
`sync_section_keybind_labels` and positioned/text-set by
`position_section_keybind_labels` (editor-camera `world_to_viewport`), rebind
armed in the `SectionChoice::None` click arm and consumed by
`apply_section_rebind` (Escape cancels; updates the live binding component AND
`PlayerSpaceshipConfig::inputs`; preserves a non-keyboard bind). Hold-to-bind
placement path untouched. Write-up + reflection:
docs/retros/20260712-editor-section-keybinds.md.

Decisions: editor-local projection over the gameplay `screen_indicator` (whose
`ScreenIndicatorCamera` is only on the chase camera, absent in the editor);
rebind entered from the None/select mode; keyboard-only rebind.

Verify: cargo check --workspace --all-targets clean; nova_editor 8/8 (3 new),
nova_gameplay binding_label green; cargo fmt clean. Not unit-tested: the actual
pixel projection in position_section_keybind_labels (needs a rendered viewport).
