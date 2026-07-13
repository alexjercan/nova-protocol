# Flight feel polish: rotation slew, handling stats, camera weight, retune

- STATUS: CLOSED
- PRIORITY: 80
- TAGS: v0.4.0,handling,juice

Spike: docs/spikes/20260709-094731-flight-feel-assisted-newtonian.md (design
calls 4 and 5); re-scoped for the diegetic-autopilot rework
(docs/spikes/20260709-103324-diegetic-autopilot.md) which replaced the
velocity-servo layer.

## Goal

Make the hull's weight legible on top of the autopilot-era flight layer: the
manually commanded rotation slews instead of teleporting, the PD constants
become tuned handling stats instead of buried defaults, and the chase camera
conveys acceleration. Ends with a playtest retune of every feel constant from
the flight cycle - spool rates, autopilot margin/standoff/alignment gate, PD
constants, slew rate, camera smoothing/push (reused tuning constants are
decisions, not defaults - juice retro).

## Steps

- [x] Slew-limit the commanded rotation: LANDED early via the flip-wobble
      fix (e28b9ed) - both the mouse path and the autopilot slew the
      commanded quat at `FlightSettings::est_turn_rate_deg` through
      `flight::slew_rotation` (pure, unit-tested). What remains here is only
      tuning the rate.
- [x] Surface handling stats. Decision: authority stays diegetic on the
      controller section (PD frequency/damping/max_torque are hardware,
      documented in nova_assets/sections.rs with the retune rationale); the
      flight-computer knobs live in the reflected `FlightSettings`
      (turn_rate_scale + min/max clamps replacing est_turn_rate_deg). Camera
      feel constants (CAMERA_SMOOTHING, BURN_PUSH_DISTANCE) are documented
      consts in camera_controller.rs.
- [x] Torque budget: `est_turn_rate_deg` replaced by
      `flight::hull_turn_rate` - `scale * sqrt(pi * max_torque / inertia) / 2`
      clamped to 10..240 deg/s, fed by the strongest live flight computer's
      torque and the body's live max principal inertia. Both the mouse slew
      and the autopilot planner consume it, so the fixed-rate mask is gone.
      Ship controller max_torque retuned 100 -> 40 (corrected in review
      round 1: 10 halved the shipped flagship's rate unacknowledged): the
      asteroid_field flagship keeps its familiar ~88 deg/s command rate,
      stripped remnants pin the 240 deg/s ceiling; flip-time optima and the
      off-center-thrust tradeoff are tabled in the retune doc.
- [x] Camera weight: `CAMERA_SMOOTHING = 0.15` applied by the mode switch
      (mutating in place per the anchor fix), and `update_camera_burn_push`
      composes `offset = mode_camera_rig(mode) + hottest forward thruster's
      spooled input * 3.0` along anchor -Z, chained after the mode switch so
      the two writers cannot fight. The per-mode rigs moved into
      `mode_camera_rig` as the single source of truth.
- [x] App-level tests: `a_camera_flip_reaches_the_command_over_many_frames`
      (input/player.rs - moves but nowhere near target after one frame),
      `burn_push_leans_back_and_returns_to_baseline` + a smoothing assertion
      in the mode-switch test (camera_controller.rs), and
      `hull_turn_rate_makes_mass_legible` (flight.rs - stripped out-turns
      full, clamps, degenerate inputs).
- [x] Playtest handoff prepared: all knobs and their reasoning tabled in
      docs/retros/20260709-flight-feel-retune.md; the live session with the user
      happens on master (checklist: flip a full vs stripped ship, watch the
      camera lean under burn, shake/flash overlap under burn, deadband
      twitch). Feedback lands as direct retunes of the documented constants.
- [x] Verify (repo policy: local test/clippy deferred to CI - see
      20260709-140816): fmt green, cargo check --workspace green, all 110
      nova_gameplay unit tests run locally and pass (including the 23 flight
      physics tests, unchanged), headless smokes 11_com_range +
      06_torpedo_range green. clippy / full workspace suite / wasm not run
      locally.

## Resolution

The hull's weight is now legible in flight. Rotation: the fixed 90 deg/s
command slew (which re-normalized every ship's turn feel) is replaced by a
live derivation from the strongest flight computer's torque budget and the
body's max principal inertia, consumed by both the mouse path and the
autopilot planner; ship controller max_torque retuned 100 -> 40 so the budget
binds without regressing the flagship's familiar rate (~88 deg/s; remnants
pin the 240 ceiling). Camera: gameplay modes get
smoothing 0.15 and a burn push-back driven by the spooled main drive, with
the per-mode rigs extracted to one source of truth. Values and reasoning in
docs/retros/20260709-flight-feel-retune.md; the live playtest with the user
happens post-merge and adjusts the documented constants directly.

Honest scope: 110 nova_gameplay unit tests + both smokes run locally and
green; fmt + cargo check --workspace green; clippy, the full workspace suite
and the wasm check were not run locally (AGENTS.md policy; CI story tracked
in 20260709-140816).

Reflection: the est_turn_rate_deg knob's own doc comment ("a knob rather
than a derivation ... recorded for the retune") was the design brief - the
codebase had already written down what this task needed to do. The 23
existing flight physics tests surviving a changed turn-rate model unchanged
is the outcome-invariant assertion style working exactly as intended.

## Notes

- Depends on: 20260709-103434 (diegetic autopilot rework; provides
  FlightSettings + spooled thruster input this task reads). The slew limiter
  must only shape the MANUAL rotation command path - the autopilot writes
  `ControllerSectionRotationInput` directly and plans its own turns.
- Relevant: bcs chase.rs:86 (smoothing field exists, unused), bcs shake.rs
  (juice already owns trauma - do not double-feed from thrust here).
- Camera offsets are currently hard-set per mode in
  `sync_spaceship_control_mode`; the burn push must compose with mode
  switches, not fight them (additive on top of the mode's base offset).
