# nova_probe: profiling layer (chrome-trace spans -> top-N systems table + Perfetto attachment + optional samply flamegraph)

- STATUS: CLOSED
- PRIORITY: 70
- TAGS: v0.8.0, spike, tooling, performance

## Goal

Profiling layer for the run-harness: capture Bevy's per-system tracing spans
via `bevy/trace_chrome` and derive the "top-N costliest systems" table by
POST-PROCESSING that chrome-trace JSON (aggregate span durations per system) -
one capture, two products: the inline table (headless, agent-readable) and the
Perfetto-openable attachment. Optionally wrap a native run in `samply` to
attach a Firefox-profiler flamegraph.

## Steps

- [x] Root Cargo.toml feature `trace = ["bevy/trace", "bevy/trace_chrome"]`
      (bevy's per-system spans are #[cfg(feature = "trace")] - verified
      bevy_ecs function_system.rs:52 `info_span!(parent: None, "system",
      name = ...)`; trace_chrome alone only adds the writer, verified
      bevy_internal Cargo.toml:456). The chrome JSON path comes from the
      TRACE_CHROME env var (bevy_log lib.rs:325-327); span names render as
      "system: name=<path>" (bevy_log name_fn, lib.rs:329-339).
- [x] Add `crates/nova_probe/src/profile.rs`: chrome-trace parser (handles
      B/E pairs per tid stack AND complete X events; ts/dur are
      microseconds) + per-system aggregation over "system: name=..." spans
      -> SystemCost { name, calls, total_ms, mean_ms_per_call, share_pct }
      where share is of TOTAL system-span time (no reliable universal frame
      span exists in bevy 0.19 - per-frame math would be fabricated) + a
      plain-text/markdown top-N table renderer. Parser rejects malformed
      files loudly.
- [x] Add the `perf_trace` thin bin: `cargo run -p nova_probe --bin
      perf_trace -- <trace.json> [--top N] [-o table.md]`.
- [x] Add `scripts/perf-profile.sh`: the PROFILED pass driver (spike review
      M2 - separate from the FPS pass; tracing overhead contaminates frame
      times). Builds the example with `--features debug,trace`, runs it
      headless (Xvfb) with TRACE_CHROME=<dir>/trace.json + BCS_AUTOPILOT,
      renders the top-N table via perf_trace, and with SAMPLY=1 does a
      SECOND run under `samply record --save-only` (samply present on this
      host; the script degrades with a note when `command -v samply` fails
      - the run must never fail on a missing profiler).
- [x] Tests: a hand-written fixture chrome trace (B/E pairs, an X event,
      system + non-system spans, out-of-order tids) with LITERAL expected
      aggregation (calls, totals, shares - catches unit and pairing bugs);
      renderer test (top-N cut, ordering by total); malformed-file
      rejection; bin arg parsing.
- [x] E2E: profiled 08_scenario run (trace build + TRACE_CHROME under
      Xvfb), aggregate the REAL trace, verify actual nova system names
      surface with plausible costs; record the headline numbers here.
      Note: the trace feature recompiles bevy - the e2e pays one big build.
- [x] Docs: wiki Performance section gains the profiled-pass paragraph
      (two-pass rule stated); CHANGELOG Unreleased entry; spike fix record.
- [x] Verify: fmt; cargo test -p nova_probe; cargo check --workspace
      --all-targets --features debug; wasm check -p nova_probe (profile.rs
      is pure parsing - target-independent, no cfg needed).

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

## Close-out (2026-07-19, branch feature/probe-profiling)

Shipped per Steps: root `trace` feature, `profile.rs` (chrome-trace B/E+X
parser, per-system aggregation, top-N markdown renderer with the honesty
header), `perf_trace` bin, `scripts/perf-profile.sh` (traced pass + optional
samply pass). 42 nova_probe tests green; workspace + wasm checks clean.

E2E headline (08_scenario, ~30 s autopilot, 2.2M-event trace): render
dominates as expected (run_render_schedule 25.5%, render_system 18.8%), the
debug inspector costs 5.8%, and the table immediately surfaced a real
finding - `insert_asteroid_collider` at 67 ms PER CALL (6 calls, 402 ms
total): asteroid hull collider generation is a frame-hitch candidate and
plausibly related to the open broadside-hitch task 20260718-004856.

TWO DISCOVERED FACTS the plan did not contain:

1. The game's own log filter KILLED all system spans: nova_core
   log_filter_str sets `bevy_ecs=warn` to silence ECS log chatter, but the
   same EnvFilter governs tracing SPANS - the first e2e produced a 38 MB
   trace with ZERO system spans. Root-caused empirically (minimal probe app
   recorded spans fine; the real app did not; diffed the difference).
   Fixed in the script: `RUST_LOG=bevy_ecs=info` rides on top (bevy_log
   0.19 ADDS RUST_LOG directives over the plugin filter and same-target
   directives win - verified by re-run: 456k system spans).
2. samply IS installed here but blocked by `perf_event_paranoid=2`; the
   first script version died on it under `set -e`. Hardened: a failed
   samply run prints the perms hint and the pass still succeeds (proven by
   re-run, EXIT=0, table + trace intact).

Also learned: a traced 30 s run writes ~800 MB of chrome JSON - documented
in the wiki as a scratch artifact, never committed.

Reflection: two rounds of "theory says it should work" were beaten by one
minimal empirical probe each time (the throwaway 99_trace_probe example
settled in 30 seconds what three source-reading theories could not).
Evidence-first debugging remains cheaper than clever reading.

## Addendum (2026-07-19, user-requested in-cycle): the `profiling` profile

User field-tested the samply pass (after raising perf_event_mlock_kb for the
24-core host - "mmap failed" was the ring-buffer budget, now documented) and
found flamegraphs full of raw `0x7fff...` addresses. Root causes: the dev
profile's line-tables-only + unpacked split-debuginfo starves the
symbolicator, and missing frame pointers degrade unwinding (the 0x7fff range
IS stack garbage from misread frames). Fix shipped: a dedicated
`[profile.profiling]` (inherits dev, `debug = true`,
`split-debuginfo = "off"`, dev's package opt mirrored explicitly since
`inherits` does not carry package overrides) and the samply branch builds
with it plus `-C force-frame-pointers=yes`, WITHOUT the trace feature
(sampling needs no spans; span overhead would distort the sampled costs).
Verified structurally: .debug_info 929 MB (profiling) vs 1.6 MB (dev), and
`push %rbp` prologues on our functions; pass green end to end with the
flamegraph artifact produced. Driver-blob frames stay hex (their stripping).
