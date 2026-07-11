# Retro: Torpedo raw-clock launch

- TASK: 20260711-114640
- BRANCH: fix/torpedo-spawn-raw-clock (squash-merged as 883b01d)
- REVIEW ROUNDS: 1 (APPROVE, no blocking findings)

A short, clean cycle: the third application of a twice-proven pattern.

## What went well

- **Pattern reuse compounded across the family.** The raw-pose spawn
  (turret, 20260710-231930), the easing seed (bullet pop,
  20260711-121839, whose review explicitly asked this cycle to pick it
  up), and the real-plugin regression rig (handoff, 20260711-140241) all
  transferred directly; the only new judgment was keeping tick-quantized
  launch timing, documented at the spawn site.
- **Helper promotion at second use**: `local_pose_in_root` moved to
  sections/mod.rs when the second consumer appeared, exactly when the
  task file suggested it - no premature abstraction, no third copy.
- **The regression asserts both clocks in one rig** (raw Position on the
  raw bay line, first rendered Transform on the rendered bay line), so
  either half regressing fails one test with a named clock.

## What went wrong

- Nothing substantive. The pre-planned rig needed one collateral repair
  (the allegiance test's bare-World spawner was not in the ship's mount
  chain and lacked raw-pose components) - anticipated during planning,
  fixed in minutes.

## What to improve next time

- When a spawn-shaped system changes its query shape, grep the test
  modules for bare-World `run_system_once` rigs FIRST and list them in
  the plan; they break structurally, not behaviorally, and are easy to
  pre-empt.

## Action items

- [x] Fuze render-clock staleness recorded as observed-and-left in
      TASK.md (gameplay tolerance, not a physics bug; file on playtest
      evidence only).
- [x] Ledger: `two-clocks` family note extended with the shared-helper
      promotion; no new lessons - the existing ones carried the cycle.
