# Inset kill cam: freeze-frame linger on target death

- STATUS: OPEN
- PRIORITY: 46
- TAGS: v0.5.0,hud,ux,spike

## Goal

Playtest (2026-07-13): the viewfinder closes on the exact frame the kill
lands. Option B from the spike: when the framed target becomes
UNRESOLVABLE (died - the discriminator vs tap-clear/decay/retarget, whose
targets remain alive), the panel and RTT camera stay up with the camera
FROZEN at its last pose for KILL_CAM_SECS (~2 s), filming the slicer
fragments, then close. A new combat lock preempts the linger instantly;
Chrome-tier hide stays immediate. Presentation-only: no targeting-layer
changes - locks, safety and turrets behave exactly as today.

## Steps

- [ ] State (target_inset.rs): `KILL_CAM_SECS` const (~2.0);
      `TargetInsetLastFramed { target: Entity, pose: Transform }` written
      onto the PANEL entity every camera-framed frame (panel-lifecycle
      state, not a Local - a HUD respawn must not inherit a stale frame);
      `TargetInsetKillCam { pose: Transform, remaining: f32 }`.
- [ ] `drive_inset_camera` rework: on a teardown-eligible frame (no
      framable lock), check the panel's `TargetInsetLastFramed` - if its
      target is NO LONGER ALIVE (`Query<Entity>` miss = despawned, the
      death discriminator; a tap-cleared/decayed/flipped target is still
      alive) enter the kill cam: keep panel visible, keep/spawn the RTT
      camera at the FROZEN pose, suppress NO-SIGNAL, tick `remaining`
      down on `Time`. Expiry -> normal teardown (remove both state
      components). A live framable lock at any point removes the kill cam
      and re-frames normally (preemption). Chrome-tier hidden -> full
      teardown including both state components. ORDERING NOTE (verified):
      targeting's validity clear runs in SpaceshipInputSystems, the HUD
      in NovaHudSystems after it - the HUD sees the cleared lock the same
      frame the target died, and the asteroid root stays lockable until
      its actual despawn (husk path), so clear + despawn land together.
- [ ] The faction line and border need NO changes (they poll the live
      lock/safety: caption clears, border relaxes as the safety
      disengages - the composed stand-down beat from the spike).
- [ ] Tests (target_inset.rs, the existing rig): death lingers (despawn
      the framed target + clear the lock -> panel visible, camera alive,
      pose frozen at the pre-death pose); expiry closes (force
      `remaining` negative, run, everything torn down - the ghost-test
      shape, no Time dependency); tap-clear with a LIVE target does not
      linger (the discriminator pin, delivery-guarded by the death case);
      a NEW lock preempts mid-linger (camera re-frames on the new
      target); Chrome-hide mid-linger tears down immediately.
- [ ] 12_hud_range: INVERT the post-kill inset asserts (currently pin the
      old teardown at ~+0.4 s after the kill) into the kill-cam live pin:
      one RTT camera still up + panel visible after the target ship dies.
      Expiry-closes stays unit-tested (the 6 s autopilot window ends
      before the linger does).
- [ ] fmt + check; target_inset + gesture filters; the three autopilots
      (defer live runs if the user's game instance is up - contention
      flake documented in 20260713-124000).

## Notes

- Spike: docs/spikes/20260713-154023-inset-kill-cam.md (options B vs C/D,
  the free safety-click composition).
- Open question resolved at plan time: an active radar SEARCH (no lock
  yet) does NOT end the linger early - only a landed lock re-frames; the
  sweep needs a lock to show anything anyway, and the preemption pin
  covers the moment it lands.
- Player death mid-linger: the panel despawns with the HUD (both state
  components live ON the panel entity), so nothing dangles by
  construction.
