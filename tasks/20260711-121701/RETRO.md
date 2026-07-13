# Retro: Decel wobble - premise falsified, camera-side redirect

- TASK: 20260711-121701
- BRANCH: fix/decel-wobble (squash-merged as d08f153)
- REVIEW ROUNDS: 1 (APPROVE, no findings)

A falsification cycle: the reported physical wobble does not exist; the
evidence, the pinned regression, and the camera-side redirect are in the
task file.

## What went well

- **Reading the shipped scenario config first inverted the investigation
  in minutes.** The plan's balancer-chatter hypothesis died on one fact
  (the player ship has a single engine) BEFORE any probe code was
  written, and the diagnostic was re-aimed at the exact playtest
  scenario instead.
- **The trace answered a question the user could not**: hull steadiness
  vs perceived steadiness. 0.0023 rad/s over a full 22 s burn is a
  decisive negative, and converting the trace into a delivery-guarded
  regression turned "we found nothing" into a permanent invariant.
- **Falsified-theory bookkeeping** (residual-roll retro rule): the wrong
  hypothesis, the scope correction, the verified command-path fact
  (PointRotationOutput is camera-independent), and the redirect all live
  in TASK.md, so the next session hunting this feel report starts
  camera-side instead of re-tracing physics.

## What went wrong

- **Third occurrence of "the plan encoded a mechanism that reading one
  file would have falsified"**: 231931's straight-line-burn regression
  (torque-blind), 231930's overshoot formula (wrong algebra), now
  121701's balancer chatter (single-engine ship). Root cause each time:
  plan steps written from the spike's mental model without a
  verify-the-premise step naming the file that could kill it.

## What to improve next time

- Promotion threshold reached (3 occurrences). Proposed plan-skill
  addition (user's call, since the skill is global): under "Guidelines
  for Good Steps", add: "A step that encodes a physical mechanism or a
  dependency's behavior must either cite the file/derivation that
  verifies it, or be phrased as a verify-first question ('confirm X,
  then...'). Plans written from a model of the system, rather than the
  system, have been wrong three cycles in a row."

## Action items

- [ ] Propose the plan-skill guideline above to the user (global skill
      file; not edited unilaterally).
- [x] Camera-side redirect recorded on 20260711-121711 (zoom slew during
      decel) - already in the task queue.
- [x] User re-test guidance written in the task file (5ba0e3c pin landed
      after their session).
