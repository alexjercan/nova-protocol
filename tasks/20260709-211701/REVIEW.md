# Review: Turret lead ignores inherited shooter velocity

- TASK: 20260709-211701
- BRANCH: fix/turret-lead-inherited-velocity (implementation commit 6b58297)

## Round 1

- VERDICT: APPROVE

Verified independently: fmt clean, `cargo check --workspace` green, 8 turret
tests pass, 12_hud_range full PASS. The frame math is right (checked the
derivation: solving with v_target - v_muzzle and aiming the barrel at the
resulting point makes dir*s*t + v_muzzle*t land exactly on target + v_t*t),
and the shooter velocity uses the IDENTICAL point-velocity + COM-lift
computation as shoot_spawn_projectile, so aim-time and fire-time physics
cannot drift apart. The formation-flight test is the direct encoding of the
reported bug (world-frame solve would lead a relatively-stationary target);
the strafing test closes the loop with a bullet-meets-target consistency
check rather than just a sign assertion. Solve-side fix over feed-side is
the right call: all three feeders corrected at once, component semantics
unchanged (docs updated on all three touched doc comments). Angular swing is
included via the muzzle point velocity, matching the spawn exactly.

No findings.
