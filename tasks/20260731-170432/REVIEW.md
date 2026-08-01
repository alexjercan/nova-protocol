# Review: KISS: nova_probe run harness

- TASK: 20260731-170432
- BRANCH: refactor/kiss-nova-probe

## Round 1

- REVIEWER: in-session (the session forbids spawning subagents, so the
  default out-of-context reader was unavailable; compensated by deriving the
  no-behavior-change claim from the two trees rather than from the
  implementer's summary - see Verified below)
- VERDICT: APPROVE

- [x] R1.1 (MINOR) crates/nova_probe/src/bin/probe/native/cli.rs:104 - the
  split promoted every top-level item in the bin to `pub(crate)`, including
  ~20 that no other module references: `default_run`, `parse_run` (cli.rs),
  `NON_PERF_WARMUP`, `NON_PERF_FRAMES`, `FPS_FLOOR`, `FPS_LOAD_MARGIN_SECS`,
  `env_u32`, `resolve_fps_window`, `fps_deadline_secs` (env.rs),
  `is_hash_dir_name`, `discover_baseline_root` (paths.rs), `report_aggregate`
  (report.rs), `RUN_ARTIFACTS`, `clean_out_dir`, `finish_report`,
  `passes_total` (run.rs), `spec_help` (spec.rs), `display_candidates`
  (supervise.rs), `run_many` (sweep.rs), `serve_dir` (web.rs). That blurs the
  module interfaces the split just drew. Drop `pub(crate)` on those items -
  they stay reachable from their own `mod tests`. Keep it on `XvfbGuard` and
  `RunOutcome`: both appear in the signatures of `pub(crate)` fns, so
  narrowing them would trip `private_interfaces`.
  - Response: dropped `pub(crate)` on all 20; `XvfbGuard` and `RunOutcome`
    kept as called out. Commit 4th on the branch.

### Verified

- `cargo check --workspace --all-targets` green; `cargo fmt --check` exits 0.
- `cargo test -p nova_probe --lib --bins`: 71 + 26 + 0 tests, all pass.
- No behavior change, derived independently of the close-out: a
  comment-stripped, whitespace-normalized, sorted line multiset over
  `crates/nova_probe/**/*.rs` differs between master and the branch ONLY in
  `mod`/`use`/`pub use` lines, visibility keywords, rustfmt re-wrapping caused
  by the reduced indentation, and the comment edits. No statement was added,
  dropped or reordered.
- The `#[test]` name sets on master and the branch are identical (empty diff
  of the sorted lists), so no test was dropped, renamed or weakened.
- Public API is intact: every `pub` item the old `run_report.rs` exported is
  re-exported by `run_report/mod.rs` (set difference empty), and `lib.rs`
  keeps its `pub use` block unchanged. The new `pub fn`s all sit inside
  `#[cfg(test)] mod fixtures`, so they are not API.
- End-to-end smoke on the relocated bin target (`src/bin/probe/main.rs`):
  `probe report /nonexistent-dir` prints the manifest-gate error and exits 1;
  `probe frobnicate` prints `unknown subcommand` plus the full usage and exits
  1. Both `Cmd` dispatch arms work from the new path.
- DoD 3: `grep -rnE '//.*[0-9]{8}-[0-9]{6}' crates/nova_probe/` returns zero
  hits, so no NOTES.md exception list is owed.
- DoD 4: largest file is `run_report/checks.rs` at 913 lines; every file in
  the crate is under the 1500 ceiling.
- Doc surface: no reference to `bin/probe.rs` or `run_report.rs` exists in
  AGENTS.md, `web/`, `docs/`, `scripts/`, `.github/`, README or CHANGELOG -
  the wiki names the crate and its commands, not its files, so nothing went
  stale. (`tasks/` mentions are append-only history and exempt.)
- Honesty: every number in TASK.md's close-out was re-run here and matches.
- Comment rubric: the surviving non-doc comments each state a constraint or a
  why; provenance clauses ("task <id>", "finding N", "review R1.x") are gone
  and the load-bearing ones were promoted to `NOTE:`.

### Pending user checks

- DoD 6 (`manual:`) - owner skims the diff and agrees no behavior changed.

## Round 2

- REVIEWER: in-session (same exception as Round 1; verification of a
  visibility-only change is mechanical and was re-derived from the tree)
- VERDICT: APPROVE

R1.1 confirmed fixed. Every one of the 20 items named is now module-private
(the `pub(crate)` count over `native/` drops from 59 to 39), `XvfbGuard` and
`RunOutcome` still carry `pub(crate)` as required by the signatures they
appear in, and no item that crosses a module boundary was narrowed:
`cargo check -p nova_probe --all-targets` is green with no new warning,
`cargo fmt --check` exits 0, and the same 71 + 26 tests pass. No new
findings.

### Pending user checks

- DoD 6 (`manual:`) - owner skims the diff and agrees no behavior changed.
