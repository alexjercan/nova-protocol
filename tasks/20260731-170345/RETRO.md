# Retro: KISS: nova_gameplay flight, camera, audio, juice

- TASK: 20260731-170345
- BRANCH: refactor/kiss-gameplay-flight-camera-audio-juice
- REVIEW ROUNDS: 1

## What went well

- The line-range concatenation method carried over from the sibling task
  20260731-170340 without adaptation. Map exact ranges of the original, emit
  each new file by concatenating them under a hand-written header, delete the
  original, then let `check --all-targets` find every boundary crossing.
  Because no line is retyped, "did behavior change" reduces to reading the
  visibility and import diff - and the reviewer could confirm it independently
  with a line-multiset comparison.
- Conservation was checked at two independent levels: `#[test]` count (119
  before, 119 after) and the executable-line multiset. Both were cheap and
  both were exactly what the reviewer re-derived.
- The re-export discipline the sibling task learned the hard way
  (`split-must-re-export-not-repoint`) was applied from the start: every
  `mod.rs` re-exports the full parent-path surface with explicit name lists,
  and no call site outside the three modules needed repointing.
- Round 1 found only MINOR/NIT, all in the records and in over-wide
  visibility - nothing about the code the task actually moved.

## What went wrong

- The visibility sweep was a column-0 regex (`s/^(struct|enum|fn|const) /pub(super) \1 /`).
  It did its job for the boundary crossings but also widened eight `#[test]`
  fns and seven items that only their own file references. The decision seemed
  sound because the compiler proves the LOWER bound (too narrow fails to
  build) and nothing proves the upper bound - so the sweep's over-reach is
  invisible to every check. It cost a review round to catch.
- The comment pass left records that were wrong by the time they were
  written: the separator count said four when thirteen were deleted, and a
  `NOTE:` site's line number went stale when a later re-wrap shifted it. Both
  numbers were correct when first measured and neither was re-measured after
  the last edit.
- Deleting a mid-paragraph clause leaves ragged lines that rustfmt will not
  fix. The re-wrap script handled the prose paragraphs but skipped markdown
  bullets by design, and one hand-wrapped bullet was left at 124 columns.

## What to improve next time

- After a scripted visibility sweep, grep each widened item for a reference
  outside its defining file and narrow the ones with none. It is one loop and
  it closes the only failure mode the compiler cannot see.
- Re-measure every recorded number after the LAST edit of the round, not after
  the edit that motivated it. Better: keep the producing command next to the
  number in the record.

## Action items

- Ledger: new `visibility-sweep-narrows-back` (x1).
- Ledger: bumped `doc-comment-rewrap-changes-the-render` to x5 and
  `re-measure-records-after-the-last-edit` to x4 (re-crosses the DEFER's
  stated revisit point).
- No follow-up task: no defect was uncovered, and both bumped lessons already
  have a disposition owner in the ledger.

## Diagnosis

- **Breadth.** The diff is large (9828 lines of source relocated) because the
  Story is a whole-area KISS pass over five files, three of them oversized. It
  is inherently large, not a missed split: the three splits share one method
  and one conservation argument, and separating them would have triplicated
  the epic's rubric read and the doc-surface sweep for no reviewer benefit.
- **Churn.** One round, four MINOR/NIT findings, all mechanical. The
  plan-time question that would have prevented the biggest one is not in
  `plan` at all - it is a work-phase check: a scripted edit needs a pass that
  audits what it changed BEYOND its intent. That is now `visibility-sweep-narrows-back`.
- **Context.** One compaction occurred mid-task, between the `audio.rs` split
  and its first clean compile. It cost nothing structural: the scratchpad
  split scripts and the `.orig` copies were on disk, so the pending compile
  errors were re-derived from a single `cargo check` rather than from memory.
  Next time, the same shape - keep the mechanical inputs on disk, not in
  context.
