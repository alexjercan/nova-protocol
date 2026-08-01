# Retro: KISS: nova_ui, nova_os, nova_editor, nova_debug

- TASK: 20260731-170437
- BRANCH: refactor/kiss-ui-os-editor-debug
- REVIEW ROUNDS: 1

## What went well

- The sibling tasks' two cheap proofs were run before opening the review, not
  after it asked: the `pub` item-set diff of pre-split file vs post-split
  folder (empty for both `widget` and `terminal`) and the comment-stripped
  sorted line multiset (residue is only imports, `pub(super)`, `mod`/`pub use`
  and rustfmt re-wraps). The review re-derived both independently and could
  APPROVE a ~4.2k-line move diff on evidence instead of hunk-by-hunk reading.
- The external-crate import probe that task 20260731-170340 had to add after
  its round 1 was written up front here: a throwaway test importing all 56 old
  public paths from `nova_gameplay`. The lesson transferred across tasks
  instead of being re-learned.
- Visibility was walked at split time rather than blanket-widened - only the
  systems `register()` names became `pub(super)` in `widget/`, and only
  `NovaOsTerminal`'s fields in `terminal/`. That was 20260731-170432's round-1
  finding, and it did not recur.
- Shared test fixtures were hoisted per split (`widget/fixtures.rs` for 4 test
  modules, `terminal/fixtures.rs` for 2), not copied per child.
- Split boundaries were cut by concern (widget family; terminal state/edit/view)
  rather than by size, so `register()`'s cross-family ordering edge stayed at
  one schedule site instead of becoming per-module `register` fns.

## What went wrong

- NOTES.md asserted "no test was renamed or weakened" while the branch also
  deleted the dead helper `fn only_button` and its keep-alive - the branch's
  sole non-comment deletion. The claim seemed sound because it was true of
  every test that still exists; the deletion was correct and in scope, but an
  unrecorded one is exactly what a reader reconciling the line multiset trips
  over. Review LOW; fixed here by naming it in NOTES.
- Two rustdoc paragraphs (`theme.rs:102`, `units.rs:4`) kept their old wrap
  points after a provenance clause was cut mid-paragraph, leaving a short line
  in the middle. `cargo fmt` does not re-wrap doc prose, so no check catches
  it. Review LOW; both re-wrapped here.
- REVIEW.md was written without the record schema `tatr` enforces (no
  `- TASK:`/`- BRANCH:`/`- VERDICT:`/`- REVIEWER:` lines), so the
  REVIEWING -> COMPOUNDING transition failed on six preconditions and the
  header had to be rewritten before the flow could move.

## What to improve next time

- When a comment pass cuts a clause out of the MIDDLE of a doc paragraph,
  re-wrap that paragraph in the same replacement - `fmt` will not, and no
  check will.
- Record every non-comment deletion in NOTES, however obviously right; a
  "moves and comments only" claim is only checkable if its exceptions are
  listed.
- Scaffold review records with `tatr scaffold <id> REVIEW` rather than writing
  the file freehand, so the schema `tatr flow` checks is present from the start.

## Action items

- None needing their own task. Both review findings were fixed on this branch;
  the ledger entries carry the general lessons.

## Diagnose

- Breadth: 33 files, ~4.2k lines each way, almost all moved. Inherently large -
  the Story is a four-crate pass, and the two file splits touch the same text
  the comment rubric does, so cutting them apart would have doubled the review.
  Not a missed split: `nova_editor` and `nova_debug` are comment-only and could
  not have stood alone.
- Churn: one round, no rework - both findings were record/cosmetic, neither
  blocking. No plan-time question would have prevented them; the plan's
  "keep public paths stable" already carried the load-bearing constraint, and
  the mechanical proofs it implied were run before review.
- Context: no threshold crossing, compaction warning, handoff or delegation.
  Four crates at ~9.1k lines fit one pass, with the review run as a separate
  cold session per skill default.
