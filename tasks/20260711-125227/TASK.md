# Spike: client-side smoothing and deadbands on PD and camera outputs

- STATUS: CLOSED
- PRIORITY: 52
- TAGS: v0.5.0, spike, feel

## Goal

Playtest (user, 2026-07-11): "the spaceship turning is not pixel perfect,
but that might be related to the PD solver we have, so it makes it
'realistic' in a way that it's true to what we do to solve it; would it be
possible to have some clamping done or smoothing on top of the
calculations we get from the PD controller or from the camera to somehow
make it feel better client-side, like ignoring small numbers, stuff like
that".

This is a /spike, not an implementation task: explore the design space of
feel-layer filtering (deadbands on small PD torques, output smoothing,
camera micro-motion rejection, quantization/hysteresis on displayed
values), weigh the tradeoffs (a deadband on the PD is a control-theory
change that can introduce limit cycles or steady-state error; smoothing
the CAMERA or the RENDERED pose is client-side and safe but can read as
lag), land on a direction, and seed implementation tasks.

USER-SPECIFIED QUEUE POSITION: run this spike only after the currently
queued tasks are done (camera jumps 20260711-125225, zoom cap
20260711-121711, redundant speed 20260711-125226, bullet pop
20260711-121839, torpedo clock 20260711-114640).

## Steps

- [x] Inventory what "not pixel perfect" is at the numbers level: traced
      a full GotoPos leg on the shipped rig (goto_wobble_diagnostic,
      flight.rs, kept `#[ignore]`d for the fix tasks): the wobble is a
      TERMINAL ATTITUDE HUNT - the doorstep-brake residual (0.45-0.6 u/s)
      sits just above attitude_deadband (0.4), so the computer re-aims
      the hull at 0.4-0.6 rad/s (~25 deg/s) for 3-5 s at the end of every
      GOTO leg. STOP is immune (monotone error, nose on error). Bonus
      find: the command copy into PDControllerInput runs in Update while
      producer and consumer tick in FixedUpdate - the PD fights up to
      0.22 rad of phantom command error during slews (mean torque +22%),
      and the test harness does NOT reproduce this (it wires the copy in
      FixedUpdate - production diverges from the rig).
- [x] Enumerate candidate layers and their risks: done in the spike doc -
      (a) PD-output deadband REJECTED (wrong layer: the torque executes a
      hunting command; residual-roll minefield), (b) command slew already
      exists, (c) render filtering not needed (the hull physically
      moves), (d) camera deadband DEFERRED (camera reported clean after
      5ba0e3c + eced738).
- [x] Prototype the most promising: settle-deadband raise measured at
      0.6 and 0.75 (terminal rms 0.38 -> 0.12 -> 0.03 rad/s, release
      spin 0.44 -> 0.09 -> 0.05, lateral tracking unchanged), and the
      same-tick handoff A/B (phantom error 0.22 -> 0.003 rad).
- [x] Write docs/spikes/20260711-140234-feel-filtering.md and seed tatr
      tasks 20260711-140234 (settle deadband, P58) + 20260711-140241
      (handoff clock fix, P56). Guards under the wider band answered in
      the doc: release-spin guard GAINS margin, AI settle bound
      untouched, ORBIT hold gates keep their 2x ratio via scoping, STOP
      release unchanged by the aligned-authority argument.

## Notes

- Filed from user feedback mid-flow (2026-07-11); the user explicitly
  requested the /spike treatment for this one and queued it LAST.
- QUEUE NOTE: run order changed by the user (2026-07-11, follow-up
  session): the spike ran FIRST to diagnose the GOTO wobble ("try to
  identify the wobble issue and fix it"); bullet pop 20260711-121839 and
  torpedo clock 20260711-114640 proceed via /flow alongside the two
  tasks this spike seeded.
- Context: docs/2026-07-09-flight-feel-retune.md,
  docs/retros/20260709-125640-residual-roll-release.md (PD conditioning
  history - deadbands interact with everything that retro fixed),
  tasks/20260711-121701 (the hull is provably steady; "not pixel perfect"
  is about small commanded corrections, not spurious torque).

## Resolution

Spike complete: docs/spikes/20260711-140234-feel-filtering.md. The GOTO
wobble is a terminal attitude hunt caused by the settle deadband being
tuned below the doorstep-brake residual; secondary find, the rotation
command handoff crosses clocks (Update copy between two FixedUpdate
ends) and the flight test harness silently patches it. Seeded
tasks/20260711-140234 (P58) and tasks/20260711-140241 (P56); the
`#[ignore]`d goto_wobble_diagnostic in flight.rs carries the A/B rig for
both. Camera-side filtering deferred: no measured camera residual
remains on file after 5ba0e3c + eced738.
