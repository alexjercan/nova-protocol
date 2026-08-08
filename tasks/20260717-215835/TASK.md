# Turret joint tree: generalized aim solver (analytic per-joint CCD) + multi-hinge demo

- STATUS: CLOSED
- PRIORITY: 8
- TAGS: v0.7.0, spike, refactor, weapons, turret

## Goal

Replace `update_turret_target_yaw_system` + `update_turret_target_pitch_system`
with a single generalized aim solver: an analytic per-joint CCD sweep. Walk each
muzzle's rotational-DOF chain (muzzle outward to root); for each `TurretJoint`,
transform muzzle + target into that joint's local frame and solve the single-axis
angle that swings the muzzle forward onto the target (the exact primitive the two
current systems already implement), writing `SmoothLookRotationTarget`. Iterate a
bounded number of sweeps per frame. Handle insufficient DOF (best-effort residual)
and shared-joint conflicts across muzzles (default: shared joints aim at target,
private joints refine per muzzle -- see spike open questions).

Ship an end-to-end demo per repo testing guidance: a scenario RON (and/or example
bin via the content CLI) that spawns a non-trivial mount -- a twin-barrel PDC and
a turret with elevation two hinges down -- tracking a moving target, asserting the
muzzles converge on the lead point. Integration tests alongside the existing
turret tests in turret_section.rs.

## Notes

Spike: tasks/20260717-214834/SPIKE.md (aim-solve option A; CCD convergence and
shared-joint open questions). Depends on 20260717-215804 (generic chain).
Direction-level; run /plan for steps.
