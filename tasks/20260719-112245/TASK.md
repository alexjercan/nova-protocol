# nova_probe: golden run-timeline compare + bless workflow (drift detection with tolerance)

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: backlog, spike, tooling, testing

## Closed (2026-07-24, superseded)

Closed during v0.9.0 planning triage. Superseded by the continuous invariant
assertions in nova_probe (20260719-114931, CLOSED), which were chosen as the
drift-detection mechanism instead of golden-timeline compare + bless.

## Goal

Golden run-timeline compare + bless workflow: store a checked-in golden timeline
per autopilot example, compare a fresh run against it, and surface drift
(matched / missing / extra events, and value-drift rows where a scenario
variable is outside tolerance). A `--bless` command regenerates goldens.

## Notes

- DEFERRED TO BACKLOG (2026-07-19): per the spike review round 1 (M3,
  tasks/20260719-112011/REVIEW.md) and user decision - invariant assertions
  (20260719-114931) replace goldens as the automated correctness mechanism.
  Reasons: llvmpipe CI runs differ structurally from dev-GPU runs (a
  total-order golden may never match cross-host; needs per-track partial
  order), snapshot fatigue trains rubber-stamp blessing, and the queued
  campaign-polish tasks (20260718-152313, 20260716-174729) would churn every
  campaign golden immediately.
- Entry gate for picking this back up: T2's (20260719-112238) empirical
  timeline-stability data exists AND the campaign content has settled. Also
  define the bless discipline here (bless requires the diff in the commit).
- Spike: tasks/20260719-112011/SPIKE.md.
- Drift = structured diff with per-variable numeric tolerance; do NOT compare
  wall-clock timing directly (host-dependent).
- Depends on the recorder (T2). Feeds the correctness section of the report
  (T5, which reserves the layout spot).
