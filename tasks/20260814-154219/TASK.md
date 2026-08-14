# PDC turrets can park at full elevation and stop tracking

- STATUS: OPEN
- PRIORITY: 60
- TAGS: v0.11.0,ship,turret

# PDC turrets can park at full elevation and stop tracking

Owner report (2026-08-14, live play): a PDC gets stuck pointing straight
up (the pitch hinge's +90 degree limit) and stays there instead of coming
back down onto a target.

## Where

`crates/nova_ship/src/sections/turret_section/aim.rs`,
`update_turret_target_joints_system` - the per-frame Jacobi hinge-CCD pass
that steps each articulated joint toward the aim point.

## Diagnosis (unconfirmed, from reading - reproduce first)

A pole singularity at full elevation. The solver decomposes the aim error
in each joint's hinge plane:

```rust
let d_perp = dl - a * dl.dot(a);
let t_perp = des - a * des.dot(a);
if d_perp.length() > 1e-6 && t_perp.length() > 1e-6 {
```

For the YAW joint the hinge axis `a` is +Y. At 90 degrees of pitch the
muzzle forward `dl` is parallel to +Y, so `d_perp` collapses to zero, the
guard fails, and the yaw joint receives NO correction that frame. Yaw is
the only joint that can fix a heading error, so once the chain reaches the
pole it can sit there: pitch alone cannot reduce the error, and nothing
pushes the muzzle off the axis to restore yaw authority.

The elevation limit itself (`max: FRAC_PI_2`) is long-standing and was NOT
touched by the depression change in bc227a4f, which moved the `min` floor
only (-30 -> -10 degrees). Worth confirming the bug predates that commit
rather than assuming it.

## Repro to establish

Not yet reproduced deterministically. Suggested: a headless aim test that
steers a default turret at a target directly overhead, steps frames, then
moves the target to a new bearing and asserts the muzzle converges. The
existing `a_default_turret_converges_its_muzzle_onto_a_target` test in the
same file is the harness to extend - it only covers a reachable target
from a rest pose.

## Fix directions (pick after the repro)

- Nudge the chain off the pole when `d_perp` collapses: fall back to a
  small yaw step derived from `t_perp` alone, so the muzzle leaves the
  axis and normal correction resumes the next frame.
- Or clamp elevation just below the pole (e.g. 85 degrees) so the
  singularity is unreachable. Cheapest, but it gives up straight-up point
  defense, which is the arc the PD fantasy leans on - measure before
  choosing this.

## Done when

- A failing test reproduces the stuck turret, then passes.
- Verified in a live run (`Xvfb` + a scenario with inbound torpedoes -
  `NOVA_MENU_BACKDROP=menu_gauntlet` is the densest PD scene), not by
  exit status alone.
