# nova_perf: add an HTML report-generator binary that turns the frametime JSON/CSV into a styled standalone report (per-scene percentiles, charts, deltas)

- STATUS: OPEN
- PRIORITY: 55
- TAGS: v0.8.0,tooling,performance

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

## Steps

- [ ] Add `crates/nova_perf/src/bin/perf_report.rs` (or similar): input = a
      results dir (the `perf-baseline.sh` out dir) of `<label>.json` +
      `frametime.csv`; output = one self-contained `.html` file (inline
      CSS/JS, no external deps so it opens offline and can be attached to a
      task/PR).
- [ ] Present per scene x preset: percentiles (p50/p90/p99/max), mean, frame
      count, window, renderer/GPU, resolution; a small bar/line chart per run
      (inline SVG or a tiny vendored chart lib); and deltas vs a chosen
      baseline run so regressions are obvious.
- [ ] Have `perf-baseline.sh` optionally invoke the report bin at the end
      (flag or env) so one command produces numbers + HTML.
- [ ] Keep the capture harness (lib) untouched; this is pure reporting over
      its existing output.
- [ ] Add a small test fixture (a checked-in mini results dir) and a test that
      renders it, so the report bin does not silently rot when the capture
      schema changes.
- [ ] Update the perf docs (dev wiki development.md) and the README tools
      section (20260718-152205).
- [ ] Generate the report over the existing v0.7.0 baseline results as the
      end-to-end example, and use it when investigating 20260718-004856.

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
