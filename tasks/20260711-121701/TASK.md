# Residual wobble while decelerating from high speed

- STATUS: OPEN
- PRIORITY: 88
- TAGS: v0.5.0,bug,physics,flight

## Goal

Playtest (user, 2026-07-11, AFTER the raw-pose impulse fix 20260711-103527
and the corkscrew fix f43e550 landed): "the spaceship is a bit unstable
and wobbles a bit when decelerating". The gross flip is gone and stopping
with X reads sharper (user-confirmed), but a residual decel wobble
remains. This is a NEW observation, not a reopen of 20260710-231931 (whose
cross-velocity regression pins zero spurious torque from the application
point).

## Steps

- [ ] Diagnostic first (family convention): an `#[ignore]`d tick trace of
      a scripted high-speed stop (~200-300 u/s, flip + retrograde burn +
      drift correction) logging per tick: angular velocity, command error,
      PD output, per-engine balancer throttles (primary vs recruit), and
      the alignment-gate decisions. Classify the wobble: recruit set
      flip-flop at the align_cos gate (bang-bang), PD-vs-balancer
      interplay, torque-cap saturation cycling, or genuinely physical
      drift correction.
- [ ] Check the one remaining known staleness in the loop: the thruster
      child's avian pose and any controller inputs written in Update
      (control staleness is one frame; at 300 u/s quantify whether it
      matters for the wobble amplitude).
- [ ] Fix per diagnosis (likely candidates: hysteresis width on the
      align gate, recruit throttle smoothing/spool, or PD gain shaping
      near the flip) - keep the fix on the mechanism the trace shows, not
      a feel-tune on top of an oscillator.
- [ ] Regression pinning the measured wobble amplitude bound during a
      scripted stop; delete the diagnostic in the same branch.

## Notes

- Context: docs/spikes/20260711-103527-twitching-family-two-clocks.md
  (fix record), docs/retros/20260709-125640-residual-roll-release.md
  (trace-first discipline, falsified-theory bookkeeping).
- Related guards that must stay green:
  high_speed_stop_settles_without_tumbling (settle bound),
  cross_velocity_burn_keeps_the_hull_steady_at_high_speed (application
  point), off-axis counter-torque tests (balancer).
- Filed from user feedback mid-flow (2026-07-11); not part of the
  20260711-094915 umbrella's original four, but the umbrella's combined
  verification should mention it.
