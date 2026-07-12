# Look-ray + camera-mode infrastructure: live aim in every view, robust Normal/FreeLook/Turret transitions

- STATUS: OPEN
- PRIORITY: 56
- TAGS: v0.5.0, camera, input, refactor, spike

## Goal

Fix the two infrastructure problems the round-3 adversarial review proved
(spike 20260712-222610, round 3 deltas 1-2), as directed by the user:
"Normal mode chooses target for travel" MUST work, so the code that
forbids it gets fixed. (1) The aim ray must be LIVE in every view - today
only the rig with `SpaceshipRotationInputActiveMarker` integrates input,
so outside Turret view the targeting ray is frozen at the last raise
(camera_controller.rs:616-629, targeting.rs:314-319). (2) The
Normal/FreeLook/Turret mode handling must survive nested holds - today
four last-writer-wins observers corrupt `SpaceshipCameraControlMode` when
Alt and RMB overlap (camera_controller.rs:685-711). Pure infrastructure:
no targeting-behavior change; the slot split (20260712-223035) builds on
it.

## Steps

- [ ] Replace the four mode observers with ONE derivation: each frame,
      mode = Turret if `CombatInput` is held, else FreeLook if
      `FreeLookInput` is held, else Normal (priority Turret > FreeLook;
      memoryless, so any press/release order in any nesting lands on the
      right mode). Derive a public weapon-RAISED flag from the same
      held-state (`CombatInput` held); gameplay consumers will read it
      instead of the camera enum. Add PartialEq to the enum
      (camera_controller.rs:79-85) and only write on real change
      (set_if_neq) so `is_changed()` stays meaningful for the rig-sync
      system (camera_controller.rs:582-584).
- [ ] Fix transition seeding in `sync_spaceship_control_mode`
      (camera_controller.rs:575-631): the rig being ACTIVATED seeds its
      `PointRotation` from the rig being DEACTIVATED (the active look at
      transition time), not unconditionally from the Normal rig
      (today's :586/:623-628 bug - raising out of FreeLook snaps the aim
      to ship-forward instead of the flanker being looked at; same for
      Turret -> FreeLook).
- [ ] Add the "active look ray" accessor: the `PointRotationOutput` of
      the rig currently holding `SpaceshipRotationInputActiveMarker`
      (a small pub helper/SystemParam in camera_controller or a shared
      module - targeting and HUD will consume it in 20260712-223035).
      Document the press-frame property: on the frame a transition
      begins, the marker still sits on the outgoing rig, so the accessor
      IS the live look at raise time.
- [ ] Re-point the EXISTING acquisition system (targeting.rs:313-319) at
      the accessor so the current single-lock behavior gains a live ray
      in Normal/FreeLook - this is the no-behavior-change proof that the
      plumbing works (aim at a rock in Normal view -> the lock follows,
      which today silently cannot happen). Turret slewing
      (player.rs:361-368 ray tier) keeps reading the turret rig.
- [ ] Pause interaction: mode derivation runs from held action state, so
      overlay presses cannot fire hidden transitions; verify with a test
      that a press over the pause overlay does not leak a mode change or
      a raised-flag flip on unpause (feasibility m3).
- [ ] Tests, with the camera rigs modeled as the REAL split entities
      (today's single both-marker test rig masks exactly this class of
      bug - targeting.rs:1029-1032/:1097-1101; retro rule: a clean trace
      on a non-faithful rig is not evidence): full transition matrix
      including nested holds (press Alt during RMB, release RMB while
      Alt held, both release orders) asserting mode, marker placement,
      and raised flag after every step; seeded rotation on every
      transition edge; ray-liveness regression - swivel in Normal view
      and assert the acquisition cone follows.
- [ ] cargo fmt + cargo check + run camera_controller/targeting/input
      test modules.

## Notes

- Spike: docs/spikes/20260712-222610-travel-combat-lock-slots.md (round 3
  deltas 1-2; user directive round 4: fix it properly, do not route
  around it).
- No dependency on 20260712-223034 (parallel-safe); 20260712-223035
  depends on BOTH.
- Nested-hold priority (Turret > FreeLook) is a chosen default -
  questionnaire item if contested.
- `CombatInput`/`FreeLookInput` stay private; the derivation system lives
  in camera_controller and exports only the mode + raised flag + ray
  accessor.
