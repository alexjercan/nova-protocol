# probe run multi-spec (comma list, category, --all) + aggregated index report (index.html/json, probe-all.json gate)

- STATUS: CLOSED
- PRIORITY: 59
- TAGS: v0.8.0,tooling,testing,examples

## Goal

`probe run` grows multi-example specs and an aggregated status report, so
one command evaluates a category or the whole example fleet and produces
one honest overview (design: tasks/20260719-205543/SPIKE.md; user
adjudications 2026-07-19: bare `probe run` ERRORS with the runnable list,
`--all` is the run-everything flag).

```
probe run playable                # unchanged
probe run playable,scenario       # comma list
probe run ui                      # category dir -> its cataloged examples
probe run --all                   # every cataloged example minus NOT_PROBED
probe run                         # ERROR: prints the catalog by category + spec forms
```

- Catalog discovery: parse the root Cargo.toml `[[example]]` blocks in a
  parser that LIVES IN nova_probe (category = the examples/<cat>/ path
  segment); `tests/examples_smoke.rs::catalog_matches_disk` switches to
  calling it (root already dev-depends on nova_probe) - one parser,
  pinned by the existing drift test.
- Spec resolution is a PURE fn over an injected catalog (probe-env
  pattern): exact example name wins, else category expands, else error
  naming both viable forms; assert no example name collides with a
  category name.
- Execution: sequential loop, each example through today's run() into its
  own `<base>/<example>/` dir (base = --out or probe-runs/), per-example
  timeout as today; a failed/timed-out example marks its ROW and the
  sweep CONTINUES. Per-run Xvfb spawn stays (deviation from the spike's
  shared-Xvfb sketch: ~1s/run overhead buys zero new lifecycle risk -
  recorded here as the adaptation).
- Aggregate artifacts: `<base>/index.html` + `<base>/index.json` +
  `<base>/probe-all.json` (manifest: spec, started, git sha, host, rows
  with outcome + duration). Rows are built by READING each run's
  checks.json (probe consumes its own agent surface); a run that died
  before checks.json exists becomes a FAIL row carrying the error.
- Row: example, category, verdict badge, measured n/total, six check
  glyphs, duration, link to the per-example report.html. Header: totals,
  aggregate coverage, overall verdict = WORST row; process exit mirrors
  it. NOT_PROBED exclusions (initial: render_scale_shot - BCS_SHOT
  real-GPU pixel capture, no self-ending autopilot) are LISTED IN the
  report with reasons; explicit `probe run render_scale_shot` stays
  allowed (operator choice, with a printed note).
- `probe report <dir>`: probe-all.json -> re-render the index from the
  per-example checks.json files; probe-run.json -> per-run report as
  today; neither -> refused as today.
- Honest flag combinations: `--profile`/`--samply`/`--fps`/`--release`
  apply to EACH example in a multi spec; `--scenario`/`--preset` (the
  matrix) and `--platform web` are single-example concerns and are
  REJECTED with a list/category/--all spec.

## Steps

- [x] Catalog parser in nova_probe (name, path, category; unit-tested);
      switch catalog_matches_disk to it, drift pins unchanged.
- [x] Spec resolution pure fn + parse() wiring (bare run -> catalog
      error listing; --all; comma lists; categories; collision assert)
      + multi-mode flag rejections. Parse pins for all of it.
- [x] Sequential driver: loop run(), collect row from checks.json (or
      FAIL row on missing), durations, continue-on-failure.
- [x] Aggregate render: index.html (reuse report.rs shared pieces) +
      index.json + probe-all.json; worst-of verdict + exit code;
      NOT_PROBED section. probe report gains the probe-all.json branch.
- [x] Docs: probe skill (spec forms + aggregate), wiki Performance
      section, CHANGELOG Unreleased.
- [x] Tests: resolution/rejection pins, parser units, manifest + row
      serde, worst-of pure fn.
- [x] E2E: `probe run ui` (category, 3 examples) -> index with 3 rows +
      per-example reports; `probe run scenario,hud_range` (list);
      `probe report` re-render of the index; bare `probe run` error
      lists the catalog. Record all here.
- [x] Verify: fmt; cargo test -p nova_probe; root drift test still
      passes (`cargo test -p nova-protocol --test examples_smoke
      catalog`).

## Notes

- Spike: tasks/20260719-205543/SPIKE.md. T2 (20260719-210443 wiring
  sweep) fills the rows this task will honestly show as thin (most
  examples measure 2/6 until wired); T1 lands first ON PURPOSE.
- The aggregate must never hide SKIPPED: measured n/total per row is
  mandatory, coverage in the header, worst-of verdict - an unwired
  example is visibly thinner, which is T2's motivation.
- NOT in scope: CI wiring (smoke stays the CI gate; probe-all is the
  local/nightly evidence artifact), RunMeta.profile label (T2), marker
  depth (T3).

## Close-out (2026-07-19, branch feature/probe-all)

Multi-spec + aggregate landed exactly per the spike, all e2es live:

- `probe run ui` (category): 3 sequential runs (editor 271s carrying the
  cold build, hud_range 15s, menu_newgame 13s), aggregate OK, exit 0,
  index.html + index.json + probe-all.json above the per-example dirs.
- `probe run scenario,hud_range` (list): the aggregate's whole point in
  one table - scenario OK measured 5/6 (wired) next to hud_range OK
  measured 2/6 (unwired). T2's motivation is now VISIBLE, not implied.
- `probe report probe-runs`: re-renders the index with rows re-read fresh
  from each checks.json; a foreign dir is refused naming both manifests.
- Bare `probe run`: exit 1 + the full catalog by category + the three
  spec forms. `probe run typo_example`: unknown-spec error with the same
  listing. `probe run playable --all`: contradiction error.
- Tests: 61 lib (catalog parser: fail-closed on missing keys/uncategorized
  paths/duplicates/name-category collisions/discovery-on; aggregate:
  worst-of verdict incl. fail-closed unknown verdicts, manifest JSON
  roundtrip, index-html honesty assertions) + 16 bin (parse specs/--all/
  contradictions + resolve: single-stays-single, category-expands-minus-
  NOT_PROBED, explicit-name-overrides-exclusion, dedupe, catalog-listing
  errors). Root drift test passes against the SHARED parser (the inline
  copy in examples_smoke.rs is gone).
- fmt clean, zero warnings.

Design notes recorded in-flight:
- `--platform web` bypasses catalog resolution (its positional is a
  scenario id, not an example) and rejects multi specs.
- Per-run Xvfb spawn kept (deviation from the spike's shared-Xvfb
  sketch): ~1s/run for zero new lifecycle risk.
- A new multi run into the same base dir replaces the previous aggregate
  (its manifest names ITS spec and rows); per-example dirs persist and
  single-run/`--out` semantics are unchanged.
