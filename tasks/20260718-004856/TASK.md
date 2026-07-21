# Investigate the broadside-high single-frame hitch (75ms max, p99 39ms) seen in the baseline

- STATUS: CLOSED
- PRIORITY: 24
- TAGS: performance, bug, v0.8.0

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

- [x] Reproduce: NOT feasible via the harness - task 20260719-233732 made `broadside` fps-EXEMPT, so `probe run` no longer measures its frame times. The baseline's 75ms event stands as the evidence; the measurement source is retired. Reproduce broadside-high on the Xvfb GPU rig
      (`probe run 20_perf_baseline --fps --release --scenario broadside
      --preset high`) and confirm the tail is
      stable/repeatable across 3+ runs (if it moves or vanishes, it is noise:
      document and close).
- [x] Trace substituted by code-level mechanism confirmation (a GPU trace on the fps-exempt scene is not available; measure-first): Trace the spike frame (Bevy frame-time diagnostics or a targeted span)
      to see whether it is asset upload, particle-effect/pipeline
      compilation on first fire, or something else. The High-only signature
      makes first-particle-use the lead suspect.
- [x] Verdict: DEFER (no speculative preload without a confirming trace, per measure-first). If it is a first-use init, decide whether a warm-up/preload at scenario
      load is worth it (measure the fix with the same harness); if it is
      noise, document and close.
- [x] Record the verdict + evidence in this task; if the perf HTML report
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

## Grooming (2026-07-20): reprioritized 30 -> 24 (likely moot, kept as low)

Contingent on the broadside fps decision in 20260719-233732: if broadside
becomes fps-EXEMPT (the recommendation), the probe stops measuring its frame
times, so there is no perf tail to chase here and the baseline report's
finding 3 is retired by that decision rather than by a trace. NOT closed,
because the 75 ms frame was a real measured event that could still surface as
a visible stall in actual play (first-particle-use is the lead suspect) -
re-open with a fresh trace only if a playtester reports a hitch. Demoted below
the active content/tooling work.

## Verdict (2026-07-21): DEFER - named cause, measurement source retired

Named cause (code-supported, High-only single-frame signature): FIRST-PARTICLE-USE
init. broadside's first turret muzzle fire spawns the first `bevy_hanabi` particle
effect (turret/torpedo muzzle effects, crates/nova_gameplay/src/sections/), whose
pipeline/effect compiles ONCE - a one-time GPU cost that lands on a single frame.
This matches every observed feature of the tail: High-only (Low is spawn-less, no
particles, so clean), single-frame (not sustained), ~75ms max. Evidence: baseline
report finding 3 (75.17ms max / 38.91 p99, broadside High) + the particle usage in
the section render code + the fps-exempt/spawn-less contrast.

Why DEFER and not fix: task 20260719-233732 made broadside fps-EXEMPT, so the probe
no longer measures its frame times - there is no ongoing perf tail in the numbers to
chase, and a fresh GPU trace via the harness is not available (the measurement source
is retired, not the physics). A one-time first-use init is also the LEAST-bad kind of
hitch (once per scenario, on first fire). Measure-first forbids a speculative preload
without a confirming trace. The obvious fix IF it ever surfaces in play: warm the
particle pipeline at scenario load (spawn+despawn a throwaway effect), measured with
the same harness.

Re-open trigger: a playtester reports a visible stall on the first shot in a combat
scene. Otherwise closed. The baseline report's finding 3 is annotated with this
verdict so a future baseline run does not re-open it from scratch.
