# Spike: client-side smoothing and deadbands on PD and camera outputs

- STATUS: OPEN
- PRIORITY: 52
- TAGS: v0.5.0,spike,feel

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

- [ ] Inventory what "not pixel perfect" is at the numbers level: trace
      the PD output and hull attitude error during representative manual
      maneuvers (turn, hold, small corrections) - magnitude and frequency
      of the residual micro-motion.
- [ ] Enumerate candidate layers and their risks: (a) deadband/epsilon on
      PD output torque (physics-side - risk of limit cycles and attitude
      droop; the residual-roll history is required reading), (b) slew or
      low-pass on the PD command input (already partly exists via
      slew_rotation), (c) render-clock-only filtering (smooth the eased
      Transform or the camera anchor - zero physics risk), (d) camera
      micro-motion deadband in bcs lerp_and_snap.
- [ ] Prototype the most promising one or two behind a FlightSettings
      knob; measure the trace again.
- [ ] Write docs/spikes/<ts>-feel-filtering.md with the direction and
      seed tatr tasks; explicitly answer what happens to the existing
      snap thresholds and guards (release spin bound, AI settle bound)
      under any deadband.

## Notes

- Filed from user feedback mid-flow (2026-07-11); the user explicitly
  requested the /spike treatment for this one and queued it LAST.
- Context: docs/2026-07-09-flight-feel-retune.md,
  docs/retros/20260709-125640-residual-roll-release.md (PD conditioning
  history - deadbands interact with everything that retro fixed),
  tasks/20260711-121701 (the hull is provably steady; "not pixel perfect"
  is about small commanded corrections, not spurious torque).
