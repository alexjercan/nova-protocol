# Look-ray + camera-mode infrastructure: live aim in every view, robust mode transitions

- STATUS: OPEN
- PRIORITY: 58
- TAGS: v0.5.0, input, camera, targeting, spike

## Goal

The deliberate-radar design (spike 20260713-082207) needs the LIVE look ray in
every view and mode handling that survives nested holds. Today the aim ray only
integrates input on the rig holding `SpaceshipRotationInputActiveMarker`
(camera_controller.rs:653-674), so outside Turret view it is frozen at the last
raise (targeting.rs:313-320); the four mode observers are last-writer-wins, so
Alt-release while RMB is held sets Normal and would leave manual gunnery
tracking a frozen ray (camera_controller.rs:692-711, verified 2026-07-13).
Pure infrastructure: no targeting-behavior change; the radar task
(20260713-082330) and manual gunnery (20260713-082337) build on it.

## Steps

- [ ] Replace the four mode observers (camera_controller.rs:685-711) with ONE
      derivation: each frame, mode = Turret if `CombatInput` is held, else
      FreeLook if `FreeLookInput` is held, else Normal (priority
      Turret > FreeLook; memoryless, so any press/release order in any nesting
      lands right). Add `PartialEq` to `SpaceshipCameraControlMode`
      (camera_controller.rs:79-85) and write via `set_if_neq` so
      `is_changed()` stays meaningful for the rig-sync system (:582-584).
- [ ] Derive a public weapons-RAISED flag from the same held state
      (`CombatInput` held) as a ship-root component (default off, cleared on
      respawn for free); gameplay consumers (radar slot latch, safety, manual
      aim) will read RAISED, never the camera enum. Pause rule: the derivation
      reads held action state each frame, so an overlay press cannot fire a
      hidden transition; verify with a test that pressing/releasing RMB over
      the pause overlay leaves mode + raised consistent on unpause (the
      round-3 m3 carry-over).
- [ ] Fix transition seeding in `sync_spaceship_control_mode`
      (camera_controller.rs:571-636): the rig being ACTIVATED seeds its
      `PointRotation` from the rig being DEACTIVATED (the live look at
      transition time), not unconditionally from the Normal rig (today's
      :586/:608-628 bug - raising out of FreeLook snaps aim to ship-forward).
- [ ] Add the "active look ray" accessor: the `PointRotationOutput` of the rig
      currently holding `SpaceshipRotationInputActiveMarker` (a small pub
      helper/SystemParam in camera_controller). Document the press-frame
      property: on the frame a transition begins the marker still sits on the
      outgoing rig, so the accessor IS the live look at raise/press time.
- [ ] Re-point the EXISTING acquisition system (targeting.rs:313-320) at the
      accessor - the no-behavior-change proof that the plumbing works: aim at
      a body in Normal view and the (still-automatic, pre-radar) lock follows,
      which today silently cannot happen. Turret slewing (player.rs:361-368
      ray tier) keeps reading the turret rig.
- [ ] Tests with the camera rigs modeled as the REAL split entities (today's
      single both-marker test rig masks exactly this bug class -
      targeting.rs:1029-1032/:1097-1101; production-faithful-rigs lesson):
      full transition matrix including nested holds (press Alt during RMB,
      release RMB while Alt held, both release orders) asserting mode, marker
      placement and raised flag after every step; seeded rotation on every
      transition edge; ray-liveness regression (swivel in Normal view, assert
      the acquisition cone follows).
- [ ] cargo fmt + cargo check; run the camera_controller/targeting/input test
      modules; 12_hud_range + 10_gameplay autopilots (no behavior change
      expected - they must stay green as-is).

## Notes

- Spike: docs/spikes/20260713-082207-deliberate-radar-locking.md (+ its
  adversarial round: the infra is a hard prerequisite for MANUAL aim, not just
  radar-in-Normal).
- Reincarnates closed task 20260712-231141 (wontdo with its family; its body
  was adversarially reviewed and is lifted here nearly verbatim).
- Nested-hold priority (Turret > FreeLook) is a chosen default.
- `CombatInput`/`FreeLookInput` stay private; the module exports only the
  mode + raised flag + ray accessor.
- File:line anchors re-verified 2026-07-13 (adversarial feasibility pass).
