# nova_probe: profiling layer (chrome-trace spans -> top-N systems table + Perfetto attachment + optional samply flamegraph)

- STATUS: OPEN
- PRIORITY: 70
- TAGS: v0.8.0, spike, tooling, performance

## Goal

Profiling layer for the run-harness: capture Bevy's per-system tracing spans
via `bevy/trace_chrome` and derive the "top-N costliest systems" table by
POST-PROCESSING that chrome-trace JSON (aggregate span durations per system) -
one capture, two products: the inline table (headless, agent-readable) and the
Perfetto-openable attachment. Optionally wrap a native run in `samply` to
attach a Firefox-profiler flamegraph.

## Notes

- Spike: tasks/20260719-112011/SPIKE.md (Profiling section, REVISED after
  review round 1).
- Review M1 correction: Bevy has NO per-system timing diagnostic
  (`SystemInformationDiagnosticsPlugin` is OS CPU%/memory only); per-system
  costs exist only as `trace`-feature spans. No trace feature is wired in the
  repo today - this task adds it, feature-gated so normal builds pay nothing.
- Review M2: the profiled run is a SEPARATE pass from the FPS run (tracing
  overhead contaminates frame times); this task's artifacts never feed the
  report's FPS section.
- Native-only for samply + deep diagnostics; degrade gracefully (skip with a
  note) when samply is unavailable or perms (perf_event_paranoid) block it - the
  report must never fail on a missing profiler.
- Tracy (bevy/trace_tracy) stays a documented MANUAL deep-dive option, not the
  automated path (needs a live GUI).
- Depends on the crate skeleton (T1). Feeds the profile section of the report (T5).
