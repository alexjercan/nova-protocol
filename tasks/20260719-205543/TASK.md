# Spike: probe-all - wire every example for correctness/fps, multi-run CLI (list/category/--all) + aggregated status report

- STATUS: CLOSED
- PRIORITY: 61
- TAGS: v0.8.0,spike,tooling,testing,examples


## Outcome (2026-07-19)

SPIKE.md written and reviewed by the user; all three recommendations
accepted with adjudications:
1. Bare `probe run` errors with the runnable list; `--all` runs the fleet.
2. fps wiring EVERYWHERE, with a small dev/release label in the report
   table (RunMeta.profile).
3. T3 stays in-sprint.

Cut: T1 20260719-210438 (p59, multi-run + aggregate), T2 20260719-210443
(p58, fleet wiring + profile label), T3 20260719-210450 (p52, depth
markers). Flow starts on T1.
