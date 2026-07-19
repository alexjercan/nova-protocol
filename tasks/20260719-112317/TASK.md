# nova_probe: runner CLI + example opt-in + docs (one command per autopilot example; fold perf scripts into subcommands)

- STATUS: CLOSED
- PRIORITY: 66
- TAGS: v0.8.0, spike, tooling, docs

## Goal

Runner CLI + example opt-in + docs: one entrypoint (`cargo run -p nova_probe
-- ...`) that runs a named autopilot example headless (native or `--platform
web`) in TWO PASSES - pass 1 clean (FPS + timeline + invariants), pass 2
profiled (chrome trace + samply, FPS discarded; review M2) - collects the
artifacts into the run directory, and writes the report; `--profile` enables
pass 2, `--export csv,html,json`. Fold `perf-baseline.sh` / `perf-web.sh`
into subcommands. Wire the tool into the post-feature workflow and document it.

## Steps

- [x] Add the `probe` bin + `default-run = "probe"` so `cargo run -p
      nova_probe -- ...` is the one front door. Subcommands: `run` (the
      post-feature check, native Rust orchestration) and thin wrappers
      `sweep` / `web` / `profile` that exec the battle-tested scripts
      (perf-baseline.sh / perf-web.sh / perf-profile.sh) - PLAN ADAPTED:
      "fold the scripts into subcommands" means one front door with the
      scripts as the engine, not a Rust rewrite of Xvfb/trunk/chromium
      orchestration; a rewrite would be churn without new capability
      (the tooling-inventory umbrella 20260718-152304 can revisit).
- [x] `probe run <example> [--out <dir>] [--profile] [--samply] [--fps]
      [--baseline <dir>] [--timeout <s>] [--display <:N>]`: pass 1 CLEAN -
      build `--features debug`, throwaway Xvfb (or --display), run with
      BCS_AUTOPILOT=1 + NOVA_PERF_TIMELINE + NOVA_PERF_INVARIANTS=1,
      stdout/stderr captured to run.log (--fps adds NOVA_PERF=1 +
      NOVA_PERF_OUT for the examples wired with the capture plugin); pass 2
      PROFILED (--profile) - separate build `--features debug,trace`, run
      with TRACE_CHROME into the run dir + the RUST_LOG=bevy_ecs=info
      override (env-filter-governs-spans), its log kept SEPARATE
      (trace-run.log - the profiled pass never feeds the log_clean check);
      optional --samply third run (profiling cargo profile + frame
      pointers, tolerant of missing/blocked samply). Then run_report
      IN-PROCESS (same crate - no subprocess), exit code mirrors the
      verdict. PLAN ADAPTED: no `--platform web` on `run` (timeline/
      invariants are native-only by T2/T3 design; `probe web` wraps the
      web FPS path) and no `--export` flag (the artifacts ARE the
      formats: html + json + csv sit in the run dir).
- [x] Process hygiene: Xvfb spawned on a private display and killed by
      recorded PID via a Drop guard (never pkill); the example run gets a
      poll-based timeout (default 180 s) with kill-on-expiry so a hung run
      cannot wedge the check; child env assembled by a pure, unit-tested
      function.
- [x] Tests: arg parsing (all flags, rejects), the pure env-assembly for
      both passes (timeline/invariants always; fps only with --fps; trace
      env carries the bevy_ecs=info override), display allocation, and the
      wrapper-subcommand script resolution. Orchestration itself is proven
      by the e2e, not mocked.
- [x] E2E: `cargo run -p nova_probe -- run 10_playable --out <dir>` full
      cycle on this host - verify the run dir gains timeline.jsonl +
      run.log + report.html + checks.json with verdict OK; then once with
      --profile and confirm trace.json + the report's profile section
      populate. Record the invocation + verdict here.
- [x] Docs: wiki Performance section restructured to LEAD with `probe run`
      as the post-feature correctness+perf check (the passes and scripts
      become the detail under it); CHANGELOG Unreleased entry; spike fix
      record + family-complete note; README tools section stays with
      20260718-152205 (its task already lists nova_probe).
- [x] Verify: fmt; cargo test -p nova_probe; workspace all-targets check;
      wasm check (probe bin gets the stub main IN THE SAME EDIT as its
      registration - T5's lesson).

## Notes

- Spike: tasks/20260719-112011/SPIKE.md (Architecture, REVISED after review
  round 1: two-pass runner, run-directory artifact).
- The plugin (RunProbePlugin) is opt-in per example, inert unless armed.
- Docs: dev wiki development.md (the Performance section) + README tools section
  (task 20260718-152205) + how to run it as the post-feature correctness+perf
  check.
- Depends on T5 (the report) being usable end to end.

## Close-out (2026-07-19, branch feature/probe-runner-cli)

Shipped per Steps (with the two recorded plan adaptations: script-wrapping
over rewrite; no --platform web / --export on `run`): the `probe` bin +
default-run, native `run` orchestration (clean pass always; --profile
traced pass with the bevy_ecs=info override and doubled timeout; --samply
tolerant third pass; run_report in-process; exit mirrors the verdict),
Xvfb Drop-guard by recorded PID, poll-based run timeout, pure unit-tested
env assembly, sweep/web/profile wrappers.

Verification: 55 tests green (49 lib + 5 probe + 1 doc); workspace
all-targets + wasm checks clean (the wasm stub main shipped in the same
edit as the bin - T5's lesson applied). E2E, both promised runs:
`cargo run -p nova_probe -- run 10_playable --out <dir>` -> verdict OK
(timeline + log + report + checks.json); `-- run 08_scenario --profile`
-> both passes, trace.json rendered into the report's populated top-N
profile section, trace-run.log kept out of log_clean, verdict OK.

This completes the nova_probe run-harness family (T1-T6); the golden
compare remains parked in the backlog by design.
