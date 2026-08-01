# Retro: KISS: nova_probe run harness

- TASK: 20260731-170432
- BRANCH: refactor/kiss-nova-probe
- REVIEW ROUNDS: 2

## What went well

- The comment pass ran as ~35 asserted replacements (each aborting unless its
  anchor matched exactly once) instead of a regex sweep. No mangled sentence,
  no collapsed list, no silent under-edit; `cargo doc -p nova_probe` came back
  with zero nova_probe warnings on the first run.
- Behavior equivalence was proved mechanically rather than argued: a
  comment-stripped, sorted line multiset over the crate leaves only
  `mod`/`use`/`pub use` lines, visibility keywords and rustfmt re-wrapping as
  residue, and the sorted `#[test]` name lists on master and the branch are
  identical. Both checks were cheap and settled a question the review would
  otherwise have had to take on trust.
- Shared test fixtures were hoisted, not copied: `native/fixtures.rs` (argv
  and catalog builders for three test modules) and `run_report/fixtures.rs`
  (fixture dir, scratch copy, healthy manifest).

## What went wrong

- The split blanket-promoted every top-level bin item to `pub(crate)` with one
  sed, and 20 of them never cross a module boundary. It seemed sound because
  the compiler cannot object to over-wide visibility, so the widening looked
  free; it is not - it erases the module interfaces the split exists to draw.
  Review R1.1 caught it, and the fix (drop the prefix, keep it only where a
  `pub(crate)` signature names the type) was mechanical.
- The close-out first recorded "100 tests" - a count taken from
  `grep -cE '^test '`, which also matches the three `test result:` lines. The
  producing command was not the DoD command. Corrected to 97 by re-running
  `cargo test` and reading its own totals.
- Slicing files by `sed` line range cut between an item's doc comment and the
  item, so two files ended with a dangling `///` + `#[derive]` header
  belonging to the next file's first item ("expected item after attributes").
  A boundary at the blank line before a doc block, rather than at the line
  before an item, has no such failure mode.

## What to improve next time

- After a split compiles, walk the visibility boundary in both directions
  before opening the review, not after.
- Take every recorded number from the command the DoD names, run against the
  finished tree.
- Cut split boundaries at blank lines between item blocks, never at the item
  line itself.

## Action items

- None needing their own task. R1.1 was fixed on this branch; the ledger
  entries below carry the general lessons.

## Diagnose

- Breadth: the diff is large (34 files, ~3.4k lines each way) because the
  Story is a whole-crate pass, and almost all of it is moved lines. It is
  inherently large, not a missed split: the two file splits and the comment
  rubric touch the same files, so cutting them apart would have doubled the
  review of the same text.
- Churn: one MINOR round. The plan-time question that would have prevented it
  is the from-scratch challenge applied to the mechanics, not the design -
  "if I wrote these modules from scratch, would `parse_run` be `pub(crate)`?".
  The plan named "keep public paths stable" but said nothing about the private
  side of the boundary, and the sed took the permissive default.
- Context: no threshold crossing, compaction warning, handoff or delegation
  occurred; the crate fit one pass.
