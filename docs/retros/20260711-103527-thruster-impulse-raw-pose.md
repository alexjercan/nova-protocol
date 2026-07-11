# Retro: Thruster impulses push from the stale eased pose

- TASK: 20260711-103527
- BRANCH: fix/thruster-impulse-raw-pose (squash-merged as 6b2cde0)
- REVIEW ROUNDS: 1 (APPROVE with 1 MINOR, accepted as-is with reasoning)

First spoke of the twitching-family spike
(docs/spikes/20260711-103527-twitching-family-two-clocks.md). Mechanism,
audit table and trace numbers live in the task file; this retro is about
the process.

## What went well

- **Diagnostic-first paid twice.** The ignored tick-trace test not only
  confirmed the stale application point in one run, it REFUTED part of the
  spike's model before any code was written against it: prograde burns are
  torque-benign (stale offset parallel to thrust), the damage is lateral
  thrust at speed. The fix and the regression were designed around the
  corrected mechanism, not the spike's first draft. Same discipline as the
  residual-roll cycle, same payoff.
- **The regression was proven against the bug**: 7.1 rad/s in 15 frames
  unfixed, 0 fixed, on a rig whose true torque is zero by construction
  (thrust line through the COM). It cannot pass vacuously, and the assert
  bound (0.05) is 140x below the failure signal.
- **Reading avian's source settled the fix shape before coding.** The
  planned "prefer the child's own avian Position" turned out wrong -
  `update_child_collider_position` runs before integration, so child poses
  are a tick stale too. The plan step was written as an explicit question
  and got an explicit answer (the camera-twitch retro's "if feasible must
  be answered" lesson, applied).
- **One review round.** The reviewer independently re-verified the
  direct-child assumption in both production spawn paths and re-ran the
  full lib suite; the only finding was a MINOR test-coverage judgment call
  that was resolved with reasoning, not code churn.

## What went wrong

- **The first diagnostic rig was not production-faithful and nearly
  understated the bug.** Without `TransformInterpolation` on the test
  hull, the stale pose only appears on 64-vs-60 Hz double-tick frames and
  the trace looked "mostly clean". Root cause: the rig copied the test
  file's existing spawn conventions instead of the production bundle's
  scheduling-relevant components. One added component changed measured
  severity from 0.94 to 7.11 rad/s.
- **The spike's magnitude claim skipped the vector geometry.** It priced
  the error as |offset| x |force| without walking parallel vs
  perpendicular components, which overweighted the main-drive case and
  missed that the balancer's own recruits are the delivery mechanism.
  Harmless this cycle (the diagnostic corrected it), but a wrong-mechanism
  plan was one lucky trace away.

## What to improve next time

- Clock/schedule bug rigs: before trusting a reproduction, list the
  components that change scheduling or propagation behavior on the
  production entity (TransformInterpolation, interpolation/extrapolation
  opt-ins, custom sync configs) and mirror them in the rig. A clean trace
  on a non-faithful rig is not evidence.
- Force/torque error claims in analysis docs: decompose the error vector
  against the force direction before writing magnitudes; state which
  component does the damage.

## Action items

- [x] Family follow-ups already queued by the spike: 20260710-231931
      re-tests the ship twitch against this fix; bullets/HUD/crosshair
      tasks own the render-clock side.
- [ ] None new; no AGENTS.md/skill changes proposed (both lessons are
      covered by existing retro patterns, now with a second occurrence
      recorded - promote if they recur).
