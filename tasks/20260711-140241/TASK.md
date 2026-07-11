# Rotation command handoff crosses clocks: move the PD input copy to FixedUpdate and align the test harness

- STATUS: CLOSED
- PRIORITY: 56
- TAGS: v0.5.0, bug, physics, flight

## Steps

- [x] Diagnose the same-tick limit cycle from the WARNING (prerequisite).
      Traced frame-by-frame at the terminal phase: it is a BOUNDARY
      BOUNCE, not re-aim chatter. The finishing burn of the doorstep
      brake keeps demanding until the error reads zero, but the throttle
      spool-down tail (~magnitude * input^2 / (2 * spool_down_rate * dt))
      keeps delivering impulse after the demand drops - the ship burns
      THROUGH zero (trace: +0.73 u/s killed to +0.08, then reversed to
      -0.81), exits its own standoff backwards, and the re-entry error
      (~2.3 u/s > band) re-aims the hull 180. Under the stale wiring the
      identical overshoot happened to land at 0.58 (under the band,
      inside the boundary) - the "accidental dither" was just which side
      of the knife edge the overshoot fell on.
- [x] Fix the cycle at its source: spool-tail cutoff in autopilot_system
      - for legs at desired == 0, once the aligned engines' wind-down
      tail alone covers the remaining error, the demand is zero (cut and
      coast to rest). Same-tick arrival now releases at f=975 with
      terminal spin ~0.1 (was: 700+ frames of hunting at 0.63).
- [x] Move the copy to FixedUpdate in ControllerSectionPlugin, ordered
      after(NovaFlightSystems).before(PDControllerSystems::Sync) - the
      pair transitively pins the previously-unordered
      NovaFlightSystems-vs-Sync relation. Update-schedule writers
      (player, AI, torpedo guidance) keep their exact one-frame latency
      (their command changes once per frame).
- [x] Regression `autopilot_command_reaches_the_pd_on_the_same_tick` on
      the REAL plugins (NovaFlightPlugin + ControllerSectionPlugin, not a
      hand-wired copy): probe after PDControllerSystems::Sync asserts the
      PD consumed this tick's command during an active slew, with
      delivery guards (100+ ticks sampled, command actually slewing).
      A/B: Update wiring fails at 0.048 rad (this rig; 0.22 during full
      flips), same-tick reads 0.001 = the f32 angle_between noise floor
      (bound 5e-3 documents why).
- [x] Re-wire `goto_arrival_settles_without_hunting` to the harness
      wiring (= new production semantics) and keep it green; delete the
      spike diagnostic machinery (goto_wobble_diagnostic, diag_app,
      DiagTrace, probe and the scratch trace; diag_ship stays - both
      regressions use it). Suites: flight 60, torpedo 60, ai 86,
      controller_section 4 - all green; fmt+check clean; full workspace
      suite deferred to CI per user instruction.

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

## Resolution

What changed: (1) spool-tail cutoff in autopilot_system's burn demand -
the root fix for the arrival boundary-bounce that the WARNING predicted
the wiring move would unmask; (2) the command copy moved from Update to
FixedUpdate in ControllerSectionPlugin with explicit ordering between
the flight systems and the PD sync; (3) the same-tick staleness
regression on the real plugins; (4) the arrival regression re-wired to
the now-unified wiring; (5) spike diagnostic machinery deleted per
convention.

Evidence rig (record-the-rig rule): trace = flight harness wiring
(same-tick copy), shipped 5-section geometry, GotoPos (300,0,-600),
frame-by-frame velocity/spin/command/throttle at remaining < 10;
staleness rig = MinimalPlugins + physics + PDControllerPlugin +
NovaFlightPlugin + ControllerSectionPlugin{render:false} +
thruster_impulse_system, probe after PDControllerSystems::Sync.

The WARNING's "staleness is load-bearing damping" hypothesis resolved
better than feared: the staleness was never damping anything - it just
randomized which side of the settle band the doorstep overshoot landed
on. Killing the overshoot at its source (the spool tail the demand
ignored) made BOTH wirings quiet and the move safe. The falsification
exit was not needed.

Self-reflection: the frame-by-frame trace at the exact failing phase
found in minutes what schedule-level reasoning had mislabeled
("dither breaks the limit cycle" was directionally right but named the
wrong mechanism - the cycle was positional, through the standoff
boundary, not rotational through the re-aim gate). Also, asserting
angles between near-identical f32 quaternions has a ~1e-3 noise floor;
bounds must sit above it and the test should say why.
