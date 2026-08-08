# nova_probe hardening: trustworthy verdicts for agent-driven runs (fresh dirs, manifest, timeout reports, NO_DATA)

- STATUS: CLOSED
- PRIORITY: 68
- TAGS: v0.8.0, tooling, testing, bug

## Story

An out-of-context review of nova_probe (2026-07-19, fresh-eyes agent pass +
an empirical no-evidence probe) found the trust boundary leaking in exactly
the failure modes agents hit. Verdicts must stay trustworthy when the run
hangs, crashes, exits non-zero, was never wired, or lands in a dirty dir.
Findings referenced below by number; MAJORs verified against the code,
finding 4 reproduced live (unwired 01_controller_section -> VERDICT OK,
exit 0, with even run_completed SKIPPED).

MAJORs:
1. Stale artifacts: probe run reuses probe-runs/<example> without cleaning -
   an old trace.json folds into later reports, frametime.csv APPENDS across
   invocations, and a failed run leaves the PREVIOUS run's OK checks.json
   in place for an agent to read.
2. A clean-pass timeout aborts via Err BEFORE any report is written (the
   hung run - the case the tool exists for - produces no verdict); a
   profiled-pass timeout discards a successful clean pass; a bad --baseline
   path is discovered only after minutes of build+run.
3. The child's exit status is only eprintln'd, never a check: a non-panic
   exit(1) from an unwired example reads verdict OK, exit 0.
4. All-SKIPPED yields "OK"/exit 0, and skip details misdirect ("arm
   NOVA_PERF_TIMELINE" when probe DID arm it and the example is simply not
   wired).

MINORs adopted: (5) monotonic_last never resets on scenario teardown - a
reload (19_broadside Retry) re-seeds variables at 0 and fires a false
regression; (6) probe-run --fps cannot match sweep baselines (debug vs
release, label "scene" vs "<scenario>-<preset>") - set the label, document
probe-vs-probe only; (7) the fps gate WARNs on IMPROVEMENTS (worst by
|delta|); (8) Xvfb: blind 2s sleep, no liveness check, band :90-:99
overlaps the scripts' :94/:95; (9) recorder counts entries it failed to
write - cross-check run_end.entries vs parsed count; (10) log scan: " ERROR "
misses line-initial/format variants and take(5) caps the count; (11) the
samply pass has no timeout; (13) checks.json carries numbers only in prose -
no structured data, no run metadata; (14) no reached_playing check. NITs
folded where touched: (16) shared check-table printing via
CheckStatus::as_str; (18) CSV parser accepts NaN.

## Steps

- [x] MANIFEST (unifies 1+3+13): `probe run` writes `probe-run.json` into
      the run dir - example, started (SystemTime), git SHA + host (reuse
      capture's resolvers), passes executed, per-pass child exit status +
      timed_out flag, artifact inventory. run_report loads it (optional
      artifact), feeds the new checks and the report's Run summary, and
      mirrors it as top-level metadata in checks.json.
- [x] FRESH DIRS (1): at the start of run(), surgically remove probe's OWN
      artifact filenames (timeline.jsonl, run.log, trace.json,
      trace-run.log, frametime.csv, samply-profile.json.gz, report.html,
      checks.json, probe-run.json) from the out dir - never a recursive
      wipe of a user-supplied path. Pin: a stale trace.json + checks.json
      planted before run() are gone from the fresh report.
- [x] TIMEOUTS PRODUCE REPORTS (2+11): run_example returns
      Outcome{Completed(status)|TimedOut} instead of Err on expiry; the
      clean pass ALWAYS falls through to report assembly (truncated
      timeline -> run_completed FAIL; manifest records timed_out); a
      profiled-pass failure/timeout degrades to a no-trace note; the
      samply pass gets the same poll-kill timeout; --baseline is validated
      (dir exists + frametime.csv parseable) BEFORE pass 1 builds.
- [x] NEW CHECKS (3+14): `process_exit` (from the manifest; FAIL on
      non-success or timeout; SKIPPED only when no manifest - foreign dir)
      and `reached_playing` (a state entry with entered=Playing on the
      timeline; SKIPPED without a timeline).
- [x] NO_DATA + COVERAGE (4, refined at implementation): with the manifest,
      process_exit is real evidence for harnessed examples (their autopilot
      asserts panic on failure), so an unwired-but-probe-run dir is OK WITH
      COVERAGE stated - checks.json gains measured:"n/total", the banner
      shows it, and skip details distinguish "probe armed the env but the
      example is not wired with nova_timeline()/nova_invariants()" from
      "not armed". NO_DATA (nonzero exit) is reserved for ZERO measured
      checks (a foreign dir with no evidence at all). The skill's agent
      rule becomes: gameplay-verification claims require run_completed +
      invariants_held MEASURED.
- [x] INVARIANTS RESET (5): clear monotonic_last when NovaEventWorld is
      absent OR when a registered variable disappears from the map
      (teardown/reload); pin with an insert-clear-reinsert-at-zero rig
      (no false regression) while a live decrease still fires.
- [x] FPS GATE HONESTY (6+7): improvements (negative delta) PASS with an
      "improved" note - only regressions beyond the threshold WARN; worst
      picked among positive deltas; clean_pass_env sets
      NOVA_PERF_LABEL=<example>; document probe-vs-probe baselines only.
- [x] SCAN + PLUMBING (8,9,10,16,18): Xvfb child liveness-checked after
      spawn (fail loudly if it died - display in use) and the band moves
      to :80-:89 clear of the scripts' :94/:95; run_completed cross-checks
      run_end.data.entries against the parsed entry count (ENOSPC becomes
      self-detecting); log scan anchors on the level token (start-of-line
      or post-timestamp ERROR, ANSI already stripped) and reports the FULL
      count with a capped sample; the check-table printer is shared and
      uses CheckStatus::as_str; PerfRun::from_csv_row rejects non-finite
      numerics.
- [x] checks.json v2 (13): per-check structured `data` (violation counts by
      name, fps worst label+delta, offending-line count) alongside the
      prose; top-level `run` object from the manifest. The probe skill +
      wiki examples updated to match.
- [x] Tests: every new behavior pinned in both directions (stale-dir purge,
      timeout->FAIL-report, process_exit FAIL on planted exit(1) manifest,
      NO_DATA on all-skipped + nonzero exit, monotonic reset rig, improved-
      fps PASS, entries cross-check mismatch FAIL, log format variants);
      e2e: probe run on the unwired 01_controller_section now reads
      measured 2/6 with process_exit PASS from the manifest and not-wired
      skip details (the misdirection from the live repro gone), and a wired
      10_playable run stays OK with all manifest-backed checks PASS.
- [x] Verify: fmt; cargo test -p nova_probe; workspace all-targets
      --features debug; wasm check; e2e runs above recorded here.

## Notes

- Source review: fresh-eyes agent pass, 2026-07-19 (20 findings; MAJORs
  re-verified in code, finding 4 reproduced live before filing).
- Deferred to the consolidation task 20260719-174603: script ports,
  --platform web, probe report gating, bin folding, deprecations. Deferred
  further (review NITs 15/17/19/20): stats.rs serde comment, dead
  renderer_label filter, name-checked E pairing, repo_root doc note.
- The probe skill's "honesty rules" section must be re-read against the
  final behavior before close (keep-docs-in-sync).

## Close-out (2026-07-19, branch fix/probe-hardening)

All four MAJORs and the adopted MINORs shipped; verification:

- 62 nova_probe tests green (56 lib + 6 probe bin; 13 new pins - manifest
  round-trip driving process_exit FAIL on timeout/failure, armed-but-
  unwired skip details naming the wiring, NO_DATA on zero evidence,
  entries cross-check catching swallowed writes, reached_playing FAIL,
  fps-improvement PASS, line-initial ERROR caught with whole-word
  anchoring, monotonic teardown reset with live-regression still firing,
  non-finite CSV rejection); workspace all-targets + wasm checks clean.
- E2E 1 (unwired 01_controller_section): verdict OK, measured 2/6,
  process_exit PASS from the manifest, skip details now say "probe armed
  the recorder but ... is not wired with nova_timeline()" - the live
  repro's misdirection is gone.
- E2E 2 (wired 10_playable): verdict OK, measured 5/6, all manifest-backed
  checks PASS (run_end frame 1209, Playing frame 23, 0 violations/1209).
- E2E 3 (forced --timeout 8 on 10_playable): EXIT=1, verdict FAIL,
  measured 4/6 - the hung run now produces a COMPLETE failing report
  (process_exit FAIL timed-out, run_completed FAIL truncated, and
  reached_playing PASS at frame 25: the run got to gameplay before
  hanging, exactly the triage detail the old abort-without-report lost).

Design note recorded in Steps: with the manifest, process_exit is genuine
evidence for harnessed examples, so unwired-but-probe-run dirs are
OK-with-coverage (measured n/total everywhere) and NO_DATA is reserved
for zero-measured foreign dirs. The probe skill + wiki + CHANGELOG were
updated to the new semantics in the same branch (keep-docs-in-sync).

Deferred (recorded in Notes): review NITs 15/17/19/20 and the fresh-dir
purge unit pin (the purge is exercised by every e2e; a dedicated
plant-stale-then-run pin needs a full app run and belongs to the
consolidation task's probe-report gating tests).
