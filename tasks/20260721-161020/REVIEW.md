# Review: Final Tally (ch3b) - gravity-well finale

- TASK: 20260721-161020
- BRANCH: content/final-tally-ch3b

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

- [x] R1.1 (MINOR) final_tally.rs survey handler - the kill-pickets-first
  path posted a picket objective nothing would ever complete (the taunt's
  complete() no-ops pre-post), marked two dead ships, and spoke "two
  pickets riding the well" over wrecks. Suggested: split the survey into
  fate variants per the lifeline pattern + pin the residue.
  - Response: fixed - two mutually-exclusive survey handlers (Or(a==0,b==0)
    vs both==1); the pickets-down variant skips the objective/marks and
    speaks "its pickets are already drift"; harness extended: the late
    survey asserts no picket objective is left open. Regenerated, 7 green,
    lint still zero findings, probe re-run OK.
- [x] R1.2 (NIT) the kill confirm line hardcoded "The escort is running."
  regardless of kill order.
  - Response: fixed - the line is now kill-order-agnostic ("The claim is
    going dark.").

Verification notes (out-of-context reviewer): act machine hand-traced
(flag-based gates, no deadlock on any ordering, act 4 locks the win and
the Victory is guaranteed over the wreckage - clock/pulse gate on scenario
liveness, not the player); clock-mark AST verified in builder AND generated
RON; all four lifeline chains verified (one lingering NextScenario each,
exclusivity unchanged); lint re-run (0 errors, 13 scenarios, zero
final_tally findings - the berth-move story verified: 952.4u at z=210,
~1071u at z=90, pinned); all 31 tests re-run green; the layout test
verified to genuinely derive from GravitySettings::default().soi_factor
and ASTEROID_GEOMETRIC_FACTOR_MAX; the example's event-level survey judged
a fair fidelity trade (the input-to-lock bridge has its own end-to-end
coverage in loader.rs); probe evidence real and current (20-stage walk,
verdict OK); every DoD grep re-run verbatim; conventions and Record
honesty clean.

Pending manual (batched to flow Finish): finale difficulty peak (flagship
+ escort grade, picket pair), epilogue pacing feel (4s/9s), cast names.
