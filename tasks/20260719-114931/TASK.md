# nova_probe: continuous invariant assertions during autopilot runs (health/speed/state-machine bounds)

- STATUS: OPEN
- PRIORITY: 72
- TAGS: v0.8.0,spike,tooling,testing

## Goal

Continuous invariant assertions during autopilot runs: a set of always-true
checks evaluated while a probe run plays - e.g. health never negative, speed
respects the configured cap, scenario acts/variables move monotonically where
the design says they must, entity counts stay bounded. Violations are recorded
as structured events on the run timeline (and can panic in strict mode);
results feed the correctness section and the `invariants held` auto-check of
the run report.

## Notes

- Spike: tasks/20260719-112011/SPIKE.md. Chosen over golden timelines in the
  round-1 review adjudication (user, 2026-07-19); goldens deferred to backlog
  task 20260719-112245. Invariants catch always-been-wrong bugs a golden diff
  cannot, and are immune to host timing noise (llvmpipe vs dev GPU).
- Derive invariant bounds from the engine's decision constants (speed caps,
  health floors), not hand-written expected values
  (rule-inputs-rederive-from-engine).
- Depends on the recorder (20260719-112238): violations ride the same
  structured event stream.
