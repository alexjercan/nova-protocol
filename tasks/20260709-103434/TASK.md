# Diegetic autopilot: STOP + GOTO flown through the real actuators

- STATUS: CLOSED
- PRIORITY: 85
- TAGS: v0.4.0, handling, autopilot, spike

Spike: docs/spikes/20260709-103324-diegetic-autopilot.md (design calls settled
with the user; supersedes the velocity-servo model from
docs/spikes/20260709-094731-flight-feel-assisted-newtonian.md)

## Goal

Rework the flight layer (52b582d) into a diegetic autopilot: the computer
flies the ship with the same actuators the pilot has - PD torque from the
controller section to swing the nose, spooled main thrusters to burn - and no
invisible forces anywhere. `X` engages STOP (flip retrograde, burn to zero);
`G` engages GOTO on the current aim-assist lock (accelerate, flip at the
arrival curve, decelerate, stop at a standoff outside blast radius). While
engaged, the mouse is camera-only free-look; any flight input disengages and
hands the ship back without a lurch.

## Steps

- [x] Strip the servo from `crates/nova_gameplay/src/flight.rs`: remove
      `FlightCommand`, `FlightRcsImpulse` + `apply_flight_rcs`, the
      velocity-hold branch, the RCS split, and `FlightAssistMode`
      (Assisted/Newtonian); remove `ControllerSectionRcsMagnitude` (component,
      config field, `nova_assets/sections.rs` + torpedo entries) and the
      ADQE/strafe + brake-latch input pieces. Keep: spool, capability scan
      (main authority), analog W/Space forward burn, HUD file, insert
      observer, test harness reuse.
- [x] Autopilot core in `flight.rs`: `Autopilot` component on the ship root
      (`action: Stop | Goto { target: Entity }`, `phase: Align | Burn |
      Done`), pure helpers for the arrival rule
      (`v_allowed = sqrt(2 * a * margin * d)`), desired-facing selection
      (retrograde for STOP; toward target or retrograde per the curve for
      GOTO), alignment gate (dot >= ~0.95, like the AI), and phase
      transitions. FixedUpdate system writes
      `ControllerSectionRotationInput` + spooled `ThrusterSectionInput` from
      live capability + `ComputedMass`; STOP completes at ~zero speed, GOTO
      completes stopped within the standoff (~50u, a `FlightSettings`
      tunable, > the 30u torpedo blast radius). Controller section dead =
      cannot engage / auto-disengages.
- [x] Input authority handover: gate the manual rotation copy
      (`update_controller_target_rotation_torque`, `Without<Autopilot>`) and
      the manual burn system off while engaged. (Plan simplification: no
      camera-mode machinery needed at all - the mouse keeps driving the
      camera rig, the hull just stops listening, which IS camera-only
      free-look.) On `Remove<Autopilot>` the normal rig's `PointRotation` is
      re-seeded from the hull's current attitude
      (`camera_controller::on_autopilot_disengaged`) so nothing snaps.
- [x] Engage/disengage input in `input/player.rs`: `X` -> STOP (toggles off;
      overrides a GOTO - braking wins), `G` -> GOTO current lock (toggles
      off; no lock = no-op logged, the MAN status line is the v1 hint), `Z`
      -> plain off, manual burn while engaged -> disengage. Flight rig
      bindings reduced to W/Space/right-trigger analog burn + X/G/Z.
- [x] Minimal readouts: extend `hud/flight_status.rs` to `MAN 12.3 u/s` /
      `AP STOP - ALIGN` / `AP GOTO - BURN 320m` (pure formatting helper), and
      a projected destination marker on the GOTO target (reuse the
      torpedo-target HUD projection pattern).
- [x] Tests: pure (arrival rule monotonic + zero at zero distance,
      desired-facing per phase, alignment gate, phase transitions, status
      formatting); physics-level via the shared harness (STOP from coasting:
      ship flips and speed reaches ~0 with no external force; GOTO arrives
      stopped within standoff and disengages; flight input mid-maneuver
      disengages and restores manual authority; dead controller section =
      autopilot refuses/disengages).
- [x] Verify: fmt, clippy --all-targets, cargo test --workspace, wasm32
      check. Shared CARGO_TARGET_DIR, heavy builds in background.
- [x] Rewrite `docs/2026-07-09-flight-assist.md` as the autopilot design note
      (or replace with `docs/retros/20260709-diegetic-autopilot.md` and update
      references), documenting the maneuver machine, authority handover, and
      what was removed and why.

## Notes

- Relevant: `input/ai.rs:54` (`ai_desired_direction` - the crude version of
  this brain; keep AI untouched, share pure helpers for later adoption),
  `camera_controller.rs` (`SpaceshipCameraControlMode` + rig switching +
  `initial_rotation` handoff), `input/player.rs`
  (`update_controller_target_rotation_torque` to gate),
  `hud/torpedo_target.rs` (projection pattern for the destination marker).
- GOTO replans toward the target's current position every tick: slow drift
  handled, fast targets just take longer, no collision avoidance (known v1
  limitation, recorded in the spike).
- Deceleration margin ~0.85 absorbs spool lag + PD settling; both it and the
  standoff live in `FlightSettings`.
- Depends on: nothing open (52b582d is on master). Blocks: 20260709-095043
  (re-scoped feel polish + retune).

## Close record (2026-07-09)

What changed: flight.rs rebuilt around the maneuver autopilot (servo,
FlightCommand/FlightRcsImpulse/FlightAssistMode, RCS-at-COM and rcs_magnitude
all removed; spool/capability/insert-observer kept), Autopilot component +
autopilot_system (one rule: face the velocity error, burn when aligned),
X/G/Z engagement observers, Without<Autopilot> gates on manual rotation copy
and manual burn, disengage re-seed observer in camera_controller, HUD status
line + projected cyan destination marker, design note
docs/retros/20260709-diegetic-autopilot.md (flight-assist note deleted). 12 flight
tests + 1 camera re-seed test.

Two flight-dynamics bugs found BY the physics tests, not review:

1. The naive arrival curve (sqrt(2 a m d)) assumes instant retro thrust; the
   ship entered the standoff at 30+ u/s because the 180 flip costs ~1.5s of
   un-braked travel. Fixed with a reaction budget: v solves
   v*flip_lead_time + v^2/(2 a m) = remaining. Diagnosed by instrumenting the
   failing test with a position/velocity/phase trace.
2. Completion released the ship while the engine was still spooling down; the
   dying burn pushed it ~2 u/s off station (and the pure curve also stalls
   asymptotically at the boundary). Fixed with min_approach_speed floor +
   "engines wound down" settle condition before disengage.

Also: the planned camera-mode machinery was unnecessary - gating the manual
rotation copy off makes the mouse camera-only automatically; only the
disengage re-seed was needed. The plan step was updated to match.

Self-reflection: writing the physics-level tests before trusting the math
paid for itself twice in one afternoon - both bugs are exactly the kind that
would otherwise surface as "the autopilot feels drunk" in playtest with no
trail. The debug-print-then-remove trace pattern (same as the juice review's
scratch test) is becoming the house method for physics bugs; worth keeping.
All feel constants (margin 0.85, lead 1.5s, floor 1.5, standoff 50) are
reasoned but unflown - the retune task 20260709-095043 owns them.
