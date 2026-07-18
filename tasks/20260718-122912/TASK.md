# RCS player input: SHIFT-held mouse/scroll translation with rotation-authority freeze

- STATUS: OPEN
- PRIORITY: 4
- TAGS: v0.7.0,feature,input,spike

## Goal

Wire player input to the RCS primitive (task 20260718-122906) so holding SHIFT
enters a fine-adjust translation mode:

- While SHIFT is held on an RCS-capable ship (gate on `ship_grants_verb(Rcs)`),
  enter RCS mode. Entering disengages any engaged autopilot (consistent with
  "any flight input removes Autopilot").
- Take rotation authority the way the autopilot does: freeze the helm at its
  current heading and repurpose the mouse from aiming to translation. Extend the
  `Without<Autopilot>` gate on `update_controller_target_rotation_torque`
  (input/player.rs:311) so RCS-active also stops feeding the mouse to the helm.
- Held-direction (virtual joystick) mapping into `RcsIntent`: mouse Y ->
  ship-local +/-Z (forward/back), mouse X -> +/-X (strafe), scroll -> +/-Y
  (up/down). Push-and-hold builds toward the cap; release clears `RcsIntent` and
  restores mouse-to-helm. Settle the origin/deadzone/curve during implementation.

## Notes

Spike: tasks/20260718-122508/SPIKE.md (Q1 held-direction, Q4 freeze-heading).
Depends on the RCS core primitive (task 20260718-122906). Input via
bevy_enhanced_input; flight rig at input/player.rs:574; scroll already bound for
lock stepping. Needs a /plan pass to break into steps.
