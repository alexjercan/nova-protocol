# Review: AI threat-scored target selection over the relation model

- TASK: 20260709-225727
- BRANCH: feature/ai-target-selection

## Round 1

- VERDICT: APPROVE

Reviewed `git diff master...feature/ai-target-selection` against TASK.md;
full nova_gameplay suite on the branch: 208/208 green. The acquisition
pipeline is correct and well-factored: the pure picker uses the target
kind as an Ord tier in a lexicographic min (no tunable weights to drift),
the range gate and hysteresis discount are constants with clear docs, and
the acquisition system's candidate filter handles the traps - self-
exclusion, relation-model hostility (a fellow AI is Own, an asteroid is
None -> Neutral), the committed-torpedo gate mirroring the player rule,
and turret bullets excluded by kind (they carry an Allegiance since
203708 but are neither ship roots nor torpedoes; verified in the filter
logic). Both ends of every vector stay on live-structure anchors via the
shared ai_target_anchor helper. The consumer swap removes the player
Single from all five systems, and the existing tests were upgraded to
drive the real acquisition pipeline rather than hand-set targets - the
physics test now runs the full acquire -> transition -> rotate chain.
Dead-target clearing is tested (despawn -> None on next pick). The
deferred threat-memory scoring is recorded honestly in Notes with its
blocker (no damage attribution exists) and its landing spot (225731).
No findings.
