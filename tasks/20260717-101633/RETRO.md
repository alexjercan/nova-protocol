# Retro: Controller section sounds

- TASK: 20260717-101633
- BRANCH: task-20260717-101633-controller-sounds (squash-landed edc9b14e)
- REVIEW ROUNDS: 2 (APPROVE at round 2)

Short retro; the family pattern is settling. See TASK.md/REVIEW.md.

## What went well

- The riskiest claim (player-controller lookup vs real ship hierarchy) was
  verified from PRODUCTION code before review - the input-routing systems
  already filter sections by direct ChildOf(ship), so the lookup copied a
  shipped topology instead of assuming one. Reviewer independently confirmed
  via the assembly code + a full shipped-ships prototype sweep.
- Draining message readers BEFORE the controller check (and pinning
  no-stale-replay in a test) preserved a subtle behavior the old no-bank guard
  carried; naming that behavior in the plan kept it from being lost in the
  rewrite.
- One component for five same-consumer sounds (vs five components) kept the
  cue systems' queries flat; nobody pushed back.

## What went wrong

- Same class as last cycle, next surface over: rewrote the cue model but left
  the README's "Required files" tables presenting 8 migrated sounds as
  bank-loaded (R1.1). The 101624 retro's lesson ("prose-grep the old model's
  words") was applied to RUSTDOC but not to the asset-dir README - the sweep
  stopped one surface short. Root cause: the prose grep ran over crates/, not
  assets/**/README.md.

## What to improve next time

- The old-model prose grep must cover EVERY doc surface the feature story
  touches (crates rustdoc + assets READMEs + wiki + design docs), not just the
  code tree. For the remaining family tasks: grep "bank" across assets/ and
  web/ after each flip.

## Action items

- None new; this sharpens the 101624 retro note - carried forward in the next
  cycles' plans rather than a new ledger slug.
