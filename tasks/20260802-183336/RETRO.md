# Retro: Scaffold the standalone nova_autopilot crate

- TASK: 20260802-183336
- BRANCH: feat/nova-autopilot-crate
- REVIEW ROUNDS: 1

## What went well

The plan carried a corrected DoD proof and the correction survived into the
work. The epic's dependency check (`! rg -n "nova_|..."`, unanchored, unguarded)
was unsound twice - it matched the crate's own `name = "nova_autopilot"` line,
and on a missing manifest `rg` exits 2 so the `!` reported success. The planned
form anchored the pattern and guarded with `test -f`. Base-red was confirmed
against that corrected form before any file was written, so the proof was known
to discriminate rather than assumed to.

Breadth: the diff is small and matches the plan exactly - four Steps, four
artifacts, no scope found late. Nothing was independently splittable and nothing
grew. This is what a scaffold task should look like.

Churn: one review round, one MINOR, no rework of the implementation. The MINOR
was a doc-surface miss, not a design miss, so neither the from-scratch challenge
nor the cold-reader rationale test would have caught it - it is a checklist gap,
addressed below.

Context: no pressure observed. No checkpoint, no compaction warning, no
delegation. Single focused pass.

## What went wrong

**Adding a workspace crate did not update the `AGENTS.md` code map.** Caught in
review as the round's only finding and folded in on the branch. The decision
that failed was implicit: treat "the Steps enumerate the artifacts" as "the
Steps enumerate the work". That seemed sound because the plan was unusually
concrete - it named files, line numbers and the exact manifest shape - so it
read as exhaustive. It was exhaustive about the crate and silent about the
repo-level surfaces a new crate touches.

**Round 1 was reviewed in-context.** The skill defaults to an outside reviewer
and the session directive forbids dispatching subagents unasked, so the default
could not be met. Mitigated by rerunning every proof and re-deriving the
load-bearing dependency claim through `cargo metadata` rather than re-reading
the manifest the grep already covered. Recorded as an exception in REVIEW.md
rather than papered over.

**`Cargo.lock` was nearly missed.** It was modified by the first `cargo check`
and left unstaged in the initial commit; caught on reading `git status` output
before moving on, and amended in. A `--locked` CI build would have failed on it.
The habit that saved it - read the status output rather than trusting the `git
add` list - is the one to keep.

## What to improve next time

Adding a workspace member is a repo-level change with a fixed surface, not just
a directory. That surface here is: root `Cargo.toml` members, `Cargo.lock`, and
the `AGENTS.md` code map. Two of the three were hit only because a proof or a
reviewer forced them.

## Action items

- Submitted to the central knowledge repository: the new-workspace-crate
  companion-surface checklist, and the anchored-and-guarded shape for negative
  `rg` proofs (an unguarded `! rg` over a possibly-missing file passes
  vacuously, because `rg` exits 2 on a missing path).
- Epic `20260802-120019` still carries the unsound unanchored dependency check
  in its own DoD; it should adopt the guarded, anchored form before that DoD is
  run. Already flagged in this task's Notes.
