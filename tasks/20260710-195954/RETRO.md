# Retro: GOTO parks into ORBIT on arrival

- TASK: 20260710-195954
- BRANCH: feature/goto-parks-into-orbit (squashed to master as 9fb42af)
- REVIEW ROUNDS: 2 (round 1 REQUEST_CHANGES: 1 MAJOR, 1 MINOR, 2 NIT;
  round 2 APPROVE)

## What went well

- The seam was exactly where the task notes predicted (the done branch),
  and the two prior arrival cycles had already put the park point inside
  the stable band - queueing the three arrival tasks in dependency order
  paid off across all of them.
- Landing hygiene held: worktree verified clean before both the review
  handoff and sprout rm (the new AGENTS.md convention, first outing).
- The review-agent continuation pattern (same reviewer re-verifies its
  own findings in round 2 with full context) again produced a precise,
  fast second round.

## What went wrong

- R1.1 (MAJOR): the handoff gated on "the leg completed" (done) and "the
  target is a well", but done means "wants zero velocity at near-rest",
  which the DEGRADED no-stopping-plan state also produces arbitrarily
  far from the body - a state I built myself two cycles earlier and then
  forgot when writing a condition that implicitly assumed desired==ZERO
  implies arrival. Root cause: a semantic overload (ZERO = arrived, ZERO
  = refusing) introduced in one cycle and consumed in another without
  re-enumerating the producers of the consumed signal. This is the
  project's derive-from-the-owning-system rule pointed at one's own
  recent work: the rule's "enumerate every producer of the promised
  state" step applies even when - especially when - you wrote the
  producers yourself.

## What to improve next time

- When gating on a derived condition (done, arrived, settled), list the
  states that can produce it before writing the gate - including
  degraded/edge states added in recent cycles. A one-minute enumeration
  would have caught the apex handoff at implementation time.
- Semantic overloads (one value meaning two things) deserve either a
  distinct signal or a comment at the producer naming every consumer
  assumption; the goto_arrived flag is the distinct-signal fix.

## Action items

- [ ] Playtest the zero-key parking flow at the (now stronger, 6 u/s^2)
  Gravity Rock: G from far out, hands to ORBIT at the park point, Z or
  any input breaks out.
- [x] The one-frame stale-telemetry NIT (R1.4) is recorded in REVIEW.md
  as a deliberate non-fix; no follow-up.
