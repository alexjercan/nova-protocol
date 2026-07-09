# Review: AI torpedo threat response - point-defense turret priority

- TASK: 20260709-225733
- BRANCH: feature/ai-point-defense

## Round 1

- VERDICT: APPROVE

Reviewed `git diff master...feature/ai-point-defense` against TASK.md
(including the recorded user decision); full nova_gameplay suite on the
branch: 218/218 green. The design split is right: the override lives at
the GUN layer (aim + fire + velocity feed resolve one shared gun_target)
while flight and the behavior transition keep the ship-first primary
target - pinned by the guns-defend-while-hull-chases test. The three
judgment calls are all defensible and documented at the decision site:
point defense applies in every behavior state (an idle ship defends
itself - tested), the burst cadence is bypassed only while defending
(bursts are anti-ship discipline), and the 400 m defense range sits
inside the 450 m turret envelope with the reasoning in the constant's
doc. The picker reuses the tier-then-distance lexicographic pattern from
225727 (hunting-me before hunting-others) rather than inventing a new
scoring scheme. The range and aim-point discipline gates still apply to
defensive fire, so PDC shots remain honest. The deferred break-off burn
half is recorded with its landing task. No findings.
