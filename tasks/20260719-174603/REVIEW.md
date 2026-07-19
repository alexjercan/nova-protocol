# Review: probe consolidation (one front door)

- TASK: 20260719-174603
- BRANCH: refactor/probe-consolidation

## Round 1

- VERDICT: APPROVE

Shared-session caveat: implementer and reviewer are one session; the
load-bearing claims were validated by LIVE runs, and the deletions were
each gated on their validation:

- **The deletion gates were honored in order**: perf-baseline.sh died only
  after a real release sweep produced correctly-labeled rows
  (asteroid_field-high 16.6 ms / -low 17.1 ms); perf-web.sh died only
  after a real web capture scraped 29.4 ms mean webgpu - consistent with
  the v0.7.0 baseline's 34-39 ms band on a lighter run; perf-profile.sh's
  coverage was already proven in T4/T6. The task's "script stays if not
  validated" fallback was never needed, but it existed.
- **The Chromium flag folklore survived verbatim**: diffed the ported flag
  array against the deleted script's CHROME_FLAGS - identical, and the
  provenance comment stays. The scrape parser's test fixture is the LIVE
  chromium line captured this session, not a synthetic one.
- **The e2e round did its job**: three composition gaps (console-line
  wrapping, sweep pass naming vs process_exit, per-cell logs) were found
  by real runs, fixed, pinned, and re-validated - none were reachable by
  unit tests alone. The scrape-failure path now degrades per the
  hardening's own outcome-not-abort rule.
- **The gate closes the user's original concern**: probe report refuses
  manifest-less dirs (proven, exit 1 with guidance); combined with fresh
  dirs + the manifest, a report can no longer be built from stale or
  hand-assembled data.
- **Dead code went with the bins**: report.rs's orphaned renderer +
  fixtures removed; their behavioral coverage (v1 back-compat, metadata
  display) verified still pinned in stats/run_report tests before
  deletion - not silently dropped.
- **Sweep hygiene**: the final reference grep shows only historical task
  records and deliberate provenance comments; the CHANGELOG's Unreleased
  section was consolidated to describe the FINAL surface (editable until
  release - the old bullets advertised bins that no longer exist).
- **Checks**: 64 tests green; workspace all-targets + wasm clean.

Findings:

- R1.1 (NIT) - the web row's adapter field read "unknown" in the
  validation run (chromium did not log AdapterInfo inside the scraped
  window); the parse path exists and degrades honestly. Revisit only if
  adapter identity becomes load-bearing for web baselines. Left as-is.
- R1.2 (NIT) - `probe run <scenario> --platform web` overloads the
  positional (example name natively, scenario id on web); documented in
  usage and the wiki, and the honest-combination gate rejects confusions.
  A future `--scenario` unification could tidy it; not blocking.
