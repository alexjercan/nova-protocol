# nova_perf: add an HTML report-generator binary that turns the frametime JSON/CSV into a styled standalone report (per-scene percentiles, charts, deltas)

- STATUS: OPEN
- PRIORITY: 55
- TAGS: v0.8.0,tooling,performance

## Goal

`nova_perf` already is its own crate: a frame-time capture harness (lib.rs) plus
`scripts/perf-baseline.sh` / `scripts/perf-web.sh` that sweep scenes x presets
and emit per-run JSON + an aggregated `frametime.csv`. The gap the user named is
presentation: the report is hand-written markdown in a task folder. Add a binary
to nova_perf that reads the captured JSON/CSV and generates a nice standalone
HTML report with all the details.

## Steps

- Add `crates/nova_perf/src/bin/perf_report.rs` (or similar): input = a results
  dir (the `perf-baseline.sh` out dir) of `<label>.json` + `frametime.csv`;
  output = one self-contained `.html` file (inline CSS/JS, no external deps so
  it opens offline and can be attached to a task/PR).
- Present per scene x preset: percentiles (p50/p90/p99/max), mean, frame count,
  window, renderer/GPU, resolution; a small bar/line chart per run (inline SVG
  or a tiny vendored chart lib); and deltas vs a chosen baseline run so
  regressions are obvious.
- Have `perf-baseline.sh` optionally invoke the report bin at the end (flag or
  env) so one command produces numbers + HTML.
- Keep the capture harness (lib) untouched; this is pure reporting over its
  existing output. Update the perf docs and the README tools section
  (20260718-152205).

## Notes

- Existing output contract lives in `crates/nova_perf/src/lib.rs` (JSON + CSV
  row schema) and `scripts/perf-baseline.sh`.
- Prior baseline report to mirror in content:
  tasks/20260716-123551/frametime-baseline-report.md.

