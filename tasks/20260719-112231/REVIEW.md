# Review: nova_probe rename + module split + run-metadata schema

- TASK: 20260719-112231
- BRANCH: refactor/nova-probe-rename

## Round 1

- VERDICT: APPROVE (one MINOR, addressed in-round; no BLOCKER/MAJOR)

Shared-session caveat: implementer and reviewer are one session, so the
load-bearing claims were independently RE-DERIVED, not read off the diff:

- **Schema column contract** (the crux - a silent order mismatch would swap
  fields): extracted the 17 CSV_HEADER fields, the `csv_cells()` writer
  order, and the `from_csv_row` reader indices 11..16 mechanically from the
  source and cross-checked - all three agree (backend, adapter, resolution,
  quality, git_sha, host).
- **v1 back-compat**: `perf_report` executed against the REAL v0.7.0
  baseline (`tasks/20260716-123551/perf-results/xgpu` + sw baseline): 6 runs
  render, renderer falls back to the dir name ("xgpu"), deltas populate.
  The v2 fixture renders its own metadata ("vulkan (llvmpipe ...)", git
  SHA) and the header does NOT show the dir name.
- **RenderAdapterInfo in the main world**: verified in the dependency's
  source (bevy_render-0.19.0/src/settings.rs:197,
  `main_world.insert_resource(adapter_info.clone())`), not from docs.
- **Test survival**: all 12 original test names still present; the 5
  original CSV tests were reshaped into v1/v2 variants, none weakened. The
  one changed assertion (truncated row now errors "header promises N")
  matches the new strict width check, which is itself pinned: without it an
  11-column row under a v2 header would silently parse with default meta -
  `parse_frametime_csv_rejects_a_row_width_mismatch` fails if the check is
  deleted. `v2_report_prefers_the_rows_own_metadata` fails if the report
  ignores metadata; `v1_report_falls_back_to_the_dir_name_renderer` fails
  if the fallback is dropped.
- **Checks**: 24 tests green (22 lib + 1 bin + 1 doc);
  `cargo check --workspace --all-targets --features debug` clean;
  `cargo check --target wasm32-unknown-unknown -p nova_probe` clean; fmt
  clean; only pre-existing proc-macro-error2 future-incompat notice.
- **Sweep**: remaining `nova_perf` references are exclusively historical
  records (closed task folders, the spike/review, this task's own rename
  narrative) plus the deliberate `NOVA_PERF_*` env surface; the
  `nova perf:` log-line scrape contract in perf-web.sh:76 is intact and now
  documented as a contract at its definition (stats.rs summary_line).

Findings:

- [x] R1.1 (MINOR) web/src/wiki/dev/development.md (Performance section) -
  the newly recorded run metadata (backend/adapter/resolution/preset/SHA/
  host in every capture, shown by the report) is a user-visible feature of
  the tooling but the wiki section does not mention it; add one line so the
  docs match the report readers will actually see.
  - Response: fixed in-round - added a sentence to the Performance section
    naming the recorded fields and the v1 fallback.

- R1.2 (NIT) crates/nova_probe/src/stats.rs json_safe - control characters
  in env-sourced strings (host, sha overrides) are not escaped in the JSON
  writer. Dev-only tool, inputs are operator-controlled; take it or leave
  it. Left as-is; noted for T5 if the report grows a JSON sidecar
  (checks.json), where a real serializer is warranted anyway.
