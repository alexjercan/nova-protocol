# Example categories: write the contract and resolve probe run policy from it

- PRIORITY: 85
- TAGS: v0.10.0, tooling, examples, testing
- KIND: STORY
- ACTIVITY: PLANNING
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
- [ ] Rename the report manifest's `fps_exempt: Option<String>` field to
      `fps_skipped` (`crates/nova_probe/src/run_report/manifest.rs:31-34,73,125`,
      rendered at `html.rs:181-193`). It already IS a reason string, not a list
      membership - only its SOURCE changes, from the Cargo.toml exempt list to
      the category policy ("category `sections/` carries no frame-time pass").
      Keep it `Option<String>`; no schema shape change, no compatibility shim.
- [ ] Update `tests/examples_smoke.rs` for the new taxonomy: the per-category
      const lists (`SECTIONS:32`, `GAMEPLAY:43`, `UI`, `SCREENSHOTS`) become
      `SECTIONS`/`SYSTEMS`/`STRESS`/`UI`/`SCREENSHOTS`, add the
      `<category>_reach_playing_without_panic` tests for the new categories,
      and re-justify each `NOT_SMOKED:78` entry.
- [ ] Sweep the 10 `web/src/wiki/dev/` pages naming `gameplay/`,
      `examples/perf`, `perf_baseline` or `broadside`, per
      `web/src/wiki/dev/keeping-docs-in-sync.md`.

## Definition of Done

- Probe resolves run policy per category, with `screenshots/` excluded from
  `--all`. (test: `category_run_policy_selects_passes_per_category`)
- The catalog, the on-disk layout and the smoke lists agree after the rename.
  (test: `catalog_matches_disk`)
- No doc or manifest still names a retired category.
  (cmd: `! rg -n 'examples/perf|examples/gameplay|fps_exempt' Cargo.toml web/src/wiki`)

## Notes

Owner principle 2026-08-04: **do not add tests for our tests. Examples should
NOT be tested - they ARE tests.** `catalog_examples_satisfy_their_category_contract`
is dropped: it would have inspected example SOURCE to judge whether a run
asserts enough, which is testing a test. The contract lives in the catalog
comments and the dev wiki page, and review enforces it.

Two existing tests are NOT affected, because neither judges an example's
content:

- `category_run_policy_selects_passes_per_category` (new, this task) tests
  `nova_probe` - production code with real branching.
- `catalog_matches_disk` (existing) is a BUILD-INTEGRITY gate: with
  `autoexamples = false`, an example missing from the catalog does not compile
  at all, and this is what catches that. It pins names and paths, never
  behavior.

Flag if that line lands differently than intended - it is my reading of the
principle, not something you said explicitly.

- Category strings are load-bearing in `crates/nova_probe/src/bin/probe/native/env.rs:65`
  (`if category == "perf"` selects the fps window) and in the test fixtures in
  `catalog.rs`, `aggregate.rs`, `spec.rs`, `fixtures.rs`. No CI workflow names
  a category.
- `fps_exempt` splits in three, and each piece has one owner:
  - the Cargo.toml key `fps_exempt = ["broadside"]` (:34-35) - deleted by
    whichever of `20260804-093910` / `20260804-094006` lands second;
  - the parsers `parse_fps_exempt` / `load_fps_exempt` (`catalog.rs:127-176`)
    plus their four unit tests and the `lib.rs:147` re-exports - deleted HERE,
    since the category policy replaces them;
  - the checks.json field - RENAMED here to `fps_skipped`, kept as the reason
    string it already is.
- `screenshots/` stops being probe's problem: excluded from `--all`.
- Contract table and per-category probe behavior: see SPIKE.md of
  `20260804-003244`.
