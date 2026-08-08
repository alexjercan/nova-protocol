# nova_perf: add an HTML report-generator binary that turns the frametime JSON/CSV into a styled standalone report (per-scene percentiles, charts, deltas)

- STATUS: CLOSED
- PRIORITY: 55
- TAGS: v0.8.0, tooling, performance

## Story

As the person reading perf results (owner or reviewing agent), I want one
command to turn a capture run into a self-contained HTML report with charts
and baseline deltas, so that regressions are visible at a glance instead of
being hand-transcribed into markdown per task.

`nova_perf` already is its own crate: a frame-time capture harness (lib.rs)
plus `scripts/perf-baseline.sh` / `scripts/perf-web.sh` that sweep scenes x
presets and emit per-run JSON + an aggregated `frametime.csv`. The gap is
presentation: the current report is hand-written markdown in a task folder
(tasks/20260716-123551/frametime-baseline-report.md).

## PAUSED (2026-07-19): pending the unified-nova_perf spike

User direction (2026-07-19): before finishing this, spike a UNIFIED `nova_perf`
tool - one `cargo run -p nova_perf` entrypoint that folds the two shell scripts
into subcommands, takes `--platform native|web` and `--export csv,html,json`,
and adds a `--profile` layer (samply flamegraph + Bevy Tracy/chrome-trace spans
+ a top-costliest-systems table in the HTML). This task's HTML report is the
FPS-section PIECE of that vision. The implementation below is DONE and tested
on branch `feature/perf-html-report` (checkpoint commit a9af7789).

ABSORBED by the spike (2026-07-19): the design landed as
tasks/20260719-112011/SPIKE.md, and this task's `perf_report` code is now the
FPS section of the unified run report, task 20260719-112304 (T5). Do not merge
or close this task on its own; it is carried forward by T5. Kept here as the
record of the FPS renderer's origin.

## Steps

- [x] Add `crates/nova_perf/src/bin/perf_report.rs`: input = a results dir (the
      `perf-baseline.sh` out dir) `frametime.csv`; output = one self-contained
      `.html` file (inline CSS + inline SVG, no external deps, opens offline).
      Reads only the aggregated CSV (schema `nova_perf::CSV_HEADER`), so reader
      and writer share one column contract.
- [x] Present per scene x preset: percentiles (p50/p95/p99/p999/max - the JSON
      schema carries p95 not p90), mean, frame count, window; a bar chart per
      run (inline SVG, mean bar + p99 marker + 60fps budget line); and deltas
      vs a chosen `--baseline` dir so regressions are obvious. (renderer/GPU/
      resolution are NOT in the per-run schema; renderer is shown from the dir
      name. Capturing GPU/res into the schema is spike scope.)
- [x] Have `perf-baseline.sh` optionally invoke the report bin at the end
      (`REPORT=1` / `REPORT_BASELINE=<dir>`).
- [x] Keep the capture harness (lib) untouched beyond ADDING a public CSV
      reader (`parse_frametime_csv`, `FrameStats::from_csv_row`, `PerfRun`); the
      capture path is unchanged.
- [x] Add a small test fixture (`tests/fixtures/mini` + `mini-baseline`) and a
      test that renders it (structure, numbers, delta classes), plus lib unit
      tests for the parser (literal, round-trip, header/row rejection).
- [~] Update the perf docs: dev wiki `development.md` gets a "## Performance"
      section (DONE). README tools section is task 20260718-152205 (deferred to
      run late, per that task's own note).
- [x] Generate the report over the existing v0.7.0 baseline results as the
      end-to-end example (xgpu vs sw baseline, sw no-baseline; verified 6 runs,
      deltas, budget flags, error paths). Use when investigating 20260718-004856.

## Definition of Done

- `cargo run -p nova_perf --bin perf_report -- <results-dir>` (exact name may
  differ) emits a single HTML file that renders offline with percentiles,
  charts and deltas for every run in the dir.
- `perf-baseline.sh` can produce the report in the same invocation.
- The report over the v0.7.0 baseline reproduces the hand-written report's
  numbers (spot-checked), and the schema test pins the contract.

## Notes

- Existing output contract lives in `crates/nova_perf/src/lib.rs` (JSON + CSV
  row schema) and `scripts/perf-baseline.sh`.
- Prior baseline report to mirror in content:
  tasks/20260716-123551/frametime-baseline-report.md.
- If styling is cheap to share, keep it visually consistent with the content
  audit report (20260718-152240) so the project's generated reports read as a
  family.

## CLOSED (2026-07-19): delivered through the unified run report

The perf_report HTML generator shipped with T1 (nova_probe rename,
03828732) and became the Performance SECTION of the unified run report in
T5 (20260719-112304): `run_report` renders the same chart/table/deltas
inside report.html, and perf_report remains as the standalone
FPS-results-dir tool. DoD disposition: report renders offline with
percentiles/chart/deltas (shipped); perf-baseline.sh renders in the same
invocation (REPORT=1, shipped); the v0.7.0 baseline spot-check and schema
pin shipped with T1 (v1 back-compat tests + fixtures).
