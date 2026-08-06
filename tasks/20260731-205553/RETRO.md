# Retro: Clear compiler and rustdoc warnings for v0.10.0

- TASK: 20260731-205553
- BRANCH: master (landed in place, no feature branch - owner directive)
- REVIEW ROUNDS: 2

## What went well

Inventory before fixing. The task was filed against three known warnings; the
real surface was 133. Running all three sweeps first and writing the per-lint
table into NOTES.md turned an open-ended cleanup into a countable work list, and
made the two "no longer reproduces" Story items visible before anyone hunted for
them.

Reading the lint, not the count. 71 of the 105 clippy hits were one lint,
`doc_lazy_continuation`, and the naive fix (indent the continuation) would have
been wrong for most of them: the real cause was a `-` or `+` landing at the
start of a wrapped line, where markdown reads it as a list marker. Moving the
punctuation up a line fixes the cause. One site (`content.rs`) had a single
stray `+` producing 17 of the warnings on its own.

The review earned its keep on a task that looked purely mechanical. Both DoD
sweeps were already green when round 1 ran, so a "the proofs pass, ship it" pass
would have found nothing - and the five real findings were all invisible to
`-Dwarnings` by construction.

## What went wrong

Five links were de-linked that should have been relinked. The promoted
`rustdoc-no-public-to-private-intra-doc-link` lesson says to drop the brackets
rather than widen the item, and NOTES.md restated that rule at plan time. The
decision that seemed sound then: treat "rustdoc cannot resolve this" as one
category with one fix. It is two. A link can fail because the target is private
(drop the brackets) or because the target is public but not reachable from the
doc's own scope (give it an explicit path). Applying the private-item fix to the
second category deletes a working link and the warning goes away either way, so
the DoD could not catch it.

The fix commit staged a directory rather than the edited files, sweeping in a
`tatr scaffold` REVIEW.md stub whose placeholder content contradicted the round
it claimed to record. That is exactly the failure mode the
`no-worktree-stage-explicit-paths` habit exists to prevent, and this task ran in
the main checkout where it applies most.

## What to improve next time

Split the "unresolved intra-doc link" fix by why it failed, not by the warning
text. Before dropping brackets, check the target's actual visibility: `pub` and
re-exported means relink with an explicit path; private, `pub(crate)`,
`pub(super)` or not-a-dependency means drop them.

A whole-project warning sweep should name its three commands up front - rustc,
rustdoc AND clippy. This task's original DoD listed only the first two, so
clippy's 105 warnings were outside the stated bar until the owner asked for the
full project.

When a verification is invisible to the proof commands (a link that resolves
vs. a link that was deleted), say so in the review handoff. Round 1 was asked to
check exactly that, and it did - including reading the generated HTML in round 2
rather than trusting the absence of a warning.

## Diagnose

Breadth: 60 files, but not a missed split. The unit of work is "one lint across
the workspace"; splitting per crate would have produced seven tasks that each
have to re-run the same three whole-workspace sweeps to prove themselves. The
plan predicted this ("the unbounded part is the `-Dwarnings` inventory") and
correctly scheduled the task last in the sprint.

Churn: the cold-reader rationale test would have caught it. NOTES.md recorded
the RULE (drop the brackets) without its CONDITION (because the target is
private). A cold reader given only that line applies it to every unresolved
link - which is precisely what happened. Rules copied into a plan need the
predicate that gates them, not just the action.

Context: no compaction, no threshold crossing, no handoff. One delegation, the
round-1 reviewer, resumed once for fix confirmation.

## Action items

- Knowledge submitted: `changes/silencing-a-diagnostic-is-not-fixing-its-cause`
  (a repair and a deletion are indistinguishable to the checker, so choose by
  cause and review the artifact) and
  `docs/prose-punctuation-at-a-line-start-becomes-markup` (a rewrap can turn
  prose punctuation into a list marker; fix the character above, not the
  indentation below).
- No follow-up tasks. The `proc-macro-error2` future-incompat note is
  third-party and has no first-party fix.

## Landing message

```
fix(20260731-205553): clear every compiler, rustdoc and clippy warning

Inventoried the whole workspace before fixing: rustc was already clean, rustdoc
had 28 first-party warnings and clippy 105 under the workspace lint config.

71 of the clippy hits were doc_lazy_continuation, caused by a hyphen or `+`
landing at the start of a wrapped doc line where markdown reads it as a list
marker; fixed by moving the punctuation up a line rather than by indenting. The
rest were fixed at the site: writeln!, contains, as_chunks, derived Default,
struct-literal init, &Path over &PathBuf.

Rustdoc links were split by why they failed - public-but-unreachable targets got
an explicit path, private ones lost their brackets. Two stale
#[expect(type_complexity)] and one duplicated #[allow(too_many_arguments)] were
removed; both lints are allow workspace-wide and could never fire.

No broad allow added. The only suppressions are two targeted
#[expect(inconsistent_digit_grouping)] on the shakedown belt seeds, whose
<date>_<index> grouping is meaning rather than magnitude.

All three sweeps now exit 0 under -Dwarnings.
```
