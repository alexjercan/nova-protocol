# Review: Remove the redundant closing-speed readout from the destination caption

- TASK: 20260711-125226
- BRANCH: fix/redundant-speed-caption

## Round 1

- VERDICT: APPROVE

Micro-cycle reviewed: the sweep correctly distinguished the caption
(changed) from the telemetry field (kept - reviewer confirmed the flight
planner reads closing speeds in flight.rs and the torpedo HUD's
closing_speed is an unrelated local helper). Format, comment and test
string agree; full lib suite 358/358 re-run by the reviewer; diff is
exactly as big as it needs to be. No findings.
