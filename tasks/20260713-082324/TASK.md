# Look-ray + camera-mode infrastructure: live aim in every view, robust mode transitions

- STATUS: CLOSED
- PRIORITY: 58
- TAGS: v0.5.0, input, camera, targeting, spike

## Outcome (CLOSED 2026-07-13)

Shipped as planned (all steps; the 231141 body lifted and adjusted):

- `derive_control_mode_and_raised` replaces the four observers: mode from held
  TriggerStates (Turret > FreeLook > Normal, memoryless, set_if_neq +
  PartialEq on the enum), and a ship-root `WeaponsRaised(bool)` mirroring the
  combat hold (self-healing insert; respawn starts lowered for free).
  Deliberately pause-ungated like the camera chain: memoryless derivation
  means a press+release inside a pause leaves no trace (test-pinned), and
  every gameplay consumer of the flag is pause-gated itself.
- `sync_spaceship_control_mode` seeds the incoming rig from the OUTGOING
  (active) rig's output; the Normal rig is deliberately never re-seeded (it
  steers the SHIP - documented in-code).
- `ActiveLookRay` SystemParam (pub, in the prelude) with the press-frame
  property documented; `update_spaceship_target_input` and
  `update_component_lock` now read it (menu/rig-less worlds early-return,
  matching the old Single-skip).
- Faithful split-rig fixtures: targeting test rigs are now an ACTIVE normal
  rig + a dormant turret DECOY 90 degrees off, so reading the wrong rig fails
  loudly; camera tests drive REAL device input through InputPlugin +
  EnhancedInput + the production action shape (finish/cleanup before rig
  spawn, per the wheel-e2e pattern).

Verified: 13 camera tests (3 new: nested-hold matrix incl. the Alt-release-
while-RMB-held regression, outgoing-rig seeding, pause-no-trace with a held-
through-unpause delivery guard), 46 targeting tests (new: ray-liveness -
swivel the active rig, the cone follows; None before the swivel is the
delivery guard), 147 input tests, workspace check, both autopilots green
(no-behavior-change proof). A/B per the fail-first rule: sabotaging the seed
source back to the normal rig fails `transitions_seed_from_the_outgoing_rig`
("must aim at the flanker"); restored, it passes.

Note for 082330/082337: consumers read `WeaponsRaised` / `ActiveLookRay` from
the crate prelude; player.rs turret slewing still deliberately reads the
TURRET rig (its own feed), untouched here.

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

- [x] Replace the four mode observers (camera_controller.rs:685-711) with ONE
      derivation: each frame, mode = Turret if `CombatInput` is held, else
      FreeLook if `FreeLookInput` is held, else Normal (priority
      Turret > FreeLook; memoryless, so any press/release order in any nesting
      lands right). Add `PartialEq` to `SpaceshipCameraControlMode`
      (camera_controller.rs:79-85) and write via `set_if_neq` so
      `is_changed()` stays meaningful for the rig-sync system (:582-584).
- [x] Derive a public weapons-RAISED flag from the same held state
      (`CombatInput` held) as a ship-root component (default off, cleared on
      respawn for free); gameplay consumers (radar slot latch, safety, manual
      aim) will read RAISED, never the camera enum. Pause rule: the derivation
      reads held action state each frame, so an overlay press cannot fire a
      hidden transition; verify with a test that pressing/releasing RMB over
      the pause overlay leaves mode + raised consistent on unpause (the
      round-3 m3 carry-over).
- [x] Fix transition seeding in `sync_spaceship_control_mode`
      (camera_controller.rs:571-636): the rig being ACTIVATED seeds its
      `PointRotation` from the rig being DEACTIVATED (the live look at
      transition time), not unconditionally from the Normal rig (today's
      :586/:608-628 bug - raising out of FreeLook snaps aim to ship-forward).
- [x] Add the "active look ray" accessor: the `PointRotationOutput` of the rig
      currently holding `SpaceshipRotationInputActiveMarker` (a small pub
      helper/SystemParam in camera_controller). Document the press-frame
      property: on the frame a transition begins the marker still sits on the
      outgoing rig, so the accessor IS the live look at raise/press time.
- [x] Re-point the EXISTING acquisition system (targeting.rs:313-320) at the
      accessor - the no-behavior-change proof that the plumbing works: aim at
      a body in Normal view and the (still-automatic, pre-radar) lock follows,
      which today silently cannot happen. Turret slewing (player.rs:361-368
      ray tier) keeps reading the turret rig.
- [x] Tests with the camera rigs modeled as the REAL split entities (today's
      single both-marker test rig masks exactly this bug class -
      targeting.rs:1029-1032/:1097-1101; production-faithful-rigs lesson):
      full transition matrix including nested holds (press Alt during RMB,
      release RMB while Alt held, both release orders) asserting mode, marker
      placement and raised flag after every step; seeded rotation on every
      transition edge; ray-liveness regression (swivel in Normal view, assert
      the acquisition cone follows).
- [x] cargo fmt + cargo check; run the camera_controller/targeting/input test
      modules; 12_hud_range + 10_gameplay autopilots (no behavior change
      expected - they must stay green as-is).

## Notes

- Spike: tasks/20260713-082207/SPIKE.md (+ its
  adversarial round: the infra is a hard prerequisite for MANUAL aim, not just
  radar-in-Normal).
- Reincarnates closed task 20260712-231141 (wontdo with its family; its body
  was adversarially reviewed and is lifted here nearly verbatim).
- Nested-hold priority (Turret > FreeLook) is a chosen default.
- `CombatInput`/`FreeLookInput` stay private; the module exports only the
  mode + raised flag + ray accessor.
- File:line anchors re-verified 2026-07-13 (adversarial feasibility pass).
