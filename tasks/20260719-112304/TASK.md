# nova_probe: unified run report + verdict (correctness + FPS + profile + log + checklist; absorbs 20260718-152230)

- STATUS: CLOSED
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

## Steps

- [x] Add `src/run_report.rs` + a `run_report` bin: input = a RUN DIRECTORY
      holding whatever artifacts a run produced (timeline.jsonl,
      frametime.csv, trace.json, run.log - each optional), plus
      `--baseline <dir>` and `-o`. Output = `report.html` (self-contained,
      shares report.rs's STYLE/chart via pub(crate) refactor) and
      `checks.json` (machine-readable mirror of the verdict rows, so an
      agent never parses HTML). Missing artifacts make their checks
      SKIPPED and their sections say why - never silently absent.
- [x] Auto checks (each a row: name, status PASS/WARN/FAIL/SKIPPED, value,
      threshold, detail), derived only from what the artifacts guarantee:
      run_completed (timeline has run_end with AppExit Success - a
      truncated timeline IS the crash signal, flush-per-entry made it so);
      invariants_held (invariant_summary violations == 0; per-name
      violation counts shown - a stuck entity violates every frame, T3
      review note); fps_within_baseline (same-label mean delta within a
      single tunable %; WARN not FAIL - noisy-host rule; v2 renderer
      metadata displayed, labels compared only against themselves);
      log_clean (when run.log present: no 'panicked at'/ERROR outside a
      per-example allowlist). Verdict = FAIL if any hard check fails,
      WARN on soft, OK otherwise; bin exit code mirrors it.
- [x] report.html sections per the spike spec: verdict banner (+ explicit
      'reviewer must confirm' line), run summary (from run_start metadata +
      frametime v2 row meta), correctness (invariant counts by name +
      MEANINGFUL timeline table - onupdate pulses collapsed to a count,
      full stream stays in the sidecar), performance (existing chart +
      deltas when frametime.csv present), profile (top-N systems HTML
      table from trace.json via profile.rs; NO share pie - parent/child
      spans overlap, T4 R1.2), collapsible raw-ish log/timeline tail, and
      the reviewer checklist ending in an OK / NOT-OK line.
- [x] Tests over a checked-in mini run-dir fixture (handwritten
      timeline.jsonl with invariant_summary + run_end, v2 frametime.csv,
      mini trace.json): every check's PASS path AND its FAIL/SKIPPED path
      pinned (truncated timeline -> run_completed FAIL; violations>0 ->
      invariants FAIL; artifact absent -> SKIPPED; log with a planted
      panic -> log_clean FAIL); checks.json round-trip; report contains
      each section marker and the skip explanations.
- [x] E2E: run 10_playable armed (timeline + invariants) into a run dir,
      copy in a frametime.csv + trace.json from the tooling, render, and
      EYEBALL the report (render-output-eyeball); verify checks.json
      verdict OK and the exit code.
- [x] Close the absorbed task 20260718-152230 (its perf_report is the FPS
      section; DoD now delivered through this report) - CLOSED status +
      pointer here.
- [x] Docs: wiki Performance section gains the run-report paragraph;
      CHANGELOG Unreleased entry; spike fix record.
- [x] Verify: fmt; cargo test -p nova_probe; workspace all-targets check;
      wasm check (run_report native-only next to the recorder).

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

## Close-out (2026-07-19, branch feature/probe-run-report)

Shipped per Steps: run_report.rs (RunArtifacts loader with
corrupt-vs-absent distinction, 4 auto checks, checks.json mirror, 7-section
report.html reusing the shared STYLE/chart/table) + the run_report bin
(exit mirrors the verdict; wasm stub main). 48 nova_probe tests green (8
new: every check's PASS and FAIL/SKIPPED path, incl. the truncation-is-
the-crash-signal semantics); workspace all-targets + wasm checks clean.

E2E (real armed 10_playable run dir): verdict OK - run_completed PASS at
frame 1372, invariants_held PASS (0 over 1372 frames), log_clean PASS,
fps_within_baseline honestly SKIPPED (10_playable carries no frametime
capture; the run-dir contract treats that as NOT MEASURED, displayed as
such). 1349 onupdate pulses collapsed; report is 23 KB self-contained.
The planned "copy in a frametime.csv + trace.json" e2e clause was
DROPPED deliberately: planting foreign artifacts into a real run dir
would fabricate a run that never happened - the full-artifact rendering
is pinned by the fixture tests instead, and the honest correctness-only
run shape is exactly what T6's runner will produce most often.

One deviation caught by reading exit codes, not assuming: the new bin
broke the wasm check (native-only module) - cfg'd stub main, re-verified.

Absorbed task 20260718-152230 CLOSED with its DoD disposition recorded.

Review notes inherited and honored: per-name violation counts (T3), no
share pie / rank-only profile table (T4 R1.2), WARN-not-FAIL FPS gate
(spike m4/m5), SKIPPED-never-held semantics throughout.
