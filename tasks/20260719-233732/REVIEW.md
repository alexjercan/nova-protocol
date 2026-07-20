# Review: frame-capture fps-exempt + category window defaults

- TASK: 20260719-233732 (re-scoped 2026-07-20)
- BRANCH: fix/probe-partial-fps

## Round 1

- VERDICT: APPROVE

Focused diff (nova_probe only + Cargo.toml metadata + docs), all new unit tests
green, `cargo check --all-targets` clean. Implementer == reviewer, so I
re-derived the load-bearing claims against the code rather than trusting the
summary:

- **"Skipping the fps pass makes process_exit PASS."** Re-read process_exit
  (run_report.rs:400): primary passes are filtered to `clean`/`web`/`fps`. When
  the example is exempt, no `fps` PassRecord is pushed, so primary = `[clean]`;
  broadside's clean pass succeeds (user field data: clean=ok), so failed is
  empty -> PASS. The FAIL the user saw came purely from the timed-out fps pass,
  which no longer runs. Verified.
- **"Operator env always wins for the window."** `run_supervised` builds the
  child with `.envs(...)` and NO `env_clear`, so the child inherits probe's
  environment. `fps_window_env` pushes the 60/240 default only when
  `std::env::var_os` shows the operator did NOT set it; when they did, probe
  pushes nothing and the child inherits the operator's value. Verified both
  branches (unit test `fps_window_env_defaults_non_perf_to_the_short_window`
  guards on the same env absence).
- **"The Cargo.toml metadata block cannot confuse the catalog parser."** Traced
  `parse_example_catalog` over the new manifest: `[package.metadata.nova_probe]`
  is a `[`-line -> flushes and sets `in_example = false`; the `fps_exempt = [..]`
  line is then skipped by the `if !in_example { continue }` guard. The existing
  catalog unit tests still pass, and the display-free `catalog_matches_disk`
  drift test is run to confirm (see the task's verification log).
- **"Exempt but no --fps shows the normal line, not an exempt note."** The
  native finish_report call passes `fps_exempt.filter(|_| opts.fps && !sweeping)`,
  so a plain `probe run broadside` reports the generic no-capture line; the
  exempt note appears only when `--fps` was actually requested.

Scope honesty: the re-scope (crossing out partial-emit + yield-on-primary-done)
is backed by reproduce-first evidence recorded in TASK.md - the loop work
already fills windows for the cycling examples, so those pieces had no remaining
caller. This is a legitimate falsification-driven shrink, not a corner cut.

- [x] R1.1 (NIT) probe.rs:1531 `passes_total` counts the fps pass whenever
  `opts.fps && !sweeping`, even for an exempt example whose fps pass is
  skipped, so the profiled/samply progress labels read e.g. `[2/3]` when only
  2 passes run. Cosmetic (stderr progress only; the skip is separately
  announced with its own line, and the report/verdict are unaffected). Fix
  would thread the exempt flag into `passes_total`. Take it or leave it.
  - Response: Fixed - `passes_total(opts, fps_exempt.is_some())` drops the fps
    pass from the count when exempt, so the labels stay honest.

- [x] R1.2 (NIT) The manifest `fps_exempt` json round-trip is covered
  indirectly (the `manifest_round_trips_and_drives_process_exit` test now
  carries `fps_exempt: Some(...)` and asserts `parsed == manifest`), which is
  enough, but a one-line explicit assertion on the parsed field would document
  intent. Optional.
  - Response: Left as-is - the `parsed == manifest` equality already fails if
    the field is dropped in either direction, so an extra assertion is
    redundant. Acknowledged, not actioned.
