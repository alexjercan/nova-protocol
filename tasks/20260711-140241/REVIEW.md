# Review: Rotation command handoff crosses clocks

- TASK: 20260711-140241
- BRANCH: fix/pd-command-handoff-clock

## Round 1

- VERDICT: APPROVE

No findings. Verification performed beyond reading the diff:

- Independently re-derived the spool-tail formula: input decaying at
  `spool_down_rate` from u delivers `sum u(t) * magnitude` per tick =
  `magnitude * u^2 / (2 * spool_down_rate * dt)` of impulse; the code
  matches, and the aligned projection (`dir.dot(error_dir).max(0)`)
  counts only error-reducing impulse - conservative in the right
  direction. The trace's observed through-zero swing (~0.43 u/s at
  u=0.73) is consistent with the estimate plus the pre-cut demand lag.
- Checked cutoff scope and termination: gated on `desired == Vec3::ZERO`
  (never true for ORBIT, irrelevant mid-brake where error >> tail), and
  it cannot stall - the inputs decay under zero demand, the tail shrinks
  quadratically, and the done gate exits through either the epsilon or
  the fine/no-authority branch (both observed in the trace).
- Checked the schedule move against every other registration of the
  copy: the flight harness and the AI standoff harness already wire it
  same-tick (they now MATCH production instead of diverging); the AI
  brain harness chains it in Update after the AI writers, which is
  timing-equivalent for a once-per-frame writer. Ordering against
  `NovaFlightSystems` in apps that never add NovaFlightPlugin (editor)
  is a no-op constraint, not a panic.
- Re-verified the Update-writer latency claim: player/AI/torpedo write
  CSRI once per frame in Update; both the old wiring (copy same frame in
  Update, PD next frame's ticks) and the new one (copy at next frame's
  first tick) deliver to the PD at the same tick. Only the autopilot's
  FixedUpdate writes change: from 1-2 ticks stale to same-tick.
- A/B honesty: staleness regression fails pre-move at 0.048 rad on this
  rig (plugins, real wiring) and reads 0.001 post-move - the bound
  (5e-3) is documented against the f32 angle_between noise floor, which
  is the right call rather than asserting below quaternion precision.
- The arrival regression re-wire is not a weakening: it still fails if
  the cutoff is reverted (0.63 terminal under same-tick wiring) - the
  wiring change moved which mechanism the test guards, and the doc
  comment records the history.
- Suites run in the worktree: flight 60, torpedo 60, ai 86,
  controller_section 4, all green; fmt/check clean. Full workspace suite
  deferred to CI per the user's standing instruction (reported, not
  hidden).
- Diagnostic deletion audited: no dangling references; `diag_ship`
  retained as the shared rig for both regressions, exactly the survival
  the previous review predicted the compiler would enforce.
