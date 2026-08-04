# Example categories: write the contract and resolve probe run policy from it

- PRIORITY: 84
- TAGS: v0.10.0,tooling,examples,testing
- KIND: STORY
- ACTIVITY: -
- GATES: -
- RESOLUTION: -
- PARENT: 20260802-115955
- DEPENDS ON: 20260804-003244

## Story

Give every example category an explicit contract and teach `nova_probe` to
resolve its run policy from that contract, replacing the `perf`-vs-everything
split plus the hand-listed `fps_exempt`.

The roster spike (`20260804-003244`) settled five categories: `sections/`,
`systems/`, `stress/`, `ui/`, `screenshots/`. `gameplay/` and `perf/` are
retired as directory names - `gameplay/` was never a contract, and every
frame-time claim now lives in `stress/`.

This is code, not content. Every other task in the rebuild depends on it.

## Steps

- [ ] Write the category contract down: the root `Cargo.toml` catalog comment
      AND the dev wiki example page - what each category proves, what probe
      does with it, what disqualifies an example from it.
- [ ] Teach `nova_probe` a per-category run policy replacing the
      `perf`-vs-everything split and the hand-listed `fps_exempt`: categories
      declare whether they run correctness passes, frame-time passes or
      neither, and `--all`/category expansion honors it. `screenshots/` leaves
      probe's scope.
- [ ] Update `tests/examples_smoke.rs` for the new taxonomy: the per-category
      const lists (`SECTIONS:32`, `GAMEPLAY:43`, `UI`, `SCREENSHOTS`) become
      `SECTIONS`/`SYSTEMS`/`STRESS`/`UI`/`SCREENSHOTS`, add the
      `<category>_reach_playing_without_panic` tests for the new categories,
      and re-justify each `NOT_SMOKED:78` entry.
- [ ] Sweep the 10 `web/src/wiki/dev/` pages naming `gameplay/`,
      `examples/perf`, `perf_baseline` or `broadside`, per
      `web/src/wiki/dev/keeping-docs-in-sync.md`.

## Definition of Done

- Every cataloged example declares a category whose contract it satisfies; a
  mismatch (a `screenshots/` run enrolled in fps, a `sections/` run with no
  assertion) fails the catalog test.
  (test: `catalog_examples_satisfy_their_category_contract`)
- Probe resolves run policy per category, with `screenshots/` excluded from
  `--all`. (test: `category_run_policy_selects_passes_per_category`)
- The catalog, the on-disk layout and the smoke lists agree after the rename.
  (test: `catalog_matches_disk`)
- No doc or manifest still names a retired category.
  (cmd: `! rg -n 'examples/perf|examples/gameplay|fps_exempt' Cargo.toml web/src/wiki`)

## Notes

- Category strings are load-bearing in `crates/nova_probe/src/bin/probe/native/env.rs:65`
  (`if category == "perf"` selects the fps window) and in the test fixtures in
  `catalog.rs`, `aggregate.rs`, `spec.rs`, `fixtures.rs`. No CI workflow names
  a category.
- `fps_exempt = ["broadside"]` (Cargo.toml:35) is deleted here or by the
  retire task, whichever lands second.
- `screenshots/` stops being probe's problem: excluded from `--all`.
- Contract table and per-category probe behavior: see SPIKE.md of
  `20260804-003244`.
