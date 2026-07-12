# Review: Editor rebind to mouse buttons

- TASK: 20260712-191604
- BRANCH: fix/editor-rebind-mouse

## Round 1

- VERDICT: APPROVE

Self-reviewed, with the arming-click interaction re-derived: the click that arms
the rebind sets `awaiting_release`; `apply_section_rebind` captures nothing until
no mouse button is held, so the arming LMB is released before the next press is
read - it cannot be bound as the new binding. The None-arm guard
(`rebind.target.is_none()`) stops the binding press from re-arming on whatever is
under the cursor. Capture reads key OR mouse just_pressed; `rebind_binds` replaces
the primary (keyboard|mouse), keeps gamepad, updates both the component and
`player_config.inputs`. The 3-step test proves: click-held -> no bind; released ->
ready; fresh LMB -> bound (old keyboard primary gone, gamepad kept, config
updated). Escape/keyboard paths unchanged (tests init the new MouseButton
resource; they set target directly so awaiting_release=false=ready).

Checks: cargo check --workspace --all-targets clean; nova_editor 10/10 (1 new);
cargo fmt clean.
