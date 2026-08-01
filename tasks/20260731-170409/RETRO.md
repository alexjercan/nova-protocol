# Retro: KISS: nova_assets

- TASK: 20260731-170409
- BRANCH: refactor/kiss-nova-assets
- REVIEW ROUNDS: 1

## What went well

The split calls were made on concerns rather than line counts, in both
directions: `portal.rs` split four ways even though moving its tests alone
would have cleared the 1500-line DoD, and `shakedown.rs` kept its 1221-line
production script whole even though it is the largest file left. Both calls are
argued in TASK.md's Alternatives, so review did not have to guess.

The pre-existing test failure was classified against master BEFORE it was
written up, and filed as 20260801-122138 instead of fixed inside a refactor
task. That is `merge-red-check-preexisting` applied without being prompted.

Review settled the "no behavior change" claim mechanically rather than by
reading 8.3k lines of diff: reducing both trees to a comment-stripped line
multiset left a residue of only `mod`/`use`/`pub use`/visibility lines and
rustfmt re-wraps. Worth reusing on every remaining crate in this epic - it is
the only proof of that claim that scales with the diff.

## What went wrong

The comment pass was scripted, and needed three rounds because its skip guard
was coarser than its edit unit. Round 1 skipped any comment BLOCK containing a
blank line (which is most module docs); round 2 skipped any block containing a
list item (which swallowed docs that merely precede a list). Both times the
guard was written for the right hazard at the wrong granularity, and both times
it failed silently - the pass reported success with a third of the work
undone. Moving the guard to the PARAGRAPH cleared it.

The same scripted pass then mangled ten sentences where the deleted clause was
grammatically load-bearing ("real art is task X" -> "real art is."). Six were
deferred-work comments and became the `TODO(...)` markers the rubric wanted
anyway; four needed hand repair. This is the sixth pass in this epic to take
comment-rewrap damage from a scripted substitution, and the previous one
already concluded the fix: write the pass as asserted replacements instead.

Review found three MINORs, all residue of the split rather than of the code:

- A dead `#[allow(missing_docs)]` on `PortalChannel` (portal/mod.rs:107),
  added defensively when the struct went `pub(super)`. Verified unnecessary -
  removing it produces no warning. The compiler will never ask for it back, so
  nothing would have surfaced it.
- `fn entry(...)` duplicated verbatim into `portal/catalog.rs` and
  `portal/install.rs` when the test module split. The same task solved the same
  problem correctly for shakedown, by hoisting the shared helpers into
  `shakedown/tests/mod.rs`.
- NOTES.md and the close-out both say the DoD grep returns 6 hits; it returns
  7. The 7th (`collections.rs:236`) is in NOTES.md's own marker table, so the
  count was transcribed from the sentence being written rather than produced
  from the finished tree.

## What to improve next time

Two of the three findings are the same omission: after a split compiles, walk
the new boundary once and ask what was ADDED to make it compile that is not
needed - suppressions, widened visibility, duplicated helpers. Nothing in the
toolchain flags any of them, and all three are cheap to check while the split
is still in mind.

For the counts: produce them, do not transcribe them. Every number in NOTES.md
came from a command; running it once more after the last edit costs nothing.

## Diagnose

**Breadth.** The diff is large (8.3k lines) but not from missed splitting - it
is one crate, three file splits, and a whole-crate comment pass, all of which
the task scoped explicitly. The comment pass is what makes the line count
misleading: rewrapping a paragraph re-writes lines that did not semantically
change. Scoping the rewrap to paragraphs that actually changed was the right
call and kept it reviewable.

**Churn.** One review round, no rework - the findings are MINOR and the verdict
is APPROVE, so there is no plan-time question to answer here. The plan encoded
the right design: split on concerns, name the rubric in the parent epic rather
than restating it, forbid new abstractions.

**Context.** One compaction occurred during the work phase (observed, not
measured). It cost nothing recoverable - the branch was committed before the
break and the records carried the state. The comment pass is the part worth
delegating or deferring next time: it is mechanical, it is most of the diff,
and it is independent of the structural split.

## Action items

- Fold the two code MINORs into the epic's next crate pass, or fix them
  directly on this branch if the owner prefers (both are one-line deletions
  plus a hoist).
- No new task filed; the review findings are recorded in REVIEW.md and the
  defect found during the pass is already 20260801-122138.
