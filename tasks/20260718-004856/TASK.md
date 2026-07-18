# Investigate the broadside-high single-frame hitch (75ms max, p99 39ms) seen in the baseline

- STATUS: OPEN
- PRIORITY: 30
- TAGS: performance,bug,v0.8.0

## Goal

The frame-time baseline (tasks/20260716-123551) flagged one visible tail: on the
Xvfb GPU rig, `broadside-high` showed a 75 ms max / 39 ms p99 hitch that the Low
run did not exhibit. It is a single-frame event (not a sustained cost), plausibly
asset streaming or a particle-system init on first fire. Low value, but worth one
glance so it is not a lurking stutter in the combat slice. Likely a defer after
confirming the cause.

## Steps

- Reproduce broadside-high on the Xvfb GPU rig
  (`scripts/perf-baseline.sh gpu`) and confirm the tail is stable/repeatable.
- Trace the spike frame (Bevy frame-time diagnostics or a targeted span) to see
  whether it is asset upload, particle-effect compilation, or something else.
- If it is a first-use init, decide whether a warm-up/preload at scenario load is
  worth it; if it is noise, document and close.

## Notes

- Baseline report, "Native, discrete GPU" section, finding 3:
  tasks/20260716-123551/frametime-baseline-report.md
