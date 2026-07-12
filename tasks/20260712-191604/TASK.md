# Editor rebind: allow binding mouse buttons (LMB etc.)

- STATUS: CLOSED
- PRIORITY: 55
- TAGS: v0.5.0,editor,ux,playtest

## Goal

The editor click-to-rebind (task 20260712-163912) only captures keyboard keys, so
you cannot set a mouse button (e.g. LMB for a turret's fire) as a section's
binding. Make the rebind capture mouse buttons too.

The wrinkle: LMB is also the click that ARMS the rebind, so capturing the arming
click as the new binding must be avoided.

## Steps

- [x] `apply_section_rebind` (nova_editor/lib.rs): also read
  `Res<ButtonInput<MouseButton>>` and, once ready, capture the first
  `just_pressed` keyboard key OR mouse button -> `Binding::from(...)`. Escape
  still cancels.
- [x] Don't bind the arming click: add an `awaiting_release: bool` to
  `EditorRebind` (default false). The click-arm sets it true; `apply_section_rebind`
  clears it only once no mouse button is held, and captures nothing until then -
  so the LMB that armed the rebind is released before the next press is captured.
- [x] Don't re-arm while already armed: in `on_click_spaceship_section`'s
  `SectionChoice::None` arm, only set the target when `rebind.target.is_none()`,
  so pressing LMB again to bind it does not re-arm on whatever is under the cursor.
- [x] `rebind_binds` replaces the PRIMARY input (filter out both
  `Binding::Keyboard` AND `Binding::MouseButton`, keep gamepad) and prepends the
  new binding - so rebinding swaps key<->mouse cleanly and keeps any gamepad bind.
- [x] Tests: (a) existing keyboard rebind + Escape tests keep passing (init
  `ButtonInput<MouseButton>`; they set target directly so awaiting_release stays
  false = ready). (b) new: arm with awaiting_release=true, run with a mouse button
  still held -> no capture; release (no button) -> becomes ready; then press LMB
  -> binds `MouseButton::Left` on the component AND player_config.inputs. Every
  assertion able to fail. CHANGELOG line.

## Notes

- `binding_label` already renders MouseButton as LMB/RMB/MMB, so the chip shows
  the new mouse bind with no change.
- RMB toggles the editor cursor grab (`lock_on_left_click`), a separate control;
  binding RMB is allowed (that side effect is pre-existing). LMB is not otherwise
  consumed, so binding LMB is clean.
- Relevant: nova_editor/lib.rs `apply_section_rebind`, `EditorRebind`,
  `on_click_spaceship_section` None arm, `rebind_binds` closure.

## Implementation record

`apply_section_rebind` now also reads `Res<ButtonInput<MouseButton>>` and captures
the first just_pressed key OR mouse button. `EditorRebind` gained
`awaiting_release`: the click-arm sets it (and the None arm only arms when nothing
is armed yet), and capture is suppressed until no mouse button is held - so the
arming LMB is released before the next press is read, and it is not itself bound.
`rebind_binds` now filters out BOTH Keyboard and MouseButton (the primary inputs)
and prepends the new binding, keeping gamepad binds. `binding_label` already
renders mouse buttons, so the chip shows LMB/RMB/MMB with no change.

Verify: cargo check --workspace --all-targets clean; nova_editor 10/10 (new:
`rebind_binds_a_mouse_button_after_the_arming_click_releases`, exercising the
awaiting-release gate + LMB capture + gamepad preservation + config update);
cargo fmt clean.
