# Retro: Multi-thruster autopilot (fastest-path group planner)

- TASK: 20260709-121842
- BRANCH: feature/multi-thruster-autopilot (squash-merged as 26d7c3b)
- REVIEW ROUNDS: 1 (APPROVE, 1 NIT deferred)

A smooth cycle; short retro. See the task's close record for the change and
`docs/spikes/20260709-121746-multi-thruster-autopilot.md` for the design.

## What went well

- **Ask-first held.** The previous flight retro's lesson (feel/identity calls
  get a real question round) was applied before writing anything: three
  AskUserQuestion items (firing policy, torque scope, path-choice bias)
  settled the design in one round, all recommendations accepted, zero
  redirects afterward.
- **Arithmetic before assertions.** The rotate-vs-burn crossover (~13-16 u/s
  of delta-v at default knobs) was computed by hand first, so the two bias
  tests were placed deterministically on either side of it instead of being
  tuned until green.
- **The user's framing was corrected cheaply in the spike, not in code.** The
  request suggested using the section graph to learn thruster directions; the
  spike showed each section's local Transform already carries its axis, and
  repositioned the graph/positions where they actually matter (the deferred
  torque-aware allocation). Grounding the spike in code prevented building
  the wrong plumbing.

## What went wrong

- **Two flip tests asserted the parking attitude, not the claim.** The hull
  parks wherever the last pre-deadband command left it, so `forward.dot(Z) >
  0.5` was flaky-by-design; the real claim was "stopping required leaving the
  original facing". Assert the invariant, not the incidental end state.
- **Multi-replacement doc scripts died on stale anchors twice**, losing all
  edits because the write happened after all asserts. Same family as the
  review-hash placeholder lesson: never pre-commit to text you have not just
  read. Switched to apply-what-matches loops with per-edit reporting.

## What to improve next time

- In physics tests, phrase attitude/position assertions as invariants of the
  maneuver ("left the original facing", "at rest within the standoff"), never
  as exact end states.
- Batch text edits should verify all anchors first or apply-and-report per
  edit; all-or-nothing scripts that die mid-way silently drop earlier edits.

## Action items

- [x] 20260709-095043 (retune) now also owns rotation_bias, est_turn_rate_deg
  and arrival_spool_pad. (Delivered 2026-07-09: est_turn_rate_deg became the
  derived flight::hull_turn_rate; rotation_bias and arrival_spool_pad kept
  their values deliberately - see docs/retros/20260709-flight-feel-retune.md.)
- [ ] Torque-aware allocation (section positions/COM) recorded in the spike;
  becomes a task when unbalanced builds visibly misbehave.
