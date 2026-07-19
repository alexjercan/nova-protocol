# nova_probe: unified run report + verdict (correctness + FPS + profile + log + checklist; absorbs 20260718-152230)

- STATUS: OPEN
- PRIORITY: 68
- TAGS: v0.8.0, spike, tooling, performance, testing

## Goal

The unified run report + verdict: a run DIRECTORY per run - a self-contained
report.html combining the verdict banner (auto per-check results + explicit
reviewer confirm), run summary, correctness (invariant results + rendered
timeline + assertions), FPS/frame-time (the existing chart), profile (top-N
systems + flamegraph links, from the profiled pass), a collapsible structured
log timeline, and a "what to look for" reviewer checklist ending in an
OK/NOT-OK line - plus sidecar attachments (trace JSON, samply profile, raw
timeline) and a machine-readable checks.json so an agent consumes verdicts
without parsing HTML. Provisional auto-verdicts set the process exit code
(soft perf regressions are WARN, not hard fail).

## Notes

- Spike: tasks/20260719-112011/SPIKE.md (see the full report content spec,
  REVISED after review round 1).
- ABSORBS task 20260718-152230: the built `perf_report` HTML generator
  (originally branch feature/perf-html-report, commit a9af7789; on master via
  the spike-branch squash) is the FPS section of this report.
- Correctness = invariants + timeline + assertions; NO golden diff (deferred
  to backlog 20260719-112245; the layout reserves the spot).
- Keep each auto threshold a single tunable AND per renderer class (a flat
  16.6 ms budget would permanently flag sw/web; review m4); log-scan check
  needs a per-example allowlist (review m5); noisy shared host -> avoid false
  alarms (quiet-host-before-measuring).
- Depends on T2 (recorder) / 20260719-114931 (invariants), T4 (profile), and
  the FPS renderer from T1.
