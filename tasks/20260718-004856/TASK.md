# Investigate the broadside-high single-frame hitch (75ms max, p99 39ms) seen in the baseline

- STATUS: OPEN
- PRIORITY: 30
- TAGS: performance,bug,v0.8.0

## Story

As the owner of the combat slice's feel, I want to know whether the one frame
spike the baseline flagged is a real stutter players can hit or measurement
noise, so that it is either fixed, cheaply preloaded away, or documented as
noise instead of lurking unexplained in the numbers.

The frame-time baseline (tasks/20260716-123551) flagged one visible tail: on
the Xvfb GPU rig, `broadside-high` showed a 75 ms max / 39 ms p99 hitch that
the Low run did not exhibit. It is a single-frame event (not a sustained
cost), plausibly asset streaming or a particle-system init on first fire
(High keeps particles; Low is spawn-less - consistent with the Low run being
clean). Low value, but worth one glance so it is not a lurking stutter in the
combat slice. Likely a defer after confirming the cause.

## Steps

- [ ] Reproduce broadside-high on the Xvfb GPU rig
      (`probe run 20_perf_baseline --fps --release --scenario broadside
      --preset high`) and confirm the tail is
      stable/repeatable across 3+ runs (if it moves or vanishes, it is noise:
      document and close).
- [ ] Trace the spike frame (Bevy frame-time diagnostics or a targeted span)
      to see whether it is asset upload, particle-effect/pipeline
      compilation on first fire, or something else. The High-only signature
      makes first-particle-use the lead suspect.
- [ ] If it is a first-use init, decide whether a warm-up/preload at scenario
      load is worth it (measure the fix with the same harness); if it is
      noise, document and close.
- [ ] Record the verdict + evidence in this task; if the perf HTML report
      (20260718-152230) has landed, attach its report as the artifact.

## Definition of Done

- The hitch has a named cause backed by a trace, and one of: a measured fix, a
  deliberate defer with the reason, or a noise verdict - written in this task.
- Whatever the verdict, the baseline report's finding 3 is annotated so the
  next baseline run does not re-open the question from scratch.

## Notes

- Baseline report, "Native, discrete GPU" section, finding 3:
  tasks/20260716-123551/frametime-baseline-report.md
- Measure-first policy applies: no speculative preloading without the trace.
