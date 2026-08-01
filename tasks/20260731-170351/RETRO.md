# Retro: KISS: nova_gameplay sections and integrity

- TASK: 20260731-170351
- BRANCH: refactor/kiss-gameplay-sections-integrity
- REVIEW ROUNDS: 1

## What went well

- Three of the ledger's lessons were applied BEFORE they could fire, and each
  one paid:
  - `visibility-sweep-narrows-back` - no regex sweep at all this time. The
    split was compiled first and the `pub(super)` set was driven off the
    compiler's own E0603/E0425 list, so the widened set was exactly 17 items
    and every one has an out-of-file reference. The lesson's "grep each
    widened item afterward" loop became unnecessary rather than skipped.
  - `split-must-re-export-not-repoint` - every new `mod.rs` re-exports its
    parent-path surface, and `cargo doc -p nova_gameplay --no-deps` (which the
    lesson names as the only standing check that notices) came back with zero
    warnings under the touched scope.
  - `doc-comment-rewrap-changes-the-render` - the comment pass was written as
    ~60 explicit replacements, each asserting its anchor occurs exactly once,
    instead of a scripted substitution. The rewrap-damage scan was clean on
    its first run, which is the first time in six passes that has happened.
- Conservation was proved at a level the reviewer could re-derive without any
  of the implementer's bookkeeping: `#[test]` over the whole scope, 126 before
  and 126 after, from one command over each side.
- The comment audit was a multiset diff of comment lines, base vs new. That
  turned "did I delete a guard?" from a judgement call into a 201-line list to
  read, and every guard on it was found again as a `NOTE:`.

## What went wrong

- Three counts in the close-out were wrong when review read them: `NOTE:`
  promotions (7 recorded, 10 actual), items widened (15 recorded, 17 actual),
  ID-only separators deleted (4 recorded, 5 actual). Every one was correct at
  the moment it was written and none survived the edits that followed. The
  numbers were transcribed rather than produced, so nothing tied them to the
  tree - the same failure the ledger has now seen five times.
- Two of the three came from counting the plan instead of the result: the
  `NOTE:` figure counted the promotions decided during the small-file sweep
  and never re-added the ones the turret and torpedo passes created.

## What to improve next time

- Write the producing command INTO the record next to the number as the number
  is written, then run the file's commands once at the end. The three wrong
  numbers here each had a one-line command that would have produced them; none
  was recorded, so re-measurement depended on remembering to.
- Never count a category incrementally across a multi-file pass. Count it once,
  from the tree, after the last file.

## Diagnose

- Breadth: inherently large, and correctly so. The diff is 12 files because the
  comment rubric applies to the whole scope; the two structural splits are the
  only files that grew new siblings. No independently landable split was
  missed - a "split turret" task and a "strip comments" task would have touched
  the same files twice.
- Churn: none the plan could have prevented. The single finding is a record
  accuracy defect introduced during implementation, not a design question the
  from-scratch challenge or the cold-reader rationale test would have surfaced.
- Context: no threshold crossing, no compaction warning, no handoff, and no
  delegation (the session forbids subagents, which also forced the recorded
  in-session review exception). The 3668-line file was never read whole; it was
  mapped by item inventory and sliced by line range, which is what kept the
  working set small.

## Action items

- Ledger: bump `re-measure-records-after-the-last-edit` to x5 with this task's
  three-count evidence (it already carries a PROMOTE disposition and
  20260801-112556 as its owner, so no new disposition is proposed).
- Ledger: new `comment-pass-as-asserted-replacements` (x1) - the countermeasure
  that kept the rewrap scan clean.
- No follow-up task: no defect was uncovered in the code, and both lessons
  touched already have an owner in the ledger.
