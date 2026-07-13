# AI patrol and idle flight states

- STATUS: CLOSED
- PRIORITY: 70
- TAGS: v0.4.0,ai,spike,handling


Spike: tasks/20260709-225508/SPIKE.md (wave 2)

Goal: make AI ships placeable in scenarios before combat starts. Patrol state:
fly a waypoint loop, reusing the GOTO autopilot / FlightIntent machinery
(flight.rs) where possible instead of a parallel steering path; a
hostile-detection range transitions Patrol -> Engage. Idle state:
station-keeping drift (kill velocity, hold position loosely).

Blocked on: 20260709-155921 (AI rotation path). Depends on:
20260709-225726 (skeleton).

## Steps

- [x] flight.rs: add a position goal to the autopilot -
      `AutopilotAction::GotoPos { position: Vec3 }` - sharing the GOTO
      arrival logic (resolve the goal position, then the same arrival
      curve). HUD destination marker treats it like Stop (player-only UI;
      the player never engages GotoPos).
- [x] input/ai.rs: `AIPatrolRoute { waypoints: Vec<Vec3>, current: usize }`
      component (reflected, registered), with wrap-around advance.
- [x] Behavior transitions: `AI_ENGAGE_RANGE` detection range; passive
      states (Idle/Patrol) engage only when the target is inside it; the
      no-hostile fallback is Patrol with a route, Idle without. Combat
      states hold as before.
- [x] Passive flight system (AI chain, after update_behavior_state):
      engaging states drop any Autopilot; Patrol advances the waypoint on
      arrival and keeps a GotoPos engaged toward the current waypoint;
      Idle engages Stop while drifting (station-keeping) and lets the
      autopilot disengage itself at rest.
- [x] Actuator ownership: the AI thruster system leaves
      ThrusterSectionInput alone while an Autopilot is engaged (the
      rotation system already holds its helm in passive states).
- [x] Scenario plumbing: `AIControllerConfig { patrol: Vec<Vec3> }`;
      insert AIPatrolRoute when non-empty (editor call site updated).
- [x] Tests: pure transition tests (range gate, route fallback); unit
      tests for waypoint engage/advance, idle stop-engage, engage drops
      the autopilot, thrust ownership; GotoPos on the flight_app physics
      harness; physics test that a patrol ship flies its first leg and
      engages when a hostile is inside detection range.
- [x] Verify: cargo fmt + check + the new/touched test modules.
