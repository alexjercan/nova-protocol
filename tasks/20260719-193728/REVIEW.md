# Review: examples reorg (purpose dirs + slug names)

- TASK: 20260719-193728
- BRANCH: refactor/examples-reorg
- ROUND: 1

## What I tried to break

- **Silent example loss** (the scariest failure: autoexamples off + a
  missing block = example quietly stops building). Countered three ways
  and all three verified live: `cargo check --examples --features debug`
  compiles the full catalog; `catalog_matches_disk` PASSES and pins
  disk == catalog == smoke lists (+ NOT_SMOKED); the earlier misplacement
  of `autoexamples = false` under `[lib]` (a silently ignored key) was
  caught and moved to `[package]` - and the drift test asserts the key's
  presence textually, so a future accidental removal fails a bare
  `cargo test`.
- **Vacuous test passes.** The first ui-smoke attempt WAS vacuous (ran in
  the main checkout, 0 tests matched, pipe ate the exit code) - caught by
  reading "0 passed; 1 filtered out" instead of trusting exit 0. The redo
  proves the real thing: `ui_reach_playing_without_panic ok`, 43s, in the
  worktree, 4 filtered out (correct 5-test binary). Category filters
  cannot cross-match (`sections`/`gameplay`/`ui`/`screenshots` are not
  substrings of each other's test names or of `catalog_matches_disk`).
- **Rename escapes.** Full-tree re-grep (not just the sweep list): every
  remaining numbered mention is deliberate history (task records, news,
  CHANGELOG released sections, the development.md lineage note, ci.yaml's
  pre-rework mention, hud_range's retired-example comment). Meaning-level
  re-read caught what grep could not: development.md's "four blocks" +
  "all eighteen"/HARNESSED_EXAMPLES paragraph, guide-add-section's
  numbered-slot instructions ("01-05 are taken, use the next free
  number"), and the CHANGELOG Unreleased bullets that will ship with
  v0.8.0 - all rewritten to the new scheme.
- **The probe surface.** `probe run scenario` end to end: verdict OK,
  measured 5/6, timeline + invariants armed and PASSING, manifest +
  report produced. nova_probe's own tests (renamed fixtures) pass.
- **Module/data path traps.** turret_section's `#[path]` attr updated
  with the dir; screenshot_reel's `include_str!("data/...")` is
  file-relative so co-moving data/ kept it compiling (proved by the
  check pass). No gitignore/Trunk.toml/build.rs patterns touch examples/.

## Findings

- R1.1 (NIT, recorded not fixed): old-name muscle memory gets cargo's
  "no example target named 10_playable" rather than a friendly pointer.
  A transitional old->new hint map in probe/cargo is not worth the
  surface; the CHANGELOG bullet is the migration note.
- R1.2 (NIT): the drift test parses Cargo.toml textually (trimmed
  `name = `/`path = ` lines). Nonstandard formatting would show up as a
  loud mismatch (fail-closed), so the simplicity is acceptable.
- R1.3 (accepted risk): sections/gameplay/screenshots smoke categories
  were not run locally (skip-local-tests rule; ui was the local sample).
  CI runs the full suite with the same runner and assertions - the risk
  is a category-specific runtime regression the reorg cannot plausibly
  introduce (names and paths are the only delta, and probe + ui prove
  both).

## Verdict

- VERDICT: APPROVE - land after user testing, per the flow.
