# Review: probe shared Xvfb + display walk

- TASK: 20260719-224011
- BRANCH: fix/probe-shared-xvfb
- ROUND: 1

## What I tried to break

- **The original race, structurally**: with one server per sweep there is
  no kill/respawn cycle left to race - the fix removes the mechanism, not
  the symptom. run()'s explicit-display path (no spawn when pinned) was
  already the single-run --display contract; the driver reuses it.
- **The collision case, empirically**: the first e2e died on the user's
  LIVE sweep holding :84 - an unplanned, honest reproduction. The walk
  version completed on the same host while that sweep kept running; the
  full band, once each, is pinned pure (no servers spawned in tests).
- **Worst-case latency**: ten dead candidates cost ~20s (2s probe each)
  before a clear error naming the walk - acceptable for a dev tool, and
  the error still offers --display.
- **--display semantics unchanged**: explicit displays skip the walk
  entirely, in both single and multi runs.

## Findings

- R1.1 (NIT, accepted): the 2s-per-candidate probe is sleep-based, not
  event-based; a readiness poll would shave seconds but add a protocol
  dependency (X socket probing). Not worth it at this failure rate.

## Verdict

- VERDICT: APPROVE - land immediately (the user's fleet runs are exposed until it
lands).
