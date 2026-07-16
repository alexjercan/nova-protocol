# Gameplay performance baseline: frame-time capture on heavy scenes (native + web), fix what the numbers justify

- STATUS: OPEN
- PRIORITY: 40
- TAGS: v0.7.0,performance,spike


## Goal

"Improve performance" currently has no numbers to aim at: v0.6.0 benchmarked
the modding dispatch layer (tasks/20260714-083331/modding-perf-report.md),
but there is no frame-time baseline for actual gameplay scenes. Produce one:
capture frame times on the heavy scenes (dense asteroid scatter a la
asteroid_field, full combat with particles/torpedoes, the vertical slice
once it exists) on native AND the web/WebGPU build, publish the report in
this task folder, then fix ONLY what the numbers justify - same measure-first
gate the modding perf work used. Findings that are noise get documented and
deferred, not "optimized".

## Notes

- Spike: tasks/20260716-122954/SPIKE.md (v0.7.0 release scope)
- Plan: docs/plans/20260716-v0.7.0-plan.md, strand 2
- The low-end spawn-less/reduced visual mode (20260525-133013) is tuned
  against this baseline and surfaces as the settings menu's graphics preset
  (20260711-180511).
- Web is the constrained target; test on the weakest realistic setup (see
  LESSONS.md notes on headless/iGPU verification).
