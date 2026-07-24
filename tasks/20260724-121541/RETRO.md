# Retro: Tab drawer z-order fix

- TASK: 20260724-121541
- BRANCH: fix/drawer-zorder
- REVIEW ROUNDS: 1 (in-session, trivial-diff carve-out; APPROVE)

(What/why in TASK.md close-out; this is process only.)

## What went well

- A clean, small cycle: the fix was two `GlobalZIndex` components and the
  approach was already pinned in the task body from the playtest triage, so there
  was nothing to re-decide. Grounding it took one grep - nova_menu's modal
  overlays already use `GlobalZIndex` (pause 10/11) AND already have a test that
  asserts an overlay root carries one, so both the fix tier and the test shape
  were copy-and-adapt (`reuse-known-good-stack`, third consecutive cycle).
- Recognising the diff as genuinely trivial and taking the in-session review
  round (per the review skill's carve-out) instead of spinning up a full
  out-of-context agent kept the cycle proportionate.

## What went wrong

- Nothing notable. The only real limit is inherent: a headless test can pin the
  z-index CONTRACT but not the actual render stacking, so the true proof is the
  owner's manual re-playtest - correctly left as the `manual:` DoD rather than
  faked with a green presence-test.

## What to improve next time

- Keep doing the "grep for an existing tested pattern first" move - it turned a
  UI-stacking fix that could have been guesswork into a mechanical copy of a
  proven, tested idiom.

## Action items

- [x] Lessons ledger: bump `reuse-known-good-stack` (now applied 3 cycles
  running - strengthens the pending promotion).
