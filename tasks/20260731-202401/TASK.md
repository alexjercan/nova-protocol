# Work skill: add a cargo doc baseline-diff verify step for module moves

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: backlog, chore, process
- KIND: TASK
- FLOW STEP: DROPPED
- PLAN STATUS: DRAFT

## Story

Promotion of ledger lesson `rustdoc-no-public-to-private-intra-doc-link` at
x3 (20260723-143530, 20260727-015156, 20260731-170322), approved by the owner
on 2026-07-31.

A `pub` item's rustdoc cannot `[intra-doc-link]` a private symbol, or a
cross-module item not in scope, without a `cargo doc` warning. Moving
documented code across a module boundary reliably breaks these. Some breaks
are silently WRONG rather than unresolved: a `super::`-relative doc path still
resolves after a move, just to a different module, so nothing warns loudly and
a reader is quietly misled.

Prose alone has not held. The x2 entry already said "run `cargo doc -p <crate>
--no-deps` as part of the move", and the x3 occurrence (a 14.5k-line module
split) still did not, shipping 30 new warnings and two wrong paths into review.
That is the argument for a standing verify step rather than another lesson.

## Steps

- [ ] Add to the `work` skill's verify guidance: when a change moves
      documented items across a module or crate boundary, run
      `cargo doc -p <crate> --no-deps --document-private-items` and diff the
      warning count against the BASE branch, not against zero.
- [ ] Say why the baseline matters: repositories carry pre-existing warnings,
      so an absolute count cannot distinguish inherited from introduced.
- [ ] Call out the silent case explicitly - a `super::`-relative path that
      still resolves after a move is the failure the warning count catches and
      a reader's eye does not.
- [ ] Mark the ledger entry ABSORBED once the step ships, naming this task.

## Definition of Done

1. cmd: `grep -n "cargo doc" ~/.claude/skills/work/verify.md` - the step is
   present and names the baseline diff.
2. manual: owner agrees the wording is short enough to survive in the skill.

## Notes

Skill prose is the right tier here, not tooling: the trigger ("did this change
move documented items across a boundary?") is a judgement about the diff, not
a checkable artifact state a hook could evaluate.


## Dropped

- REASON: meh
