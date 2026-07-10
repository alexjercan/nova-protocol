# Review: AI patrol and idle flight states

- TASK: 20260709-225730
- BRANCH: feature/ai-patrol-idle

## Round 1

- VERDICT: APPROVE

Spec check against TASK.md: delivered. Patrol flies the AIPatrolRoute loop
through the real GOTO autopilot (new position-goal variant, no parallel
steering path), a detection range (AI_ENGAGE_RANGE) gates Patrol/Idle ->
Engage while combat states keep holding on any acquired target, Idle
station-keeps via STOP burns, and AIControllerConfig.patrol makes routed AI
ships placeable from scenarios. Actuator ownership is clean: the AI thrust
system yields to an engaged autopilot instead of fighting its spooled
inputs, and engaging states drop the autopilot. Verified the test suite for
the touched modules myself: 44 input::ai tests (including the new
first-leg physics test), the GotoPos flight test, and the HUD destination
tests all pass; cargo check is green workspace-wide. Existing transition
tests were extended, not weakened (the new range-gate cases are strictly
stronger). CHANGELOG untouched, consistent with the sibling AI tasks in
this wave (225727/28/29 did not log individually).

Findings:

- [x] R1.1 (MINOR) crates/nova_gameplay/src/input/ai.rs:327 -
  `AIPatrolRoute::current_waypoint` indexes `waypoints.get(self.current)`;
  both fields are pub and reflected, so an inspector edit that shrinks the
  waypoint list below `current` strands Patrol forever: `current_waypoint`
  returns `None`, the system bails before `advance()` can ever wrap the
  index, and the ship never flies again. Self-heal instead: index modulo
  `waypoints.len()` (guarding empty) so a shrunk route snaps back into
  range.
  - Response: fixed - `current_waypoint` now wraps out-of-range indices
    (`current % len`) and `wrapped_current` is covered by a regression test
    (`an_out_of_range_index_self_heals`). `advance` stores the wrapped
    successor so `current` also returns to range on the next leg turn.
- [ ] R1.2 (NIT) crates/nova_gameplay/src/input/ai.rs:449 - a route whose
  legs are shorter than `arrival_standoff + AI_WAYPOINT_SLACK` (75 m at
  defaults) advances `current` every frame and re-engages alternating legs
  until the drift dies (the GotoPos inside-standoff desired velocity is
  zero, so it does converge to the on_station rest). Harmless but churny;
  consider documenting a minimum useful leg length on
  `AIPatrolRoute::waypoints`.
  - Response: documented on the `waypoints` field ("legs shorter than the
    arrival radius ... collapse into station keeping"); no behavior change,
    the convergence is acceptable for a degenerate config.
- [ ] R1.3 (NIT) crates/nova_gameplay/src/input/ai.rs:478 - the Idle arm
  only acts when no autopilot is engaged, so a ship that loses its route
  mid-leg (runtime component removal, editor/debug) keeps flying the stale
  GotoPos to a waypoint it no longer patrols before settling. If that flow
  ever becomes supported, Idle should replace a non-Stop maneuver with
  Stop; not worth code today.
  - Response: agreed, left as documented behavior - runtime route removal
    is not a supported flow yet; noted in the update_passive_flight doc.
