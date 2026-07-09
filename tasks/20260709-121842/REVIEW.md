# Review: Multi-thruster autopilot: per-engine directions, fastest-path group planner

- TASK: 20260709-121842
- BRANCH: feature/multi-thruster-autopilot

## Round 1

- VERDICT: APPROVE

Delivers the spike's three user-settled calls exactly: all aligned engines
fire (per-engine cone with per-engine hysteresis, shared throttle over the
firing set's authority), torque stays unmodeled (documented in code and
design note, follow-up recorded), and the group choice is time-optimal with
the rotation-bias knob (crossover arithmetic done before the tests were
written, so the two bias tests sit on either side of it deterministically).
The GOTO arrival plan correctly derives both deceleration and a dynamic lead
from the scored brake group, replacing the fixed flip lead. Manual mode
semantics unchanged (section-local main-drive check).

Verified by reading: the standoff branch avoids the NaN a zero to-target
would feed angle_between; the firing set is computed before the completion
check that consumes it; engineless ships disengage instead of aligning
forever; and the emergent behavior during a main-drive flip is right - a
retro that already points at the error fires while the hull swings, then the
main takes over. Checks per AGENTS.md: 21 targeted flight tests green (4 new
physics scenarios), fmt clean, cargo check native + wasm32 clean; full suite
and clippy left to PR CI.

- [ ] R1.1 (NIT) crates/nova_gameplay/src/flight.rs (`cluster_thrusters`) -
  greedy clustering is query-iteration-order dependent for engines sitting
  exactly on the cone boundary, so borderline layouts could regroup between
  ticks. Harmless today (groups are recomputed per tick and the score is
  continuous-ish); worth a stable sort or angular-bucket pass only if a
  playtest ever shows flicker on exotic builds.
  - Response: acknowledged, deferred - revisit with the torque-aware
    allocation follow-up, which reworks this scan anyway.
