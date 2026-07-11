# Retro: Crosshair on the same-frame intercept

- TASK: 20260710-231929
- BRANCH: fix/crosshair-same-frame-aim (squash-merged as 56f52ae)
- REVIEW ROUNDS: 1 (APPROVE, no findings)

Final member of the twitching family's original four; details in the task
file and the spike doc's fix record.

## What went well

- **The prior cycle's map made this one cheap.** The 231928 cycle had
  already established the PostUpdate terrain (layout before propagation,
  TransformHelper for fresh poses, the pins); this cycle's plan step
  survived contact with the code for the first time in the family, and
  the whole cycle was one review round with no findings.
- **The regression exercises production wiring.** Registering the real
  TurretLeadPlugin (instead of hand-wiring its systems in the test) means
  the A/B sabotage hit the actual registration - the drift-risk that a
  hand-wired schedule test silently stops guarding was avoided by
  design this time.
- **A user hypothesis got a durable answer.** "The target calculation
  takes time" is falsified in a test that will keep falsifying it
  (monotone intercept guard), not just in a conversation.

## What went wrong

- Nothing that cost time. Two small deref/type compile stumbles in the
  new test (Deref newtypes vs Option) - noise, fixed in one iteration.

## What to improve next time

- When a fix would require ordering after something that is itself
  ordered after the consumer (the aim chain's after-Propagate vs the
  projection's before-Layout), the resolution is usually to change WHERE
  the data comes from (TransformHelper), not to fight the schedule.
  Second application of that pattern; it belongs in the eventual
  two-clocks doc section if a third comes.

## Action items

- [x] None; family code complete. The umbrella's combined verification is
      next once the user-queued tasks are filed.
