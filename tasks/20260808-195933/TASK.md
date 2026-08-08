# Make standard nova_probe_cli diagnostics automatic

- STATUS: OPEN
- PRIORITY: 0
- TAGS: v0.10.0,tooling,perf,refactor

## Story

As a probe user, I want `nova_probe_cli run` to collect the standard diagnostics without opt-in flags, so the example wiring defines what the report measures.

An example that wires the frame-time plugin already declares its frame-time capability. Requiring `--fps` adds a second operator decision and creates a 2x2 report state: declared or undeclared x requested or not requested. Remove that flag axis. The report should show the two useful states: the example declares the capability and probe measures it, or the example does not declare it and the result is N/A.

Apply the same default-on rule to the other normal probe passes. In particular, run the trace/profile pass without `--profile`. Keep capability-sensitive behavior driven by the runtime probe contract where applicable. Remove the obsolete flags, manifest state, report branches, help text, tests, docs, and command examples.

Keep `--samply` as an explicit opt-in. It takes substantially longer and is outside this cleanup.

## Steps

- [x] Delete `RunOptions::fps` and `RunOptions::profile`, their defaults, and
      the `--fps` and `--profile` parser branches in
      `crates/nova_probe_cli/src/native/cli.rs`. Use the resulting compiler
      errors to find every control-flow dependency. Remove obsolete matrix and
      web combination gates instead of replacing them with negative flags.
- [x] Refactor `crates/nova_probe_cli/src/native/run.rs` around the execution
      rule in `NOTES.md`: run the clean native pass, read its
      `probe-contract.json`, and launch the separate frame-time pass if and
      only if it declares `Capability::FrameTime`. Apply the same declaration
      rule to matrix and web paths. An undeclared capability is N/A, not an
      error. Preserve clean measurement boundaries and window-sized deadlines.
- [x] Make the separate native trace build and run unconditional. Keep
      `--samply` and its profiling build as the only optional diagnostic pass.
      Update pass counts, progress text, and per-pass records to describe what
      executed rather than what the removed flags requested.
- [x] Collapse the evaluation handshake in
      `crates/nova_probe_cli/src/evaluation/{manifest,artifacts}.rs` and the
      checks/report consumers to the two current-run outcomes from `NOTES.md`:
      declared and measured, or undeclared and N/A. Keep old
      `probe-run.json` directories readable. A declared and scheduled
      capability with a missing artifact remains a failure.
- [x] Update CLI, orchestration, manifest, check, and HTML report tests. Cover
      removed-flag rejection, default trace execution state, declared
      frame-time collection, undeclared frame-time N/A, matrix capability
      handling, legacy manifest parsing, and explicit `--samply` behavior.
- [x] Remove obsolete flag language and command examples from the crate docs,
      root `Cargo.toml`, `README.md`, `CHANGELOG.md`, affected examples, and
      `web/src/wiki/dev/development.md`. State that program wiring controls
      frame-time collection and native trace collection is automatic.

## Definition of Done

- `--fps` and `--profile` are rejected as unknown flags, while `--samply`
  remains accepted. (test: `native::cli::tests::removed_diagnostic_flags_are_rejected`)
- A default native run records clean and traced passes, and records a separate
  frame-time pass only when the clean contract declares `frametime`.
  (test: `native::run::tests::default_passes_follow_the_runtime_contract`)
- Normal and matrix runs without a frame-time declaration resolve frame-time
  checks as N/A without failing for the missing capability.
  (test: `evaluation::checks::fps_within_baseline::tests::undeclared_frame_time_is_not_applicable`)
- A declared and armed frame-time capability with no artifact fails instead of
  becoming N/A. (test: `evaluation::checks::fps_within_baseline::tests::declared_frame_time_without_output_fails`)
- Existing manifests with `armed.fps` still load and render.
  (test: `evaluation::manifest::tests::legacy_armed_fps_manifest_loads`)
- The touched crate tests pass.
  (cmd: `nix develop --command cargo test --lib -p nova_probe_cli`)
- Formatting and the compiler-assisted refactor are clean.
  (cmd: `nix develop --command cargo fmt --all --check`)
- Shipped documentation and website references are valid.
  (cmd: `cd web && npm run ci`)
- A default run of one example with `NovaProbePlugin` completes without the
  removed flags and produces `frametime.csv`, `trace.json`, and a graded
  report. (cmd: `nix develop --command cargo run -p nova_probe_cli -- run player_path`)
- The generated report shows frame-time evidence without an opt-in state and
  presents undeclared frame time as N/A. (human: open representative reports
  for declared and undeclared examples and inspect the frame-time and trace
  sections)

## Notes

- Preserve separate clean, frame-time, and traced runs where required to prevent instrumentation from contaminating measurements.
- Matrix and web behavior must remain coherent after flag removal.
- The compiler-assisted refactor exposed a release-run bug in the old optional
  trace path: it built `target/debug/examples/<example>` with tracing but ran
  the clean pass's `target/release` binary. The automatic trace pass now names
  the traced debug binary directly.
- A missing or malformed clean contract does not authorize frame-time capture.
  The artifact loader retains the failure and the report grades it.
- Verification:
  - `nix develop --command cargo test --lib -p nova_probe_cli` - 98 passed.
  - `nix develop --command cargo fmt --all --check` - passed.
  - `nix develop --command bash -c 'cd web && npm run ci'` - passed.
  - `nix develop --command cargo run -p nova_probe_cli -- run player_path` -
    aggregate OK; clean, fps, and profiled passes succeeded; `frametime.csv`,
    `trace.json`, and `report.html` present under
    `probe-runs/c2dde47d/player_path/`.
- Human report review remains open by definition.
- Review fixes:
  - `post_clean_passes` is the production scheduling source for frame time,
    profiling, and samply. Its test covers undeclared, declared, matrix, and
    samply plans; the exhaustive executor records each attempted pass.
  - v0.8.0 changelog history is restored. The automatic behavior is recorded
    under `[Unreleased]`.
  - The wiki category table now states the runtime-contract rule for every
    category.
  - Post-fix `player_path` run: aggregate OK with clean, fps, and profiled
    passes under `/tmp/nova-work-20260808-final/c2dde47d/player_path/`.
