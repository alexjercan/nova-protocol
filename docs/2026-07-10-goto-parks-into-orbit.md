# GOTO at a gravity-well body parks into ORBIT

Task: tasks/20260710-195954 - User request: a GOTO at a big object with a
gravity well should end in a parked orbit, not a dead stop that
immediately starts falling.

## The rule

One branch in `autopilot_system`'s done path (crates/nova_gameplay/src/
flight.rs): when the completed action is `Goto { target }` and the target
is a gravity well (`q_wells.contains`), the autopilot is replaced in
place with `Autopilot::engage(AutopilotAction::Orbit { well: target,
plan: None })` instead of being removed - gated on the ship actually
being INSIDE the park envelope (the surface-relative telemetry distance
at or under the standoff), because "wants zero velocity" alone is not
arrival: the degraded no-stopping-plan state (well pull at or above the
brake authority) also zeroes the desired velocity arbitrarily far out,
and a done-at-apex there releases exactly as it always did rather than
parking into an orbit whose ring correction assumes it starts near the
ring. `engage()` resets the phase to Align; the existing ORBIT plan
block picks the ring from the arrival radius on the next tick - the
surface-relative park point (standoff + body_radius) lands inside the
stable band, which the plan clamps into regardless. The one-key parking
flow becomes zero-key when the computer was already told where to go.

Everything else is deliberately untouched:

- GotoPos, well-less/unsized targets and STOP release exactly as before
  (the sized-target arrival test is the contrast case).
- Breakout semantics are ORBIT's own: any flight input or Z disengages,
  no matter how the orbit was entered.
- Telemetry needed no code: the Orbit arm publishes no ManeuverTelemetry,
  so the pre-existing clear branch drops the GOTO numbers on the first
  orbit tick, and the HUD's AP ORBIT states, ring and cue retirement all
  follow from the action switch.
- `q_wells`'s `Without<SpaceshipRootMarker>` filter carries its design
  statement into the handoff: a ship target never parks you into orbit
  around it.
- No settings toggle, per the task note: default on, revisit only if
  playtests dislike the automatism.

## Verification

The well integration test now asserts the full arc: GOTO from outside
the SOI at speed, never below the surface, handoff to Orbit at near-rest
at the surface-relative park point, then 1200 more frames of engaged
station-keeping above the surface, ending on (within 15u of) the planned
ring. Checks: flight 56,
input::ai 73, hud 55, `cargo check --workspace --examples` clean; the AI
uses GotoPos/Stop only, so no AI behavior changes.
