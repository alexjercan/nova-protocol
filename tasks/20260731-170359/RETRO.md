# Retro: KISS: nova_menu - split the 7705-line lib.rs

- TASK: 20260731-170359
- BRANCH: refactor/nova-menu-split
- REVIEW ROUNDS: 2

## What went well

The structure axis. A brace-balance item parser that attaches each item's
leading doc/attribute lines, plus a name -> module assignment map, turned a
7705-line file into eleven by moving text byte-for-byte. Behavior preservation
became a property of the method rather than a claim about diligence.

Its verification matched it: a line-level multiset diff of non-comment source
against the base, which left 28 base-only lines, all of them import fragments
or rustfmt re-wraps. The out-of-context reviewer reproduced that number and the
`Plugin::build` byte-identity independently and raised no structural finding.

Cheap safety nets earned their keep. `cargo check`'s `never constructed`
warning caught `ScrollableList` emitted into two files from a four-line range
overlap - which is why the bar was zero warnings, not zero errors.

## What went wrong

The comment axis used a weaker method and a much weaker check. Regex-stripping
`task <id>` from prose damaged seven of 67 sites: glued words, an eaten `()`,
a flattened bullet list, a dangling "Since ... , so ...". None fails a compile.

The repair pass then declared six repaired and missed the seventh. The root
cause is the shape of the check, not the care taken: I hunted for damage with
patterns I had imagined - long words with tell-tale character classes, glued
preposition+article pairs - so `untilwires` was invisible, because "until" and
"wires" are both ordinary words. The decision seemed sound because the first
six hits came out of exactly that grep; finding damage felt like evidence the
detector worked, when it was evidence only that some damage happened to be of
the shape I had guessed.

Two smaller repeats of known lessons. The rewrap flattened `on_mod_toggle`'s
bullet list, the render-changing failure `doc-comment-rewrap-changes-the-render`
already describes. And the round-1 fixes shortened `mods.rs` from 875 to 873
lines, falsifying three NOTES.md rows until re-measured after the last edit.

## What to improve next time

For a mechanical edit to prose, write the conservation check first and derive
it from the edit's invariant, not from imagined failure modes. Here that is a
word-level multiset diff of every comment against the base: nothing vanishes
that was not deliberately deleted, nothing is invented. It is the same idea as
the line-level diff that made the structure axis trustworthy, one level down,
and it costs about ten lines of Python.

Symmetry is the tell. When one half of a task has a conservation check and the
other half has a pattern hunt, the pattern-hunt half is where the misses are.

## Action items

- Ledger: bump `doc-comment-rewrap-changes-the-render` to x3 (-> Pending
  promotions), `conserve-on-regroup` to x2 with the sentence widened from
  regrouping to any mechanical edit of prose, `re-measure-records-after-the-last-edit`
  to x2, and `generated-links-need-real-targets` to x4.
- 20260801-005057 opened: pre-existing `ambiguous import visibility` warnings
  in `nova_gameplay`, which rustc will make a hard error.
- DoD 6 stays open for the owner.

## Diagnose

- **Breadth.** Inherently large, and not splittable. The unit of work is one
  file: the structure and comment axes both had to touch every line of it, and
  a half-split `lib.rs` is not independently landable. 8047 insertions against
  7587 deletions is the move, not new code.
- **Churn.** Both review rounds were the comment axis; the structural half
  needed no rework. The plan-time question that would have prevented it is not
  in `plan` - the plan was right about WHAT to do. It is the work-time question
  of what proves a mechanical edit correct, which the structure axis answered
  well and the comment axis answered badly. Recorded as a lesson, not a plan
  defect.
- **Context.** No measured pressure: no compaction warning, no checkpoint, no
  handoff. One delegation, the round-1 out-of-context reviewer, which found the
  MAJOR the primary had missed - the case for the out-of-context default.
