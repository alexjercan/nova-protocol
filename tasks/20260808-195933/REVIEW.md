# Review

## Findings

- [x] MAJOR `crates/nova_probe_cli/src/native/run.rs:423` -
  `default_passes_follow_the_runtime_contract` only tests that
  `declares_frametime` parses two contract files. It does not observe the pass
  schedule named by the test or the DoD: deleting the unconditional trace
  block, running frame time unconditionally, or omitting the `fps`/`profiled`
  manifest records would leave this test green. Add a behavior-boundary test
  around pass planning or an injected runner that asserts clean + profiled for
  an undeclared contract, clean + fps + profiled for a declared contract, and
  optional samply without restoring flag state.
- [x] MAJOR `CHANGELOG.md:310` - The change rewrites the v0.8.0 release entry
  to claim traces and frame-time scheduling were automatic in that release.
  v0.8.0 shipped the `--profile` and `--fps` behavior described by the old
  lines. Restore the historical entries and add one short Internals & Tooling
  entry under `[Unreleased]` for the v0.10.0 behavior.
- [x] MINOR `web/src/wiki/dev/development.md:132` - The category table still
  says `sections/`, `systems/`, and `ui/` receive correctness passes only, but
  examples in all three categories wire `nova_frametime()` and now receive the
  automatic frame-time pass. Make the column capability-driven, or state the
  actual per-category behavior without contradicting the runtime-contract
  explanation at lines 138-142.

## Verification

- `nix develop --command cargo test --lib -p nova_probe_cli` - 97 passed.
- `nix develop --command cargo fmt --all --check` - passed.
- `nix develop --command bash -c 'cd web && npm run ci'` - passed.
- `nix develop --command cargo run -p nova_probe_cli -- run player_path --out
  /tmp/nova-review-20260808-declared` - aggregate OK; manifest records clean,
  fps, and profiled; frame-time and trace artifacts exist.
- `nix develop --command cargo run -p nova_probe_cli -- run render_scale_shot
  --out /tmp/nova-review-20260808-undeclared` - aggregate OK; no fps pass;
  profiled pass exists; frame-time check is N/A `not claimed`.
- Human proof remains pending: inspect representative declared and undeclared
  `report.html` files.
- Re-review: `post_clean_passes` drives the exhaustive executor for
  undeclared, declared, matrix, and samply schedules. Every arm returns one
  `PassRecord`, appended at the single loop boundary.
- Re-review: the changelog diff adds only the new `[Unreleased]` entry; v0.8.0
  history is unchanged.
- Re-review: the wiki table applies the runtime-contract and automatic-trace
  rule to every category.
- Re-review: 98 crate tests, formatting, and website CI pass. Final
  `player_path` artifacts under
  `/tmp/nova-work-20260808-final/c2dde47d/player_path/` contain successful
  clean, fps, and profiled records plus frame-time and trace output.

## Verdict

APPROVE
