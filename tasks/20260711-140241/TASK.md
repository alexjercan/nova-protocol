# Rotation command handoff crosses clocks: move the PD input copy to FixedUpdate and align the test harness

- STATUS: OPEN
- PRIORITY: 56
- TAGS: v0.5.0,bug,physics,flight

## Goal

Found by the feel spike (docs/spikes/20260711-140234-feel-filtering.md)
while hunting the GOTO wobble; matches the playtest note "the PD
controller doesn't sync that well with the game".

`update_controller_section_rotation_input` (the one-line copy from
`ControllerSectionRotationInput` to the bcs `PDControllerInput`) runs in
the Update schedule (controller_section.rs), while BOTH its producer (the
autopilot writes the command in FixedUpdate) and its consumer (the bcs PD
reads `PDControllerInput` in FixedUpdate, `PDControllerSystems::Sync`)
tick on the fixed clock. The PD therefore always chases a command 1-2
ticks stale, and the staleness varies with the 64 Hz vs render-rate beat:
on a double-tick frame the second tick fights a command 2 ticks old.

A constant command (manual mouse-hold, STOP's retrograde) hides this
completely - which is why the decel-wobble falsification (20260711-121701)
correctly dismissed it for that scenario. A GOTO command SLEWS every tick
(align turn, flip-and-burn, corrections), and there the spike measured
the PD fighting a phantom command error up to 0.22 rad (12.7 deg) with
mean torque 22% higher than the same leg under a same-tick handoff -
permanent micro-jitter injected into every autopilot maneuver, plus
wasted torque budget. Torpedo controller sections inherit the same seam.

Also fix the harness divergence this uncovered: the flight test harness
(`unfinished_flight_app`) already wires the copy in FixedUpdate chained
after the autopilot - so every "hull is dead steady" regression runs a
wiring PRODUCTION DOES NOT HAVE. The fix makes production match the
harness (same-tick handoff), not the other way around.

Direction:

- Move the copy to FixedUpdate in `ControllerSectionPlugin`, ordered
  after the command writers and before `PDControllerSystems::Sync`.
  NovaFlightSystems and PDControllerSystems::Sync are currently mutually
  UNORDERED in production FixedUpdate (both are only `.before(
  SpaceshipSectionSystems)`) - benign today, load-bearing after the move;
  pin the order explicitly.
- Player mouse input keeps writing the command in Update; its latency is
  unchanged (written frame N, consumed by frame N+1's first tick - the
  same as today, minus the beat aliasing).
- Regression (A/B-proven by the spike diagnostic): during an active
  autopilot slew, assert the PD's consumed input equals the same tick's
  written command (angle < ~1e-3) on every tick; the Update wiring fails
  at up to 0.22 rad. Probe inside FixedUpdate after
  PDControllerSystems::Sync, as `goto_wobble_diagnostic` does.

## WARNING from 20260711-140234 (read before planning)

The arrival dynamics are WIRING-DEPENDENT and the naive version of this
task REINTRODUCES the GOTO wobble that 140234 fixed. Measured on the
settle-deadband build (spike fix record): under the shipped Update-copy
wiring the arrival is quiet (terminal spin 0.097, release 0.047); under
the same-tick FixedUpdate copy this task proposes, the SAME build hunts
at arrival (terminal 0.63, release 0.19, leg takes twice as long) - the
one-frame command staleness acts as accidental dither that breaks the
re-aim/overshoot limit cycle, and a same-tick loop phase-locks it. The
production-wired regression `goto_arrival_settles_without_hunting`
(flight.rs) is the gate: moving the copy flips that rig's wiring
assumption, so this task must (a) diagnose and fix the same-tick limit
cycle FIRST (candidates: rate-limit the command slew against the PD's
tracking lag, a one-tick command pipeline delay made explicit, or
hysteresis on re-aim entry), then (b) re-wire the regression to the new
production schedules and keep it green. If the limit cycle resists a
clean fix, closing this task as "falsified: the staleness is
load-bearing damping; keep the Update copy and document it" is a
legitimate outcome - the phantom-error cost (0.22 rad peak, ~20% torque)
buys arrival stability that would otherwise need an explicit mechanism.

## Notes

- Spike: docs/spikes/20260711-140234-feel-filtering.md. The `#[ignore]`d
  `goto_wobble_diagnostic` in flight.rs contains the probe and both
  wirings; reuse, then delete per the diagnostic convention (140234
  landed first and kept it for this task).
- This does NOT fix the GOTO arrival wobble (that landed in 140234). It
  removes the micro-jitter layer and restores test-rig fidelity - the
  fourth "the rig didn't match production" instance in this family
  (plan-skill lesson thread).
- Cross-repo note: no bcs change needed; the copy system and the
  schedules involved are all nova-side.
