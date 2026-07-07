# Retro: Turret aim lead (intercept aim)

- TASK: 20260707-150001
- BRANCH: feature/turret-aim-lead
- PR: #33 (open against master, not merged)
- REVIEW ROUNDS: 1 (APPROVE, two benign NITs)

See `tasks/20260707-150001/TASK.md`. The range built the previous task made this
one fast and its result unambiguous.

## What went well

- The range's telemetry was a ready-made acceptance test. "Aim error against the
  sweeping gate should stay single digits" - I could run the exact same headless
  harness before/after and watch 7-20 deg become 0.1-0.7 deg. No guessing whether
  the fix worked; the number said so. This is the compounding payoff of building
  the range as a task in its own right.
- Reading first found the real shape of the problem: `SmoothLookRotation` already
  caps the slew rate (so "add smoothing" from the task was already done), and the
  turret already had a direct muzzle reference (`TurretSectionMuzzleEntity`). That
  turned "where does the lead live" into a clean answer: a single
  `update_turret_aim_point` producing a public `TurretSectionAimPoint`.
- Chose the component design over the quick one. The first instinct was to compute
  the lead inline in both yaw and pitch systems (duplicated) and expose the pure
  fn so the example could recompute the aim error. Switching to a single
  `TurretSectionAimPoint` component removed the duplication, kept the example
  honest (it reads the same point the turret steers to), and gives a HUD lead
  indicator a hook for free.
- The intercept test asserts *consistency* (bullet and target meet at the same
  point at the same time), not just "leads in +X" - so it would catch a subtly
  wrong quadratic, which a direction-only assert would not.

## What went wrong

- A false start on where to measure aim error: with the lead in place, the
  example's original metric (barrel vs. raw target position) would read *worse*
  because the barrel now points ahead of the target. Caught it while wiring the
  example and switched the metric (and gizmos) to barrel-vs-aim-point. Cost a few
  minutes of rethink, no rework of shipped code.

## What to improve next time

- When adding a lead/predictive aim to a system, remember the observability that
  measured the old behavior measures the *wrong thing* now - update the metric to
  the new aim goal in the same change, or the range will look broken.

## Action items

- [ ] NITs R1.1/R1.2 (constant-velocity intercept approximation; one-frame muzzle
      position) left as-is - sub-degree in practice.
- [ ] Turret range sliders (`20260707-150002`) can now also show the aim-error
      collapse live while tuning `muzzle_speed`.
