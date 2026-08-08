# nova_probe: grow + rename nova_perf into the run-harness crate (frame-capture + perf_report as modules)

- STATUS: CLOSED
- PRIORITY: 76
- TAGS: v0.8.0, spike, tooling, refactor, performance

## Goal

Grow `nova_perf` into the unified run-harness crate and rename it to
`nova_probe` (name confirmed by user, 2026-07-19). The existing frame-time
capture harness and the `perf_report` HTML generator become MODULES of the new
crate; nothing about the measurement is lost. Also extend the capture schema
with run metadata - renderer/GPU, resolution, graphics preset, git SHA, host
class - today the renderer is inferred from the results dir NAME only, and
baseline deltas / per-renderer thresholds / the report's Run summary all need
it (spike review m2). This is the foundation the correctness, profiling, and
report tasks build on.

## Steps

- [x] Rename the crate: `git mv crates/nova_perf crates/nova_probe`, package
      name -> `nova_probe`, root Cargo.toml (dev-dependency + workspace
      member), Cargo.lock refresh. Bin names (`perf_web`, `perf_report`), the
      `NOVA_PERF_*` env vars and the `nova perf:` log line (scraped by
      perf-web.sh:76) STAY - they are the measurement surface T6 redesigns;
      this task renames the crate identity only.
- [x] Restructure the lib into run-harness modules: `src/capture.rs` (the
      env-gated FrameTimePlugin, config, drivers, perf_param), `src/stats.rs`
      (FrameStats + RunMeta + CSV/JSON schema + parsers), `src/report.rs`
      (the HTML rendering, moved out of the bin), `src/lib.rs` (crate docs +
      re-exports so dependents keep flat paths); bins become thin wrappers.
- [x] Extend the capture schema with run metadata (spike review m2): RunMeta
      { backend, adapter, resolution, quality, git_sha, host }. backend +
      adapter read from `Option<Res<RenderAdapterInfo>>` in the MAIN world
      (verified: bevy_render-0.19.0/src/settings.rs:197 inserts it there);
      resolution from the config; quality via perf_param("quality");
      git_sha: NOVA_PERF_SHA override else `git rev-parse --short HEAD`
      (native, degrades to "unknown"); host: NOVA_PERF_HOST else
      /etc/hostname else "unknown" ("browser" on wasm). New columns join the
      CSV (v2 header, values comma-sanitized) and the per-run JSON.
- [x] Parser back-compat: accept BOTH the v1 header (meta -> unknown; the
      v0.7.0 baseline in tasks/20260716-123551 must keep parsing) and v2;
      the report shows per-run renderer from row metadata when present and
      falls back to the results-dir name for v1 data.
- [x] Sweep the live surfaces to nova_probe: root Cargo.toml, the
      20_perf_baseline example, perf.html (Trunk href), perf-baseline.sh,
      wiki development.md, AGENTS.md crate table, the live plan doc
      (docs/plans/20260718-v0.8.0-plan.md) and the queued task specs
      (152205/152214/152304). Historical records (closed task folders, the
      spike/review) stay untouched. Re-grep both `nova_perf` and `nova perf`
      to confirm only historical + kept-contract refs remain.
- [x] CHANGELOG (Unreleased > Internals & Tooling): one entry covering the
      perf_report HTML bin (missed at f4bfb3af) + the nova_probe rename +
      the schema metadata.
- [x] Tests: existing 17 move with the modules; add v2 roundtrip with
      metadata, v1 back-compat parse, comma-sanitization of meta values,
      report-prefers-row-metadata; add a v2 fixture dir next to the v1 ones.
- [x] Verify: cargo fmt; cargo check --workspace --all-targets; cargo test
      -p nova_probe (moved + new tests); cargo check --target
      wasm32-unknown-unknown -p nova_probe (the wasm cfg paths); final grep
      sweep clean.

## Notes

- Spike: tasks/20260719-112011/SPIKE.md (foundation task, do FIRST).
- Move `crates/nova_perf/*` under the new name; update every dependent
  (`20_perf_baseline` example, `perf_web` bin, the two perf scripts, Cargo
  members, `perf.html`/Trunk wiring). Sweep the repo for the old crate name.
- Keep the `nova_scenario` criterion bench separate - not part of this crate.
- The built `perf_report` code (originally branch `feature/perf-html-report`,
  commit a9af7789) lands on master with the spike-branch squash; incorporate
  it under the new crate rather than re-writing.

## Close-out (2026-07-19, branch refactor/nova-probe-rename)

What shipped: `crates/nova_probe` (formerly nova_perf) split into
`capture.rs` / `stats.rs` / `report.rs` with a re-exporting lib root; bins
thinned (report rendering now lives in the lib for T5 to grow); CSV/JSON
schema v2 with RunMeta (backend, adapter, resolution, quality, git_sha,
host) resolved at emit time from the main-world RenderAdapterInfo + env/fs
fallbacks; parser accepts v1 AND v2 (the v0.7.0 baseline still loads, all
meta "unknown"); the report prefers row metadata over the dir-name
convention. 24 tests green (22 lib + 1 bin + 1 doc); workspace
--all-targets --features debug check clean; wasm32 check clean; end-to-end
render verified over the real v0.7.0 xgpu results (v1 fallback: renderer
"xgpu") and the v2 fixture (renderer "vulkan (llvmpipe ...)", git SHA
shown).

Decisions and alternatives:

- Kept `NOVA_PERF_*` env vars, the bin names and the `nova perf:` log
  prefix: they are the measurement surface (perf-web.sh scrapes the log
  line at line 76) and T6 redesigns that surface wholesale; renaming them
  now would churn scripts + docs twice. The crate rename is identity only.
- Metadata rides IN the CSV (6 sanitized columns) rather than only in the
  per-run JSON: the report reads only frametime.csv, and keeping it that
  way avoided adding a serde dependency for JSON parsing (the hand-rolled
  writer stays, gaining the same fields).
- Strict row-width validation per header version (a v1-shaped row under a
  v2 header is an error, not a silent meta-default) so corrupt/mixed files
  fail loudly.

Difficulties: none material. The one real risk - whether RenderAdapterInfo
exists in the MAIN world - was settled by reading bevy_render-0.19.0
source (settings.rs:197) before writing the system, per
verify-engine-guarantees-in-source; it does, and Option<Res<...>> degrades
--norender builds to "unknown".

Reflection: the sweep discipline paid off - grepping both `nova_perf` and
the prose form `nova perf` surfaced the perf-web.sh scrape contract BEFORE
the rename could break it, and the live-vs-historical split kept closed
task records intact. Writing Steps with the verifying citations already in
them (bevy source line, scraper line) made implementation mechanical.
