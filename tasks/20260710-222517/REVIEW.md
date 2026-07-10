# Review: Smooth the camera when autopilot hands back to manual

- TASK: 20260710-222517
- BRANCH: fix/camera-handback-blend

## Round 1

- VERDICT: APPROVE (findings fixed in-round; see responses)

Verified sound by the reviewer (several claims verified EMPIRICALLY with
scratch tests, reverted after): the ship-side no-lurch path is unchanged
and unconditional; ChaseCameraInput.anchor_rot has zero consumers outside
the bcs chase camera, so the blend cannot touch the PD, targeting or
turrets; the GOTO-to-ORBIT handoff is a Mut write and never fires the
Remove observer (no spurious blend); rig children despawn with the
controller (no accumulation across respawns); exactly one rig is active
per frame and a skipped Single merely pauses the blend; the survey dolly
and the blend write disjoint components; the 2e-3 epsilons match the
acos error model (2*sqrt(2*eps) ~ 1e-3 at f32) rather than masking; the
harness's manual PointRotationOutput write reproduces exactly what the
real bcs insert-observer does; the double-handback pop is documented
where it happens. Tests re-run by the reviewer: camera_controller 9.

- [x] R1.1 (MINOR) camera_controller.rs (on_autopilot_disengaged) - the
  "or it is despawning" guard comment is false: Remove observers see
  sibling components during a despawn flush, so a dead ship's camera DOES
  get a blend. Today the controller teardown removes it (the try_remove
  tuple), but correctness silently depends on nova_scenario's
  camera-revert observer; a future despawn path that skips it would play
  a wrong 0.45s swing on the next controller re-add.
  - Response: fixed - comment rewritten to state what actually happens,
    and insert_camera_controller defensively removes any stale
    CameraHandbackBlend so a fresh controller always starts blend-free.
- [x] R1.2 (NIT) "the next mode switch, which is already smooth"
  overstates: FreeLook-to-Normal after a dormant-rig re-seed pops to the
  hull attitude (pre-existing, out of scope).
  - Response: fixed - observer comment, test doc and docs file now call
    it a separate, pre-existing transition instead of claiming
    smoothness.
- [x] R1.3 (NIT) CameraHandbackBlend derives Reflect but is not
  register_type'd.
  - Response: left as is - consistent with every other component in this
    file; a file-wide registration pass is separate cleanup.
