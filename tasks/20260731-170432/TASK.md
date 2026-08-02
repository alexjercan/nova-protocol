# KISS: nova_probe run harness

- PRIORITY: 35
- TAGS: v0.9.0, refactor, chore, probe
- KIND: STORY
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE
- PARENT: 20260731-170222

## Story

As a maintainer I want the probe run harness to fit an agent context and carry only comments
that earn their place, so future work in this area starts from a smaller,
quieter surface.

Rubrics (comment + structure) live in the parent epic. Read the epic index
before starting; do not restate the rubric here.

Scope: crates/nova_probe/
Current size: ~8.3k lines across 13 files. Largest file: bin/probe.rs at 2460 lines.

## Steps

- [x] Read the parent epic's comment and structure rubrics.
- [x] Inventory: per-file line counts and the concerns each file holds.
- [x] Split files that hold more than one concern; keep public paths and
      prelude exports stable.
- [x] Apply the comment rubric file by file: delete narration and provenance
      clauses, promote surviving constraints to NOTE/TODO/FIXME/BUG, keep
      rustdoc.
- [x] Open backlog tasks for any defect the pass uncovers; do not fix here.
      (None found.)
- [x] Verify: check, fmt, and the existing tests for this area.

## Definition of Done

1. cmd: `nix develop --command cargo check --workspace --all-targets` - green.
2. cmd: `nix develop --command cargo fmt --check` - clean.
3. cmd: `grep -rnE '//.*[0-9]{8}-[0-9]{6}' crates/nova_probe/` - every hit is a
   deliberate NOTE/TODO/FIXME/BUG reference, listed in NOTES.md.
4. cmd: `wc -l crates/nova_probe/` - no file over 1500 lines, or NOTES.md justifies the
   exception as one cohesive concern.
5. test: existing tests covering this area still pass.
6. manual: owner skims the diff and agrees no behavior changed.

## Notes

Moves, renames, deletions only. No new abstractions, no behavior change.

## Close-out

**What and why.** Split the two files over 1500 lines into folder modules by
concern and stripped the comment fluff crate-wide. `src/bin/probe.rs` (2460)
became `src/bin/probe/main.rs` + `native.rs` + nine `native/*.rs` concern
modules; `src/run_report.rs` (1590) became `run_report/{mod,manifest,
artifacts,checks,html,fixtures}.rs`. Largest file in the crate is now
`run_report/checks.rs` at 913 lines, which is one concern. Full module map in
NOTES.md.

**Alternatives.** Keeping the bin at `src/bin/probe.rs` and putting its
modules beside it in `src/bin/` was rejected: a bin file resolves submodules
against its own directory, so `probe`'s private modules would have become
sibling bin candidates in the shared `src/bin/` namespace. Moving the target
to `src/bin/probe/main.rs` (one Cargo.toml path edit) keeps them private to
the bin.

**Difficulties.** Two mechanical traps. Slicing by line range left a dangling
doc comment + `#[derive]` at each file boundary (the header of the item that
starts the next file), which rustc reports as "expected item after
attributes"; the fix was to move each dangling header into the file that owns
its item. And the extracted `mod tests` bodies came out one indent level deep
because they had lived inside `mod native`.

**Evidence.** DoD 1-2: workspace check + `cargo fmt --check` green. DoD 3:
`grep -rnE '//.*[0-9]{8}-[0-9]{6}' crates/nova_probe/` returns zero hits, so
no exception list is needed. DoD 4: `wc -l` maximum is 913. DoD 5:
`cargo test -p nova_probe --lib --bins` runs 97 tests, all green, and the
sorted set of `#[test]` names is byte-identical to the pre-split tree. DoD 6
stays pending for the owner.

**Reflection.** The line-range extraction is fast but its boundaries are
between an item's doc comment and the item; cutting at "the blank line before
the next doc comment" instead of "the line before the next item" would have
avoided both dangling-header errors. Comparing the test-name multiset before
and after is a cheap, strong no-behavior-change proof for a pure move pass and
is worth doing first, not last.
