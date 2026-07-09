# Review: Turret auto-fire feed from the ship/component lock

- TASK: 20260709-173700
- BRANCH: feature/turret-lock-feed (implementation commit d5571b4)

## Round 1

- VERDICT: APPROVE

Verified independently: fmt clean, `cargo check --workspace` green, 35 input
tests pass (5 new tier tests plus the adapted 150711 aim-ray test, which
correctly gained the two lock resources and the velocity component - its
turret would otherwise silently not match the widened query), and the
scripted 12_hud_range run passes end to end including the new world-space
discriminator (turret aim point within 5 m of the locked ship's anchor,
where the old camera-ray feed would sit ~50 m short; the dead-ahead geometry
makes screen-space assertions blind to the tier, so world-space is the right
probe). Tier fall-through is same-frame and tested for both dead sections
and dead locks. The feed ordering after SpaceshipTargetingSystems is
correct and commented. Behavior change (locked turrets no longer slave to
the crosshair) is the spike's explicit decision; the manual path survives as
the no-lock tier. The Xvfb freeze during verification is honestly recorded
in the Resolution with its root cause.

No findings.
