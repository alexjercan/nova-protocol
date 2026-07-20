# probe: --baseline works across a group (per-example baseline root, skip examples without one)

- STATUS: IN_PROGRESS
- PRIORITY: 56
- TAGS: v0.8.0,tooling,performance

## Story

As someone running `probe run <category>` / `--all` with `--fps`, I want
`--baseline` to work across the group - comparing each example against its own
prior baseline and quietly skipping the ones that have none - so I can regression-
check a whole fleet in one command instead of one example at a time.

## Current behavior (the gap)

`--baseline` is HARD-REJECTED for any multi-run spec:
`probe run gameplay --baseline <dir>` errors with "--baseline compares one run
dir; it does not combine with a list/category/--all spec" (probe.rs ~451). So a
fleet regression check is impossible today - you baseline one example at a time.

## Design

A multi-run already writes `probe-runs/<example>/` per example, so a prior
`probe run --all --out probe-runs/before` yields `probe-runs/before/<example>/
frametime.csv`. Extend `--baseline` to mirror that layout:

- SINGLE example: `--baseline <dir>` keeps its current meaning - `<dir>` IS the
  baseline run dir (`<dir>/frametime.csv`).
- GROUP / category / `--all`: `--baseline <dir>` means the baseline ROOT (a
  prior probe-runs-shaped dir). Per example, resolve `<dir>/<example>/`: if it
  has a `frametime.csv`, use it as that example's baseline; if not, SKIP the
  comparison for that example (its `fps_within_baseline` stays SKIPPED - the
  normal no-baseline default, NOT an error).
- Log which examples were skipped for lack of a baseline, and warn if the
  baseline root matched NONE of the group's examples (likely a wrong path).

Label matching already lines up: probe labels each example's rows by the
example name, so `before/playable/frametime.csv` (label `playable`) matches the
new `playable` run automatically.

Decision to make explicit (and document in the flag help): `--baseline <dir>`
is the run dir itself for one example, but the root of per-example run dirs for
a group. Intuitive - you baseline `--all` against a previous `--all` out dir.

## Steps

- [x] Drop the multi-run `--baseline` rejection (probe.rs resolve gate); keep
      rejecting the `--scenario/--preset` matrix + `--baseline` combo if that is
      still nonsensical for a group (matrix is single-example only).
- [x] run_many: per example, set `opts.baseline` to `<root>/<example>` ONLY
      when `<root>/<example>/frametime.csv` exists; else None (skip). Add a
      pure/testable helper `group_baseline_for(root, example) -> Option<PathBuf>`
      so the resolve is unit-testable without a full run.
- [x] Diagnostics: eprintln a per-example "no baseline in <root>, skipping fps
      comparison" for the misses, and a single warn if the root matched zero
      examples in the group.
- [x] Keep the single-example path unchanged (its `--baseline <dir>` is still
      the run dir; run()'s pre-run frametime.csv validation still applies -
      the group path only sets opts.baseline when the csv exists, so run()'s
      validation never trips on a group miss).
- [x] Tests: `group_baseline_for` (present -> Some, missing csv -> None,
      missing dir -> None) against a temp fixture; resolve_spec still rejects
      the matrix+multi combos it should.
- [x] Docs: USAGE/flag help (the two meanings of --baseline), probe skill,
      development.md baseline paragraph, CHANGELOG.
- [x] Verify: `probe run <two-example list> --fps --baseline <prior out dir>`
      where only one has a baseline -> that one compares, the other's report
      says SKIPPED, aggregate still green; no bare error.

## Definition of Done

- `probe run <group> --fps --baseline <root>` runs the whole group, comparing
  each example that has `<root>/<example>/frametime.csv` and skipping (SKIPPED,
  not error) those that do not, with a note per skip and a warn if none matched.
- The single-example `--baseline` behavior is unchanged.
- The two meanings of `--baseline` are documented in the flag help + wiki.

## Notes

- Ties to the multi-run aggregate (20260719-210438) and the fps baseline gate
  (fps_within_baseline in run_report). No bcs change; probe-only.

## Verification (2026-07-20)

- Unit test `group_baseline_for` (present -> Some, dir-without-csv -> None,
  missing dir -> None); `cargo check -p nova_probe --all-targets` clean.
- E2e: `run playable,scenario --fps --out before` wrote before/{playable,
  scenario}/frametime.csv. Deleted before/scenario/frametime.csv, then
  `run playable,scenario --fps --baseline before`:
  - playable `fps_within_baseline: PASS` ("worst playable: +1.4%") - compared
    against before/playable.
  - scenario `fps_within_baseline: SKIPPED` + log "scenario: no baseline in
    before, skipping fps comparison" - quiet skip, not an error.
  - group run exited 0 (aggregate green).
- Dispatch verified: single example -> run() (baseline = the run dir,
  unchanged); group -> run_many() (baseline = root, per-example resolve).
