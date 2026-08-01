# Retro: KISS: small crates and root binary

- TASK: 20260731-170448
- BRANCH: refactor/kiss-small-crates-root
- REVIEW ROUNDS: 1

## What went well

The comment-stripped residue check was run BEFORE review and came back total:
all eight touched `.rs` files are identical to base once comment-only lines,
trailing `//` tails and blanks are dropped. That turned "no behavior changed"
from a claim the reviewer had to read 250 diff lines to believe into a
one-command check the reviewer re-derived independently in seconds. It is the
whole reason a comment pass over five crates closed in one round.

Refusing the structure axis was also right. The inventory came first, showed
every file single-concern with a 446-line maximum, and the pass reported that
instead of manufacturing a split to match the shape of the sibling tasks. A
split here would have been line-count-driven, which the epic rubric forbids,
and would have buried an otherwise provably inert change under a move diff.

## What went wrong

Breadth: not applicable - the diff is 9 files, comments only, and every file
was named in the task scope. Nothing grew.

Churn: one MINOR in one round. `nova_core/src/lib.rs`'s retained NOTE points at
`nova_gameplay's hud/nova_os.rs`, a file an EARLIER child of this same epic had
already split into `hud/nova_os/`. The pass rewrote that exact comment and
carried the dead path through the rewrite. The failed decision looked sound at
the time: the comment-rubric loop asks of each comment "does this constraint
still bind?", and this one does - so it was rewritten and kept. The loop never
asks whether the PATHS inside a kept comment still resolve, which is the one
part of a comment pass that is mechanically checkable rather than a judgment
call. No plan-time question would have caught it; a check would have.

Two independent sprouts implemented this one task in parallel,
`refactor/kiss-small-crates` (16:07) and this branch (16:24), cut separately
from `master` with no shared history. Both ran to a complete close-out. Roughly
half the work here was thrown away, and the review opened by having to pick a
branch rather than by reading a diff. The stale flow state is the likely
mechanism: the first sprout committed `--to REVIEWING` on its own branch, so
`master`'s copy of TASK.md and the second sprout's copy both still read
PLANNED, which reads as unclaimed work.

The task header's "largest file: lib.rs at 622 lines" was wrong by ~40% (real
maximum 446, and that file is `nova_modding`'s, not `nova_core`'s). Had it been
trusted, this task would have been worked as a split.

Context: no pressure observed. The scope fits one context and, unlike the
earlier children of this epic, fits the box's RAM - every test in scope ran to
completion rather than being filtered.

## What to improve next time

Resolve paths inside every comment the pass KEEPS, not just the ones it writes.
A rewritten comment inherits its predecessor's rot, and a sibling task in the
same epic is the most likely thing to have moved the file it names.

Check the plan's measured figures against the tree before choosing an approach.
One `wc -l` would have replaced the header's 622 with 446, and the approach
question (split or comment-only) turns entirely on that number.

One task, one sprout. Before cutting a worktree, check whether a branch for the
id already exists.

## Action items

- Bumped `generated-links-need-real-targets` to x6 in the ledger with the
  kept-comment case; it is already in Pending promotions with a tool proposed.
- Bumped `conserve-on-regroup` to x6 with the total-inertness result.
- New `one-task-one-sprout` (x1) and `re-measure-plan-figures-before-choosing-an-approach`
  (x1) in the ledger.
- Delete the abandoned `refactor/kiss-small-crates` branch and its worktree
  when this lands.
