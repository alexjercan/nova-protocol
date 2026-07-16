# Retro: Radial lock-acquisition ring HUD (UiMaterial shader)

- TASK: 20260717-004302
- BRANCH: feat/lock-dwell-ring-hud
- REVIEW ROUNDS: 1 (APPROVE, NITs only)

See TASK.md for what shipped; this is process only.

## What went well

- The `screen_indicator` widget did all the projection / visibility / sizing /
  offscreen work, so nova's first `UiMaterial` ring was a genuinely thin
  consumer - one driver system setting an anchor + a `progress` uniform. The
  weapons-HUD spike's "extract the projection substrate early" call paid off two
  arcs later, exactly as predicted.
- Verifying the bevy 0.19 `UiMaterial` / `MaterialNode` / `UiMaterialPlugin` API
  against the actual installed crate source BEFORE writing (not from memory)
  meant the material compiled on the second try (only a `mut` binding nit), for
  a pattern nova had never used.
- The atomic squash-land (`merge --squash && commit` in one command) worked
  first try - the `shared-checkout-write-leak` lesson written in task 1's retro
  compounded within the SAME flow and prevented a repeat tangle.
- An independent out-of-context review pass plus a hand re-derivation of the
  shader angle math (clockwise-from-top at the four cardinals) confirmed the
  load-bearing WGSL before trusting a render I could not run locally.

## What went wrong

- The dwell mechanic (task 1) silently broke `11_hud_range`'s instant-lock
  assertion (`CombatLock` committed at ~+1.1). Root cause: a foreseeable ripple
  - gating the commit behind a dwell moved the commit time, and the example was
  timed against the old instant commit. Caught at compile/reasoning time, not by
  a surprise, but it is the same class as the gesture-test churn in task 1.
- Could not run the example to completion locally: lavapipe software rendering
  is ~100x too slow here to drive the heavy scene (RTT inset + full scenario) to
  the assertion stages within any reasonable timeout. Spent ~3 build+run cycles
  (300s / 90s / 360s) discovering this before falling back to a reported skip.

## What to improve next time

- When a MECHANIC task changes commit/timing, grep the examples/tests that
  script that timing (`grep -rn CombatLock examples/`) as part of the mechanic
  task, so the downstream example fix is anticipated, not discovered a task
  later.
- For heavy `DefaultPlugins` render examples, do not try to run them to
  completion under local lavapipe - validate the logic with headless unit tests
  (driver + asset, no render app) and lean on CI's `examples_smoke`. Budget one
  short smoke attempt only to confirm the render path INITS and the shader
  loads without error, then stop.

## Action items

- [x] Ledger: added `gpu-example-local-skip` (heavy render examples time out
  under local lavapipe; unit-test the logic + rely on CI).
- [ ] No follow-up code task: the three NITs (top-seam sliver at progress=0,
  injected `acquired:false`, missing candidate-drop driver test) are all
  cosmetic / redundantly covered; not worth a task.
