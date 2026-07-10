# Review: GOTO standoff must be surface-relative

- TASK: 20260710-202408
- BRANCH: fix/surface-relative-standoff

## Round 1

- VERDICT: APPROVE (findings fixed in-round; see responses)

Verified sound by the reviewer: ManeuverTelemetry.distance has exactly one
consumer (the readout chip's text formatting) - the ribbon and flip gate
build from world points and never scale by distance; the inside-standoff
gate stays consistent with the surface-relative publication; the negative
BodyRadius trace saturates at 0.0 through the map_or/max chain and both
use sites clamp; every in-closure standoff use was converted and the
input/ai.rs waypoint check is legitimately untouched (GotoPos, radius 0);
the AI never issues Goto at sized entities; asteroid_scenario_object is
the sole asteroid bundle so every path authors BodyRadius, and fragments
inherit nothing; the -45 park tolerance matches the flat-space tests'
pre-existing terminal-creep bound on master (not a new mask) while the
never-below-surface floor holds over the whole trajectory; the
surface-relative telemetry assertion recomputes; no pre-existing test was
weakened; docs claims ("pure helpers did not change", Known limits) match
the code. Tests re-run by the reviewer: flight 55, hud 55, input::ai 73,
nova_scenario 9.

- [x] R1.1 (MINOR) TASK.md step ticked but not fully delivered: no test
  proves the max(BodyRadius, well.body_radius) preference with the two
  sources disagreeing; GotoPos-unchanged is only implicit. The
  Resolution's "no deviations" overstates.
  - Response: fixed - goto_radius_resolution_prefers_the_larger_source
    spawns a target with BodyRadius(20) + a 40u-body well and asserts the
    telemetry budgets 40; flight is now 56 tests. Resolution amended
    (GotoPos coverage remains the pre-existing goto_pos test, stated
    explicitly).
- [x] R1.2 (MINOR) telemetry.distance can go negative (ship inside the
  resolved radius) and the chip prints e.g. "-12m"; the field doc does
  not mention it.
  - Response: fixed - both publication sites clamp the surface distance
    at zero and the field doc says so.
- [x] R1.3 (NIT) flight.rs q_wells comment - the Without filter's design
  statement predates the GOTO arm's radius read; a ship target with a
  bolted-on well silently resolves radius 0.
  - Response: taken - comment now names the second consumer and the
    inherited "ships stay center-relative" consequence.
- [x] R1.4 (NIT) the holo ribbon still terminates at the target center,
  visually plunging radius+standoff past the park point on big bodies.
  Pre-existing (always overshot by the standoff), out of scope.
  - Response: agreed out of scope - filed as a follow-up tatr task after
    landing (HUD polish, low priority).

Landing note: these round-1 fixes were re-applied on master as a
follow-up commit after the squash - a broken shell chain (`grep -c`
exiting nonzero on zero matches) skipped the branch commit and the
worktree was removed with the fixes uncommitted. Content is identical to
what the responses describe.
