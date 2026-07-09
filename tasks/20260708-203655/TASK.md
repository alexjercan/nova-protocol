# Spaceship handling / Newtonian flight-feel overhaul

- STATUS: CLOSED
- PRIORITY: 85
- TAGS: v0.4.0, handling, juice

Spike: docs/spikes/20260709-094731-flight-feel-assisted-newtonian.md (design)
Roadmap: docs/spikes/20260708-203517-roadmap-reprioritization-and-juice.md

## Goal

Make flying the capital ship weighty, precise, and readable without faking the
physics: an assisted-by-default flight-control layer (velocity-command with
hold, explicit brake, soft commanded-speed cap) over the existing honest
thruster simulation, with a Newtonian "FA off" toggle that drops back to raw
thrust. Design calls are settled in the spike - assisted default, no uninvited
auto-brake (no-input = hold velocity), 6DOF intent, capability model instead of
a thrust-allocation solver, forces clamped by live sections so destruction
still bites.

## Steps

- [x] New module `crates/nova_gameplay/src/flight.rs` (sibling shape to
      `juice.rs`): `FlightAssistMode` component (Assisted default | Newtonian)
      and `FlightIntent` component (`linear: Vec3` per-axis -1..1 in ship-local
      frame, `brake: bool`) on the ship root; reflected `FlightSettings`
      resource (accel gain, soft `max_commanded_speed`, spool up/down rates,
      RCS default authority) - register the whole tree (juice retro R1.1).
      Wire `NovaFlightPlugin` into `plugin.rs`.
- [x] Capability model: add an `rcs_magnitude` field to
      `ControllerSectionConfig` (crates/nova_gameplay/src/sections/
      controller_section.rs, defaults in crates/nova_assets/src/sections.rs);
      pure helper deriving `FlightCapability { main: f32, rcs: f32 }` from the
      ship's live sections (main = sum of forward-aligned live thruster
      magnitudes; rcs = live controller's rcs_magnitude; dead sections
      contribute nothing).
- [x] FCS system in `FixedUpdate` (ordered with `SpaceshipSectionSystems`,
      before `thruster_impulse_system`): assisted mode holds commanded
      velocity (no input = hold, input nudges commanded vector in ship frame,
      brake commands zero, commanded speed soft-capped), computes the velocity
      error -> desired force, splits it into a main-drive component (project
      onto ship forward, delivered by writing the live thrusters'
      `ThrusterSectionInput` so plume/audio/impulse all ride the real seam)
      and an RCS remainder (applied at the COM via avian `Forces`, clamped by
      rcs capability). Newtonian mode: no FCS force, thrust input drives
      `ThrusterSectionInput` directly (today's behavior).
- [x] Spool: `ThrusterSectionInput` is written through a ramp helper
      (exponential toward target, up/down rates from `FlightSettings`) so the
      exhaust shader and audio hum stop snapping 0-to-100.
- [x] Input rework in `crates/nova_gameplay/src/input/player.rs`: translation
      intent axes (W/S forward/back, A/D lateral, Q/E vertical - see Notes),
      brake (X / gamepad B), assist toggle (Z / gamepad Y) writing
      `FlightIntent`/`FlightAssistMode`; keep Space bound as full-forward for
      muscle memory and keep `SpaceshipThrusterInputBinding` working as the
      Newtonian direct-burn binding. Update the player `input_mapping` in
      `crates/nova_assets/src/scenario.rs`.
- [x] Minimal HUD readout in `crates/nova_gameplay/src/hud/`: one text line
      with assist mode + actual speed + commanded speed, in `NovaHudSystems`
      (real HUD work stays with the weapons-HUD tasks).
- [x] Tests: pure helpers (capability derivation incl. dead sections, velocity
      error -> clamped force split, spool ramp, soft cap); App-level
      integration via `integrity/test_support::integrity_physics_app` - brake
      nulls velocity over settled frames, no-input holds velocity under an
      external nudge, Newtonian coasts unchanged, destroyed thruster removes
      main authority; observer-level tests for the input->intent wiring
      (event-driven modules get App tests from day one - audio/juice lesson).
- [x] Verify: fmt, clippy --all-targets, cargo test --workspace (includes the
      examples smoke test), cargo check --target wasm32-unknown-unknown.
      Shared CARGO_TARGET_DIR, heavy builds in background.
- [x] Design note `docs/2026-07-09-flight-assist.md`: the assisted/Newtonian
      model, capability rules, what was deliberately deferred (allocation
      solver, match-target-velocity, RCS visuals, AI on the intent API).

## Notes

- Relevant files: crates/nova_gameplay/src/sections/thruster_section.rs:129
  (impulse at point - keep), controller_section.rs:30 (PD config),
  input/player.rs:311 (binary input to replace), input/ai.rs (AI writes the
  raw seams - must keep working untouched), crates/nova_assets/src/
  scenario.rs:84 (player bindings), hud/velocity.rs (existing velocity HUD).
- bcs has no linear FCS (checked) - the pure core goes in a math layer for a
  later promotion split.
- Assumption (cheap to change): ADQE strafe keys, X brake, Z toggle. Flag at
  review if the user wants different bindings.
- The soft cap applies only to assisted *commanded* speed; Newtonian is
  uncapped; no drag anywhere.
- v1 Newtonian = main-drive only (blueprints have one thruster); retro/lateral
  thruster groups arrive with multi-thruster blueprints (deferred).
- Rotation slew limit, camera smoothing/burn-push, and the playtest retune are
  the follow-up task (20260709-095043) so this branch stays reviewable.

## Close record (2026-07-09)

What changed: new `crates/nova_gameplay/src/flight.rs` (FCS: assisted
velocity-command with hold/brake-latch/soft cap, Newtonian direct mode, spool
ramp, capability from live sections), `ControllerSectionRcsMagnitude` on the
controller section (default 0.5; torpedo warhead explicitly 0.0), flight input
rig (WASDQE intent, Space/right-trigger full burn, X brake, Z toggle, left
stick trim), player scenario mapping dropped the per-section thruster binding,
`hud/flight_status.rs` one-line readout, design note
`docs/2026-07-09-flight-assist.md`. 15 new tests (9 pure + 6 physics-level
integration through the real thruster impulse system).

Alternatives considered are in the spike; two implementation calls made here:
(1) avian's `Forces` query writes `LinearVelocity`, so the FCS uses the same
compute/apply split as the PD controller (`FlightRcsImpulse` component between
them) instead of one system; (2) thrusters carrying a manual
`SpaceshipThrusterInputBinding` (editor feature) are excluded from FCS
authority and drive, so the computer never fights a hand-bound engine.

Difficulties: none blocking - the query-conflict was anticipated from reading
`Forces`' fields before writing code, and the whole suite passed first run.
The subtle part was spool compensation: the RCS covers what the still-spooling
main drive has not delivered yet, which required reading back the post-spool
inputs in the same tick.

Self-reflection: reading the sibling modules (audio/juice) and bcs sources
before writing anything made this a one-pass implementation; the physics-level
tests via the shared integrity harness (made `pub(crate)` for reuse) were
cheap and caught nothing only because the pure helpers were written first.
Feel constants (command_accel 15, cap 30, spool 6/10, rcs 0.5) are reasoned
guesses pending the playtest retune task 20260709-095043 - do not trust them
until a human has flown with them.
