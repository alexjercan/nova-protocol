# Spike: client-side smoothing and deadbands on PD and camera outputs

- DATE: 20260711-140234
- STATUS: RECOMMENDED
- TAGS: spike, feel, flight, pd, camera
- TASK: tasks/20260711-125227

## Question

The playtest asked for feel-layer filtering: "would it be possible to have
some clamping done or smoothing on top of the calculations we get from the
PD controller or from the camera ... like ignoring small numbers". The
follow-up report sharpened it: the ship "wobbles" on GOTO but STOP is
perfect, and "the PD controller doesn't sync that well with the game".
The spike's question: WHERE in the chain (PD output, PD command, autopilot
error, render pose, camera) does the residual micro-motion actually come
from, and which layer is the safe one to filter? A good answer names the
mechanism with numbers and picks a layer with no control-theory risk.

## Context

- The two-clocks family (tasks/20260711-103527/SPIKE.md) fixed every
  FixedUpdate consumer of render-clock poses; the hull is provably steady
  on a constant-command decel (tasks/20260711-121701/RETRO.md: max spin
  0.0023 rad/s over a 22 s burn - regression-pinned).
- Camera side landed separately: the chase-move ordering pin (5ba0e3c)
  and the velocity lead on the zoom lag (eced738). The user reports the
  camera "is no longer affected by jitter".
- So the remaining wobble had to be command-side and GOTO-specific: STOP
  and GOTO share the burn/balance/PD machinery; they differ only in the
  desired-velocity source (constant zero vs replanned-every-tick).
- The PD conditioning history (tasks/20260709-125640/RETRO.md) makes
  physics-side deadbands on the PD a known minefield: limit cycles,
  attitude droop, saturation flip-flops. Required reading before touching
  that layer; this spike does not touch it.

## Diagnostic (numbers first, per family convention)

`goto_wobble_diagnostic` (flight.rs tests, `#[ignore]`d): the shipped
5-section player geometry (PD 4/4/40, single rear drive, 12.8 u/s^2),
GotoPos 30 deg off the nose over ~670 u, 60 fps frames against the 64 Hz
fixed tick. Probes: hull spin per frame, and - inside FixedUpdate, after
`PDControllerSystems::Sync` - the angle between the command the autopilot
wrote this tick and the `PDControllerInput` the PD actually consumed.

Run under four variants (test wiring = command copy in FixedUpdate;
production wiring = copy in Update, as shipped):

| variant                | terminal spin max/rms | release spin | pd stale max | mean torque |
|------------------------|-----------------------|--------------|--------------|-------------|
| test wiring, db 0.4    | 0.67 / 0.42 rad/s     | 0.31         | 0.003        | 5.09        |
| PROD wiring, db 0.4    | 0.59 / 0.38 rad/s     | 0.44         | 0.222        | 6.21        |
| PROD wiring, db 0.6    | 0.28 / 0.12           | 0.09         | 0.048        | 4.35        |
| PROD wiring, db 0.75   | 0.10 / 0.03           | 0.05         | 0.048        | 3.96        |

(terminal = the last ~15 u of the approach plus the settle; release spin
is what the hull still carries when the autopilot disengages; max lateral
path deviation is 4.41 u in ALL variants - the deadband never touches
mid-course tracking on this leg.)

## Findings

### 1. The GOTO wobble is a terminal attitude hunt (primary)

A GOTO leg crosses the standoff boundary hot by design (~3.5 u/s here:
the arrival speed limit still allows it, and `min_approach_speed` floors
it - that is what makes the ship actually cross instead of approaching
asymptotically). The doorstep brake inside the boundary leaves a residual
velocity error of ~0.45-0.6 u/s pointing where the drive is NOT - and
that residual sits just ABOVE the shipped `attitude_deadband` (0.4). The
"crumbs" rule ("never re-aim the hull for it") therefore never engages at
exactly the moment it was written for: the computer re-aims the hull
after a sub-visible velocity error at 0.4-0.6 rad/s (~25 deg/s of nose
swing) for 3-5 seconds at the end of EVERY GOTO leg, slowly eroding the
error until it creeps under the band. STOP never hunts: its error
direction is constant (retrograde), the nose stays ON the error, and the
done gate releases at 0.2 u/s while the drive still has axial authority.

The deadband's own tuning comment ("a 0.4 u/s drift is a slow creep
nobody notices") is correct - the bug is that the typical doorstep
residual is 0.45-0.6, so the band is tuned BELOW the very crumb it exists
to ignore.

### 2. The command handoff crosses clocks (secondary, "PD doesn't sync")

`update_controller_section_rotation_input` copies the rotation command
into the bcs `PDControllerInput` in the UPDATE schedule, while both the
autopilot (producer) and the PD (consumer) run in FixedUpdate. The PD
chases a command 1-2 ticks stale, varying with the 64 Hz-vs-render beat.
Invisible for constant commands (why 20260711-121701 could rightly
dismiss it), but a GOTO command slews every tick: measured phantom PD
error up to 0.22 rad (12.7 deg) during the flip, mean torque 22% higher
than the same leg with a same-tick handoff. This is a permanent
micro-jitter/wasted-torque layer on every autopilot maneuver (torpedo
controller sections included). It does NOT cause the arrival hunt (the
hunt reproduces under both wirings).

Corollary: the flight test harness already wires the copy in FixedUpdate
- every existing "hull steady" regression runs a wiring production does
not have. Production should be fixed TO the harness. This is the fourth
"rig didn't match production" instance in this family.

## Options considered

- **(a) Deadband/epsilon on PD output torque** (the playtest's literal
  ask) - REJECTED. The terminal torque is not noise; it is faithfully
  executing a hunting COMMAND. Filtering the output would fight the
  loop's own commands, risks limit cycles and attitude droop (residual
  roll history), and interacts with the release-spin and AI-settle
  guards. Wrong layer.
- **(b) Slew/low-pass on the PD command input** - already exists
  (`slew_rotation`, urgency scaling); the command is smooth. The problem
  is WHERE it points (crumb-chasing), not how fast it moves. No change.
- **(c) Widen the settle deadband in the desired-velocity-zero regime**
  (arrival inside the standoff, STOP at rest) - RECOMMENDED. This is the
  existing, proven "ignore small numbers" mechanism, tuned above the
  actual doorstep residual (0.75 measured: hunt gone, terminal rms
  0.38 -> 0.03 rad/s, release spin 0.44 -> 0.05, path tracking
  unchanged). Scoping it to the desired==0 regime keeps ORBIT
  station-keeping and mid-course drift correction on the tight 0.4 band
  and preserves the documented `orbit_hold_enter = 2x deadband`
  relationship. STOP precision is untouched by construction (nose on
  error -> aligned authority -> the tight 0.2 done gate still decides).
  A global raise to 0.75 is the simpler runner-up and measured equally
  well on this leg; it costs the ORBIT band relationship and mid-course
  tolerance, so the scoped form wins.
- **(d) Fix the cross-clock command handoff** - RECOMMENDED alongside
  (c), as its own cycle: move the copy to FixedUpdate ordered between
  the command writers and `PDControllerSystems::Sync` (pinning the
  currently-unordered NovaFlightSystems vs PDControllerSystems::Sync).
  Removes the 0-12.7 deg phantom error and ~20% torque waste; restores
  harness fidelity. Zero-risk for manual input (same one-frame latency,
  minus beat aliasing).
- **(e) Render-clock filtering (smooth the eased Transform / camera
  anchor)** - NOT NEEDED for this wobble: the hull PHYSICALLY oscillates;
  smoothing the render would hide real motion behind lag. Keep in the
  back pocket for residual micro-motion after (c)+(d) land.
- **(f) Camera micro-motion deadband in bcs lerp_and_snap** - DEFERRED.
  The camera fixes already landed (ordering pin, velocity lead) and the
  user reports the camera clean; no measured camera-side residual is on
  file to size a deadband against. Revisit only if the post-fix playtest
  still reports micro-motion with the hull quiet.
- **Do nothing** - the wobble is at the end of every GOTO leg, i.e. the
  most-watched moment of the autopilot's showpiece verb. Not acceptable
  for a feel-focused release.

## Recommendation

Two implementation tasks, independent, both regression-first:

1. `tasks/20260711-140234` (P58): widen the settle deadband in the
   desired-velocity-zero regime (option c), value ~0.75 per the
   measurements, knob shape decided in /plan. Regression: terminal-phase
   max spin < ~0.15 rad/s and release spin < ~0.1 on the shipped rig
   (shipped config fails at 0.59).
2. `tasks/20260711-140241` (P56): move the command copy to FixedUpdate
   with explicit ordering, align production with the test harness
   (option d). Regression: PD consumed command == same-tick written
   command during an active slew (shipped wiring fails at 0.22 rad).

What happens to the existing snap thresholds and guards under the wider
band (the task file's explicit question): the post-release spin guard
(0.5) gains margin (release spin measured 0.44 -> 0.05 - the shipped
config was one bad frame from tripping its own guard); the AI settle
roll bound (0.05) is untouched (it gates PD roll damping, not re-aim
scheduling); ORBIT's hold gates keep their 2x relationship because the
tight band still rules everywhere desired != 0; STOP's release stays at
`stop_speed_epsilon` via the aligned-authority argument above.

## Open questions

- Exact knob shape and value for the settle band (single field vs
  multiplier; 0.6 vs 0.75) - decided by the fix task against the full
  guard suite.
- Whether the brake-tail sawtooth (the limit curve re-allowing
  acceleration just before the boundary, forcing a late re-aim) deserves
  its own smoothing (a "never speed back up once braking inside the
  final ramp" rule). The wider settle band already absorbed the visible
  part on the measured leg (terminal max 0.10 at 0.75); only revisit if
  a longer/heavier rig still shows it.
- Whether any camera-side micro-deadband (option f) is wanted at all
  after (c)+(d) - needs a fresh playtest report, not code.

## Next steps

- tatr 20260711-140234: quiet the GOTO arrival hunt (settle deadband,
  desired-velocity-zero regime) - P58.
- tatr 20260711-140241: rotation command handoff to FixedUpdate + harness
  alignment - P56.
- The `#[ignore]`d `goto_wobble_diagnostic` in flight.rs carries the
  full A/B rig for both tasks; each converts its half into a regression
  and the LAST one deletes the diagnostic per convention.
- After both land: user playtest of GOTO feel (and the camera, to
  confirm it stays clean) before considering options (e)/(f).

## Fix record

### 20260711-140234: settle deadband (landed)

- Shipped as `FlightSettings::settle_deadband` (0.75) scoped BY LEG:
  STOP/GOTO/GotoPos use it for both the crumb band and the urgency
  denominator; ORBIT keeps the tight `attitude_deadband`. Production-wired
  regression `goto_arrival_settles_without_hunting` pins terminal spin
  < 0.15 and release spin < 0.1 (pre-fix: 0.59).
- TWO of this spike's claims were falsified by the implementation and
  corrected (details in tasks/20260711-140234/TASK.md):
  1. Scoping the band to `desired == Vec3::ZERO` does NOTHING (terminal
     spin bit-for-bit unchanged) - the hunt's onset is in the brake tail
     where desired is still nonzero. The working scope is by leg.
  2. The spike's deadband A/B was CONFOUNDED: the global raise moved the
     crumb band AND the urgency denominator (both keyed to
     attitude_deadband) together. The band alone does not kill the hunt;
     band + urgency scoped together reproduce the spike numbers exactly.
- NEW FINDING, load-bearing for 20260711-140241: the arrival dynamics are
  WIRING-DEPENDENT. Under the shipped Update-schedule command copy the
  fix yields terminal 0.097/release 0.047; under the same-tick FixedUpdate
  copy (the harness wiring, and 140241's planned production change) the
  hunt SURVIVES the fix (terminal 0.63, release 0.19, arrival takes 2x
  longer) - the one-frame command staleness acts as accidental dither
  that breaks the re-aim/overshoot limit cycle, and the same-tick loop
  phase-locks it. 140241 must fix the hunt under same-tick wiring before
  moving the copy, and the production-wired regression is the gate.

### 20260711-140241: same-tick command handoff + spool-tail cutoff (landed)

- The copy into `PDControllerInput` now runs in FixedUpdate between the
  flight systems and `PDControllerSystems::Sync`
  (ControllerSectionPlugin); regression
  `autopilot_command_reaches_the_pd_on_the_same_tick` runs the REAL
  plugins and pins the same-tick handoff (Update wiring fails at
  0.048-0.22 rad phantom error).
- The wiring-dependence finding RESOLVED at the mechanism level: the
  "accidental dither" was never damping a rotational re-aim cycle - the
  hunt was a POSITIONAL bounce on the standoff boundary, driven by the
  finishing burn ignoring its throttle spool-down tail and burning
  through zero (trace in tasks/20260711-140241/TASK.md). The spool-tail
  cutoff in autopilot_system removes the overshoot at its source; both
  wirings now arrive quietly, the falsification exit was not needed, and
  the family's regressions all run one wiring: the shipped one.
- Diagnostic machinery deleted per convention (goto_wobble_diagnostic,
  diag_app, probe); diag_ship survives as the shared regression rig.
- Family status: all three spike-seeded threads are closed - settle
  band (140234), handoff clock + overshoot (140241), and the bullet
  first-frame seed (121839, separate task). Remaining from the spike's
  open questions: none blocking; camera options (e)/(f) stay deferred
  pending the user's post-fix playtest.
