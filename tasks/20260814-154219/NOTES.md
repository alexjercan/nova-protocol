# NOTES

## Diagnosis (confirmed)

The reported diagnosis was right about the mechanism and understated the size of
the trap. It is not a knife edge at exactly 90 degrees of elevation - it is a
whole region of target bearings.

The pitch hinge's demand is the target's angle measured in the hinge's own sweep
plane, from the current yaw heading. A target more than ~90 degrees off that
heading and above the horizon therefore demands MORE than 90 degrees of
elevation, and `SmoothLookRotation` clamps the output to exactly `max`. At
exactly `max` the muzzle forward is exactly parallel to the yaw axis, `d_perp`
collapses, and yaw takes no correction. Both joints are now frozen:

- pitch is saturated and its demand still points further into the limit,
- yaw is blind.

Nothing in the chain moves again. The turret parks pointing straight up.

The trap is reached the ordinary way: a target crossing the zenith flips its
bearing by 180 degrees, which puts it in the >90-degrees-off region while pitch
is still high. Yaw has one frame of stale command left (`out + 0.35 * delta`,
so ~63 degrees) and stops there, well short of the ~90 degrees it needed to pull
the pitch demand back under the limit.

Predates bc227a4f as suspected: only `min` moved there, and the pole depends on
`max` alone.

## Fix

Each articulated joint now remembers the last in-plane heading it could measure
(`TurretJointAimHeading`), stored in the joint's POST-rotation frame so the copy
turns with the joint. At the pole the solver steers by that instead of skipping
the joint, which turns yaw onto the target's bearing; once yaw is within 90
degrees the pitch demand drops back under the limit, the hinge leaves the pole
and the normal solve resumes.

Storing it in the post-rotation frame is the part that makes it converge. Stored
in the joint's pre-rotation frame the reference does not turn with the joint, the
measured error never shrinks, and the yaw spins forever.

### Rejected alternatives

- **Clamp elevation below the pole (85 degrees).** Cheapest, but it gives up
  straight-up point defense, which is the arc the PD fantasy leans on. The
  fallback keeps the full 90.
- **Fall back to the lever arm from the hinge axis to the muzzle** (the standard
  position-CCD step, and what "derive a step from `t_perp` alone" turns into in
  practice). Actively WRONG for the shipped turret: every shipped offset is on
  the yaw axis' plane and the barrel hangs behind the pivot, so at full elevation
  the lever points ASTERN. Aligning it with the target bearing yaws 180 degrees
  the wrong way, which keeps the pitch demand above the limit and keeps the
  turret stuck. The regression covers the shipped chain for exactly this reason.

Also raised the degeneracy guard from `1e-6` to a named `AIM_HINGE_POLE_SIN`
(`1e-3`, ~0.06 degrees). Below that the in-plane component is rounding noise, so
reading a heading out of it - or storing one - is worse than using the last good
one.

## Verification

- `a_turret_tracks_a_target_across_the_zenith`, run against BOTH the default
  chain and a rebuild of the shipped `turret_joint_tree` geometry. Without the
  fallback: 45.3 deg and 46.0 deg off, barrel parked at +Y. With it: under 5 deg
  on both. The other 43 turret tests still pass, including the no-shake and
  stuck-behind regressions.
- Live: `NOVA_AUTOPILOT=1 cargo run --example turret_section --features debug`
  under Xvfb, twice - completes with 0.4 / 0.5 deg tracking on both rounds. No
  regression.

## What did NOT get verified live, and why

Neither live scene drives a PDC to its elevation limit, so the pole itself is
covered by the deterministic regression only.

- **The turret range example.** Its turret section is mounted
  `Quat::from_rotation_x(-FRAC_PI_2)`, which lays the turret on its side: the
  yaw axis points along the ship's -Z and the barrel rests pointing DOWN. Every
  shipped ship mounts turrets at identity (`PartSide::None`), so the range is
  testing a mount the game never uses. Its reachable set is a forward cone of
  ~100 degrees half-angle, and a scripted zenith pass there asks the barrel for a
  bearing it physically cannot reach - the pass was built, measured at 38.5 deg
  off, traced to reachability rather than the pole, and reverted. Worth a look
  on its own account.
- **`NOVA_MENU_BACKDROP=menu_gauntlet`.** 120 s under Xvfb with the pole branch
  instrumented: the branch never fired once. The gauntlet's batteries sit on
  both flanks roughly in the fight plane, so no torpedo ever crosses a PDC's
  zenith.
