# nova_probe consolidation: one verb (run absorbs sweep/web/profile), manifest-gated report, delete the .sh scripts

- STATUS: OPEN
- PRIORITY: 66
- TAGS: v0.8.0,tooling,refactor

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
