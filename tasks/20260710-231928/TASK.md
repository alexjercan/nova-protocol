# HUD text anchored to moving objects twitches (e.g. velocity on the ship)

- STATUS: OPEN
- PRIORITY: 82
- TAGS: v0.5.0, hud, bug

## Goal

Playtest bug (user, 2026-07-10): HUD text anchored to moving objects
twitches - most visibly the velocity/speed text at the ship. Root cause
(docs/spikes/20260711-103527-twitching-family-two-clocks.md): screen
indicators project in Update with the camera pose from the PREVIOUS frame
(the bcs chase camera moves later, in PostUpdate), so HUD nodes lag the
rendered world by one frame of camera motion; additionally Point anchors
are computed from raw avian `Position` while Entity anchors resolve eased
`GlobalTransform` - two pose families in one overlay.

## Steps

- [ ] Check whether bevy-common-systems exposes a public system set (or
      ordering handle) for the chase-camera PostUpdate systems
      (bcs src/camera/chase.rs:149). If not, add one in bcs via its own
      task + full cycle in that repo (cross-repo precedent:
      docs/retros/20260709-125640-residual-roll-release.md), and bump the
      nova dependency.
- [ ] Move `update_screen_indicators`
      (crates/nova_gameplay/src/hud/screen_indicator.rs:176-179) from
      Update to PostUpdate: after the bcs chase-camera set and
      `TransformSystems::Propagate` (fresh camera + anchor poses), before
      `UiSystems::Layout` (placements land this frame). Verify with a
      schedule-order assertion or test that the camera pose it reads is
      this frame's final pose.
- [ ] Sweep the anchor driver systems that feed ScreenIndicator (speed chip
      flight_status.rs:202, maneuver_instruments.rs drivers, turret pips)
      for schedule fit: Entity anchors resolve at projection time and can
      stay where they are; Point anchors must be recomputed on the render
      clock at projection time (radius spoke reads raw avian `Position`,
      maneuver_instruments.rs:344-349 - switch to the eased pose family or
      drive it in PostUpdate before projection).
- [ ] Regression test: headless app, moving ship + chase camera; step
      frames and assert an entity-anchored indicator's placement equals a
      fresh `world_to_viewport` recomputed with the SAME frame's final
      camera pose (fails on the one-frame-stale camera bug).
- [ ] cargo check + fmt + new tests; extend the spike doc fix record.

## Notes

- Evidence: screen_indicator.rs:176-179 (Update projection), :418
  (world_to_viewport), plugin.rs Update set chain (NovaHudSystems before
  NovaCameraSystems), bcs chase.rs:149 (camera moves in PostUpdate).
- 20260710-231929 (crosshair) depends on this task's PostUpdate move; its
  pip-staleness fix chains onto the same ordering.
- UI layout/text drivers that only set STRINGS (speed value etc.) are not
  part of this bug; only world-to-screen placement is.
