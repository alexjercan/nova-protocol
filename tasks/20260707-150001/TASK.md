# Turret aim lags moving targets: add lead (and/or smoothing) to the slew

- STATUS: CLOSED
- PRIORITY: 80
- TAGS: v0.4.0,turret,bug

Surfaced by the turret test range (task 20260707-095008, `examples/08_turret_range.rs`).
The turret aims by slewing its yaw/pitch rotators toward the target's *current*
position at a fixed angular rate (`update_turret_target_yaw_system` /
`update_turret_target_pitch_system` in `crates/nova_gameplay/src/sections/turret_section.rs`),
with no lead and no smoothing. Against a moving target the barrel therefore
tail-chases: in the range the aim error catches the sweeping gate down to ~7 deg,
then breathes back up to ~20 deg and oscillates as the gate reverses direction -
the "clunky" feel. A PDC that never settles on a crosser also wastes most of its
fire.

Expected: the turret leads a moving target (aims where it will be, given bullet
`muzzle_speed`), so the aim error against a constant-velocity crosser stays small.

## Steps

- [x] Lead solution: `lead_intercept_point(shooter, target, target_vel, muzzle_speed)`
      solves the projectile-intercept quadratic for the smallest positive time-to-hit
      and returns where the target will be. Target velocity is carried on the turret
      via a new `TurretSectionTargetVelocity` (set by whoever aims - the range feeds
      the gate's `LinearVelocity`; the player crosshair leaves it zero, so a fixed aim
      point is unchanged). A new `update_turret_aim_point` writes the intercept into a
      public `TurretSectionAimPoint`, and the yaw/pitch systems steer to that.
- [x] Smoothing already exists (`SmoothLookRotation` caps the slew rate); no deadzone
      was needed - with the lead the barrel sits sub-degree on target, no jitter.
- [x] Verified in `08_turret_range`: aim error against the sweeping gate is now
      **0.1-0.7 deg** (was 7-20 deg and oscillating). 3 unit tests on the pure lead
      function (stationary -> target; crossing -> bullet and target meet; uncatchable
      -> safe fallback).

## Resolution

The turret slewed toward the target's *current* position, so it tail-chased movers.
Added a lead: `lead_intercept_point` (pure, tested) computes the intercept from target
position + velocity + bullet `muzzle_speed`; `update_turret_aim_point` resolves it into
`TurretSectionAimPoint`; the yaw/pitch systems now steer to the aim point. Target
velocity comes from a new `TurretSectionTargetVelocity` (zero for a stationary crosshair
aim -> no lead, unchanged). The range's `range_aim` feeds the moving gate's velocity and
its telemetry/gizmos read `TurretSectionAimPoint`. Headless the aim error dropped from
7-20 deg to sub-degree; 29 nova_gameplay tests, clippy, and both example builds are green.

## Notes

Source: `crates/nova_gameplay/src/sections/turret_section.rs`
(`update_turret_target_yaw_system`, `update_turret_target_pitch_system`, `muzzle_speed`).
The torpedo PN guidance (task 20260525-133021) is the reference for a lead solution.
