# nova_probe consolidation: one verb (run absorbs sweep/web/profile), manifest-gated report, delete the .sh scripts

- STATUS: CLOSED
- PRIORITY: 66
- TAGS: v0.8.0, tooling, refactor

## Goal

One verb, one bin: `probe run` absorbs the perf scripts and the report bins
so the front door is the WHOLE surface, and reports can never be built from
data probe did not produce.

- Port the sweep: `probe run 20_perf_baseline --fps --release --render
  gpu|sw --scenario S... --preset P...` (repeatable flags form the matrix
  into ONE fresh run dir; frametime.csv accumulates rows within the
  invocation by design; --render sw sets the lavapipe ICD env exactly as
  perf-baseline.sh does today).
- `--profile`/`--samply` already cover perf-profile.sh: verify parity (the
  top-N lands in the report; keep a `probe trace <trace.json>` verb for the
  standalone table), then delete the script.
- Port the web capture as `--platform web` (perf_web bin via trunk +
  headless Chromium + console scrape): PRESERVE THE EXACT CHROMIUM/WEBGPU
  FLAGS from perf-web.sh (folklore calibrated in the v0.7.0 baseline work);
  validate with a real web capture BEFORE deleting the script - if the port
  cannot be validated in-cycle, the script stays and says so here.
- Fold the report bins: `probe report <run-dir> [--baseline <dir>]`
  re-renders ONLY dirs carrying probe's manifest (see the hardening task -
  its probe-run.json is this gate); remove the run_report, perf_report and
  perf_trace bins (perf_web STAYS - it is the wasm capture app itself, not
  a CLI).
- Deprecation: `probe sweep|web|profile` become one-line aliases that print
  the new invocation and forward; delete the three .sh scripts and sweep
  every live reference (wiki, skill, AGENTS.md, capture-crate docs,
  CHANGELOG note; historical task records stay).

## Steps

- [x] `probe report <run-dir> [--baseline <dir>]`: re-render GATED on
      probe-run.json (a dir without the manifest is refused with a message
      naming `probe run`); prints + exits like run's report step. `probe
      trace <trace.json> [--top N] [-o]` absorbs perf_trace. Remove the
      run_report, perf_report and perf_trace [[bin]]s (perf_web stays - it
      is the wasm capture app); lib re-exports unchanged.
      GATE ADAPTATION recorded: sweep results dirs no longer feed a bare
      report - the sweep now RUNS under probe (next step), which writes the
      manifest, so sweep-vs-sweep comparison keeps working through the gate.
- [x] Sweep port: `probe run <example> --fps --release --render gpu|sw
      --scenario <id>... --preset <p>...` - repeatable flags form the
      matrix; each cell runs with NOVA_PERF_SCENARIO/_QUALITY/_LABEL
      (label = <scenario>-<preset>, the sweep convention) appending into
      THIS run's fresh frametime.csv; --render sw sets the lavapipe ICD
      env exactly as perf-baseline.sh (VK_ICD_FILENAMES/VK_DRIVER_FILES/
      WGPU_BACKEND=vulkan + the sw warmup/frames defaults); --release
      builds + runs target/release (sweep parity - dev-profile FPS numbers
      are not baselines). Manifest records the matrix. Delete
      perf-baseline.sh after a real gpu sweep matches the old script's
      shape (CSV columns + labels).
- [x] perf-profile.sh retirement: `probe run --profile` + `--samply` +
      `probe trace` cover it (verified in T4/T6 e2es); delete the script.
- [x] Web port (`--platform web`, the risky one): port perf-web.sh's flow
      into probe - trunk build of perf.html into a scratch dist, a static
      file server (std TcpListener, no new deps), Chromium with the EXACT
      flag set (copied verbatim from the script - WebGPU adapter folklore),
      scrape the `nova perf: label=` line from the chromium log into a
      frametime.csv v2 row in the run dir, timeout + cleanup by recorded
      PIDs. Correctness surfaces stay native-only (recorder/invariants do
      not arm on web); the report shows the FPS row + manifest.
      VALIDATE with a real web capture BEFORE deleting perf-web.sh; if the
      port cannot be validated in-cycle the script STAYS and this step
      says so.
- [x] Deprecated aliases: `probe sweep|web|profile` print the new
      invocation and forward to it (no scripts left to wrap); remove after
      one release cycle (note in CHANGELOG).
- [x] Reference sweep: every live mention of the three scripts and the
      removed bins re-pointed (wiki Performance section, probe skill,
      AGENTS.md, capture-crate docs, .gitignore comments, CHANGELOG
      Unreleased); historical task records untouched. Re-grep
      perf-baseline.sh|perf-web.sh|perf-profile.sh|run_report|perf_report|
      perf_trace to confirm only history remains.
- [x] Tests: parse pins for the new flags (matrix accumulation, --render
      env, --release path, --platform web rejection of native-only flags);
      report-gate pin (manifest-less dir refused); trace verb pin; alias
      forwarding pin; env fns for the sweep cells (scenario/quality/label
      per cell).
- [x] E2E: (a) probe report over a probe-produced dir renders + a foreign
      dir is refused; (b) a small real gpu sweep (1 scenario x 2 presets,
      short frames) through `probe run 20_perf_baseline --fps --release
      --scenario asteroid_field --preset high --preset low` producing
      labeled rows + report; (c) a real `--platform web` capture (the
      validation gate for deleting perf-web.sh). Record all three here.
- [x] Verify: fmt; cargo test -p nova_probe; workspace all-targets
      --features debug; wasm check; final reference re-grep.

## Notes

- Depends on 20260719-174541 (hardening): fresh-dir semantics + the
  probe-run.json manifest are the foundation for the report gate.
- Spike: tasks/20260719-112011/SPIKE.md (T6 recorded script-wrapping as the
  interim with this consolidation as the named revisit; user direction
  2026-07-19: one verb, scripts gone, reports only from probe-produced
  dirs).
- NON-GOAL: renaming the NOVA_PERF_* env surface - once the CLI is the only
  user surface, the env vars are plugin-arming internals; renaming them
  churns capture/recorder/invariants + docs for no user-visible gain.

## Close-out (2026-07-19, branch refactor/probe-consolidation)

One bin, two passes of e2e validation, three scripts and three bins retired.

E2E evidence (all real runs, recorded before the deletions they gate):
- (a) Report gate: a manifest-less dir is REFUSED with the probe-run
  guidance, exit 1; `probe report` over a probe-produced sweep dir renders.
- (b) Sweep: `probe run 20_perf_baseline --fps --release --scenario
  asteroid_field --preset high --preset low` produced correctly-labeled
  release rows (asteroid_field-high 16.6 ms / -low 17.1 ms at 60 frames)
  and, after the first-round fixes, verdict OK measured 2/6 with
  process_exit "2 pass(es), all clean exits".
- (c) Web: `probe run asteroid_field --platform web` - trunk build,
  embedded static server, calibrated Chromium flags - scraped
  asteroid_field-high-web 600 frames mean 29.4 ms (consistent with the
  v0.7.0 web baseline) into a v2 CSV row; verdict OK, exit 0.

The first e2e round found three REAL composition gaps, each fixed + pinned:
chromium wraps the console line in %c style markers with trailing junk
(parser now stops at the first non-key=value token; the live line is the
test fixture); sweep manifests name passes "clean <label>" so process_exit
now judges ALL primary passes; sweep logs are run-<n>.log so the loader
concatenates them (web-run.log stays out - chromium noise is not the game
log). A scrape-parse failure now degrades to a failed pass instead of
aborting the report (the hardening's own rule, applied to the new path).

Also swept: the dead perf_report renderer left in report.rs (render_report/
read_runs + 5 tests whose coverage lives in run_report's), the orphaned
mini* fixtures, and the report's own skip-messages that still recommended
deleted scripts. Adapter name in the web row is "unknown" this run
(chromium did not log AdapterInfo in the scraped window) - the parse path
exists and degrades honestly; noted, not blocking.
