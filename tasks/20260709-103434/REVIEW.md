# Review: Diegetic autopilot: STOP + GOTO flown through the real actuators

- TASK: 20260709-103434
- BRANCH: feature/diegetic-autopilot

## Round 1

- VERDICT: APPROVE

Delivers the spike exactly: the autopilot flies with the pilot's own
actuators (PD rotation command + spooled thruster inputs), no invisible
forces anywhere in the diff, engage/disengage semantics match the four
user-settled design calls, and the servo-era surface (FlightAssistMode,
FlightCommand, FlightRcsImpulse, rcs_magnitude, strafe keys) is genuinely
gone, not stubbed. Checks re-run at branch tip: fmt clean, clippy clean,
cargo test --workspace green (89 nova_gameplay tests incl. 12 flight + the
camera re-seed test, examples smoke), wasm32 check green.

The standout: the physics-level tests did the reviewer's hardest job before
review - the flip-time reaction budget and the settle-before-release rule
both exist because a failing test trace demanded them, and both are exactly
the class of bug ("autopilot feels drunk") that a diff-reading review would
have missed. The instrumentation trace is recorded in the close record.

Verified by reading, not trusting: the `Without<Autopilot>` gates cover both
manual seams (rotation copy, burn system); the AI writes raw seams and owns
no `FlightIntent`/`Autopilot`, so it is untouched; a dead controller section
drops the autopilot the same way it already drops rotation authority; the
HUD systems skip cleanly with no camera; editor-bound thrusters stay
excluded from autopilot authority and drive.

No BLOCKER or MAJOR findings. The NITs below are recorded, not required:

- [ ] R1.1 (NIT) crates/nova_gameplay/src/flight.rs (`autopilot_system`,
  settle branch) - while the maneuver is complete but the engines are still
  winding down, `phase` reads `Align`, so the HUD says "ALIGN" for the last
  half-second of a maneuver. A third phase (`Settle`) would be more truthful.
  Cosmetic; the diegetic-instruments task will redesign this readout anyway.
  - Response: acknowledged, deferred to 20260709-103454 (the readout gets
    redesigned there; adding a phase now would be churn).
- [ ] R1.2 (NIT) history - the deleted `docs/2026-07-09-flight-assist.md` is
  still referenced by the (historical) retro and REVIEW.md of task
  20260708-203655. Those records describe the superseded cycle accurately as
  of their writing; leaving them untouched is the repo's convention for
  history, and the new design note names its predecessor. No action.
  - Response: agreed, history stays immutable.
- [ ] R1.3 (NIT) input semantics - pressing X while holding W engages STOP
  for one frame before the held burn's Fire event disengages it, so "X does
  nothing while flooring the throttle". This is the letter of "any flight
  input disengages" and the design note's input table implies it; if
  playtest finds it surprising, the retune task can debounce (e.g. burn only
  disengages on a fresh press, not a held one).
  - Response: acknowledged; flagged for the 20260709-095043 playtest.
