# Retro: Salvage pickup sound + WorldSfx deletion

- TASK: 20260717-101659
- BRANCH: task-20260717-101659-salvage-pickup-sound (squash-landed 71de2e1e)
- REVIEW ROUNDS: 1 (APPROVE)

The family's closing cycle; short retro. See TASK.md/REVIEW.md and the spike's
fix record (tasks/20260717-101524/SPIKE.md) for the family's full arc.

## What went well

- The 101641 lesson (sweep-content-repo-wide, webmods included) paid on the
  FIRST swing this time: the-ledger ch1's 4 salvage crates were found and
  wired by my own pre-review sweep - the exact gap class the reviewer caught
  last cycle reached zero rounds this cycle. That is the compounding working
  as intended.
- Deleting the WorldSfx machinery end to end in one cycle was safe because
  every prior cycle had already deleted its own keys - the "shrink to
  deletion" migration design meant the final deletion was 4 files, not a
  repo-wide untangling.
- The dedup-books-every-pickup subtlety (unauthored crates still enter
  DingedCrates) was caught at design time by asking what silence should NOT
  change.

## What went wrong

- Nothing worth a lesson; the two scripted-edit assert-aborts during the
  README/spike updates were the asserts doing their job against drifted text.

## What to improve next time

- Nothing new; the family's lessons are recorded on the earlier cycles.

## Action items

- None. The spike's Next steps are all CLOSED; no follow-ups seeded.
