# ORBIT autopilot verb: circularize and station-keep inside a gravity well

- TASK: 20260709-193339
- SPIKE: tasks/20260709-193147/SPIKE.md
- MODULE: crates/nova_gameplay/src/flight.rs (verb), input/player.rs (key),
  hud/flight_status.rs (readout + cue)

## What was built

The third diegetic autopilot verb (spike recommendation D, second half).
Inside a well, O (gamepad South) engages `AutopilotAction::Orbit`; the
existing maneuver machine flies a real insertion through the real actuators:

- **Plan** (first engaged tick): target ring = current radius clamped into
  the stable band (`orbit_target_radius`: above 1.5x the surface clearance,
  below 0.9x the fade-band start); orbit plane from `r x v`
  (`orbit_plane_normal`), falling back to the pilot's horizon (ship up
  rejected onto the radial) when the velocity is near-zero or near-radial.
  The plan is stored in the action and stays sticky - a per-tick replan
  would chase its own drift; re-engaging replans.
- **Align / Burn**: `orbit_desired_velocity` = tangential
  `circular_orbit_speed(mu, R)` plus one bounded arrival-curve correction
  toward the nearest ring point (radial and out-of-plane error folded into
  a single term). Everything downstream - group choice, wrench allocation,
  spool, deadband - is the existing machinery, untouched.
- **Hold**: a new `AutopilotPhase::Hold` label with enter/exit hysteresis on
  the velocity error (`orbit_hold_enter`/`orbit_hold_exit`). Micro-burns
  keep firing inside Hold when drift exceeds the attitude deadband - that
  IS the station-keeping; the phase is a readout, not a mode switch.

ORBIT never self-completes (an orbit is not a destination); it disengages
on any flight input (inherited breakout), Z, a vanished well (mirrors
GOTO's vanished target), or capability loss.

HUD v1: the flight-status line grows `MAN ... | GRAV <name>` while coasting
in a well and `AP ORBIT - <ALIGN|BURN|HOLD> | r <radius> | <speed> u/s`
while engaged; the GOTO destination marker doubles for the orbited well;
and an `[O] ORBIT` label projects onto the dominant well while parking is
on offer - the first hand-placed instance of the keybind-hint idea that
task 20260709-103454 will systematize.

## Decisions and deviations

- **Dead engines disengage ORBIT** (like STOP/GOTO), rather than the task
  text's "aligns but cannot burn". The maneuver machine has one uniform
  capability rule - no live engines or no live computer means no autopilot -
  and carving an ORBIT-only exception would special-case the shared
  disengage path for no player-visible gain (either way the ship stops
  correcting and the orbit decays). Recorded in TASK.md as a deviation.
- **The plan lives in the action enum** (`Orbit { well, plan: Option<..> }`)
  instead of a separate component: it is maneuver state exactly like the
  GOTO target, travels with the Autopilot component, and vanishes on
  breakout for free.
- **The well lookup uses avian `Position`, not `GlobalTransform`**, so the
  ring the computer flies is the same frame the gravity force system pulls
  in.
- **Playtest lock-on fix folded in as a prerequisite** (b01a76b): putting
  well sources on rails in the gravity task silently dropped them from the
  targeting system's lockable set (it filtered to `RigidBody::Dynamic`).
  Lockable is now "dynamic, or a gravity-well source" - the ORBIT approach
  flow (lock the rock, GOTO, O) needs it.

## Verification

- 3 pure-helper tests (plane-normal fallback chain, band clamp incl. the
  degenerate tiny-well case, desired-velocity properties on/inside/off the
  ring), 2 status-line tests (GRAV state, ORBIT phases incl. the dead-well
  degradation), 3 HUD anchor tests (cue shows in a well / hides while
  orbiting / clears outside; destination marker follows the orbited well),
  1 targeting regression test, and 3 physics-level tests on the real
  harness + real gravity plugin: engage from near-rest at r = 50 -> plan
  keeps 50, a full ~64s lap stays in [0.8R, 1.25R], Hold is reached, speed
  ends near v_circ, still engaged; well death disengages; dead engines and
  dead controller disengage.
- All 51 flight tests, 14 gravity tests, 25 targeting tests, 7 flight-status
  HUD tests pass; cargo fmt + cargo check --workspace --examples clean
  (the --examples flag is the gravity retro's lesson). Full suite and
  clippy on CI per project policy.

## Difficulties

- None blocking. The main design risk - Hold flickering against the Burn
  boundary - was pre-empted with enter/exit hysteresis copied from the
  align-gate pattern already in the file.

## Self-reflection

- Reading the whole autopilot seam map before touching it (one exploration
  pass) meant the verb needed zero new actuator code; the diff is almost
  entirely goal computation and readouts. The "one rule flies every
  maneuver" architecture paid off exactly as the diegetic-autopilot spike
  hoped.
- The user's playtest reports (SOI too small, lock-on broken) both arrived
  while this task was in flight and both traced to the previous task's
  review fixes. Post-merge playtest feedback is part of the cycle;
  budgeting for a same-day tuning/fix pass after any physics-facing merge
  would have made this smoother.
