# Review: Harness-prove ally allegiance + orbit-directive combat guards

- TASK: 20260721-160906
- BRANCH: test/ch3-mechanisms-rig

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

No findings. Tests-only branch; all DoD proofs re-run verbatim and green
(3 ally rigs, spawn-path Player case, T1 verdict grep, empty assets diff).
Reviewer independently confirmed: the acquisition rigs would fail if the
relation model excluded Player-allegiance AI ships (relations.rs:54-62);
the Idle-park makes the Engage pull a real assertion (spawn default is
Engage); the nearest-draw rig is free of hysteresis/attacker discounts
(fresh AITarget None, AIThreat default expired); the two amended-to-citation
rigs are as strong as claimed (orbit-guard tests at ai.rs:2786/2139 cover
the picket configuration exactly; on_destroyed_entity has no allegiance
filter); Record's 95/31 green counts match measurement exactly.

In-session note: the load-bearing mechanism claims (relation matrix,
pick_ai_target discounts, next_behavior_state pull) were each read directly
from source in the implementing session before the rigs were written; the
reviewer's independent derivation agrees on all three.

No open manual: DoD items on this task.
