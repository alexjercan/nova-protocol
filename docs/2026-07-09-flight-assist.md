# Flight assist / Newtonian handling

Task: `tasks/20260708-203655/TASK.md`; design decided in
`docs/spikes/20260709-094731-flight-feel-assisted-newtonian.md`. Adds the
ship's flight computer: an assisted-by-default velocity-command layer over the
existing honest thruster simulation, with a Newtonian "FA off" toggle. The
feel-polish half (rotation slew, camera weight, playtest retune) is the
follow-up task 20260709-095043.

## Where it lives

`crates/nova_gameplay/src/flight.rs` (`NovaFlightPlugin`), sibling shape to
`audio.rs`/`juice.rs`. The flight computer only drives ships that carry a
[`FlightIntent`] - inserted for the player by an `Add<PlayerSpaceshipMarker>`
observer - so AI ships (which write `ThrusterSectionInput` and the rotation
input directly) and scenario ships are untouched.

## The model

Intent, not thrust: the player states what they want (`FlightIntent.linear`,
ship-local, `-Z` forward; `brake`), and the computer turns it into forces that
never exceed what the surviving sections can produce.

- **Assisted (default).** A commanded velocity (`FlightCommand`) is nudged by
  intent (`command_accel` per second held), latched to zero by brake, held
  when there is no input - a true Newtonian hold, not drag - and soft-capped
  at `max_commanded_speed`. Each physics tick the computer computes the
  impulse that would close the velocity error (`(command - velocity) * mass`,
  a deadbeat controller that the authority clamps turn into a gradual burn):
  the forward component becomes the main-drive input, everything the engines
  cannot deliver goes to the RCS, clamped by its authority.
- **Newtonian ("FA off", Z).** No servo. Forward intent drives the main
  thrusters directly; laterals and retro are direct RCS burns; momentum
  persists. The command tracks the actual velocity while in this mode, so
  toggling back to assisted never yanks the ship.

### Capability comes from live sections

- **Main drive** = the summed magnitudes of live, forward-aligned thruster
  sections (alignment cosine >= 0.9). Delivered through the real seam: the
  computer writes `ThrusterSectionInput`, so the plume shader, the engine hum,
  and the actual `apply_linear_impulse_at_point` all agree with the burn.
- **RCS** = the live controller section's new `rcs_magnitude`
  (`ControllerSectionRcsMagnitude`, default 0.5 vs the basic thruster's 1.0).
  The flight computer *is* the controller section: destroy it and assisted
  mode, station-keeping, and lateral/retro authority all die together, exactly
  like rotation authority already does. RCS impulses are applied at the
  center of mass (no spurious torque) via the same compute/apply system split
  the PD controller uses (avian's `Forces` conflicts with reading
  `LinearVelocity` in one system); the computed impulse is exposed as
  `FlightRcsImpulse` for future RCS visual/audio cues.
- A thruster carrying a manual `SpaceshipThrusterInputBinding` (the editor
  supports binding keys straight to thrusters) belongs to the pilot: the
  computer neither counts nor drives it.

### Spool

Thruster inputs are written through an exponential ramp (`spool_up_rate` /
`spool_down_rate`; engines cut faster than they light), so the exhaust shader
and audio hum - which read `ThrusterSectionInput` directly - stop snapping
0-to-100. The RCS covers the transient while the engine spools, which is also
what a real flight computer's trim would do.

## Input

A flight input rig (`Input: Flight`, spawned/despawned with the player ship):

- W/S/A/D/E/Q - forward/back, lateral, vertical intent (`Spatial` preset,
  which already outputs `-Z` for forward, matching the ship convention).
- Space / right trigger - full burn (muscle memory from the old binary
  binding); left stick - lateral/vertical trim.
- X / east button - brake (kill velocity; a latch, see below).
- Z / north button - flight-assist toggle.

The old per-section `thruster -> Space` entry in the default scenario's
`input_mapping` is gone; the per-section binding mechanism itself remains for
ships without a flight computer.

## Semantics worth knowing

- **Brake means two things.** In assisted mode it is a latch: tapping X
  commands zero velocity and the ship keeps decelerating until it gets there;
  any direction input re-takes the command (chosen over hold-to-brake so
  "come to a full stop" does not require pinning a key through a long burn).
  In Newtonian mode there is no servo to command, so X is a plain full retro
  RCS burn while held - it will burn straight past zero if you let it, which
  is the FA-off contract.
- **Assisted mode station-keeps.** A torpedo blast that shoves the ship is
  counter-burned back to the commanded vector automatically (up to RCS/drive
  authority). That is the flight computer doing its job; pilots who want to
  tumble with the hit fly FA-off.
- **The soft cap is on the *command*, not physics.** The computer refuses to
  command past `max_commanded_speed` (30 u/s: above the AI's 20 u/s chase
  ceiling, below the torpedo's 35 u/s); Newtonian mode is uncapped and
  nothing anywhere applies drag.

## HUD

`hud/flight_status.rs`: one text line, lower-left - `FA ON 12.3 -> 20.0 u/s` /
`FA OFF 12.3 u/s` - so the toggle and the command are legible. Formatting is a
pure helper (`flight_status_line`) shared with the flight module and
unit-tested. Real HUD work stays with the weapons-HUD tasks.

## Testing

Pure helpers (spool, intent split, main/RCS impulse split, command nudge/cap,
alignment, HUD line) are unit-tested. The integration tests reuse the
integrity physics harness (`integrity/test_support`, now `pub(crate)`) with
the real `thruster_impulse_system` (made `pub(crate)` for this), covering the
whole pipeline - intent -> command -> spooled input -> impulse -> velocity:
brake kills velocity, assist station-keeps after an external shove, Newtonian
coasts exactly, a full burn accelerates and dies with its thruster section,
no controller means no station-keeping, and the commanded cap holds over a
long burn.

## Deliberately deferred

- Rotation slew limit, camera smoothing/burn push, playtest retune - task
  20260709-095043.
- Match-target-velocity (zero relative velocity to the locked target), a
  thrust-allocation solver + multi-thruster blueprints, RCS visual cues
  (wasm particle block 162908 applies), AI on the intent API - recorded in
  the spike's Next steps.
