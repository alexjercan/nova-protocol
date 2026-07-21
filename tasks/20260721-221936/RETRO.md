# Retro: F11 debug-toggle inversion (20260721-221936)

## What went well

- The owner's report was precise ("F11 shows the inspector + avian when I expect
  it to hide everything"), which mapped straight onto the exact defect I had
  flagged in the cursor task as an "acceptable trade" - it was not acceptable.
  The fix was already scoped before the sprout.
- Routing the three nova_debug `DebugEnabled` inserts through one const turned a
  "keep four literals in sync" hazard into "keep two in sync" (the const covers
  three; only the cross-crate ammo mirror stays a literal), and both remaining
  sides are pinned by a test.

## What went wrong

- This was self-inflicted: task 20260721-211500 flipped ONE of four sibling
  F11-toggled defaults and I reasoned the resulting inversion was a minor dev-UX
  trade. It was a regression a reviewer/owner would (and did) reject. The lesson
  the cursor review surfaced (predicate/state duplication across crates) had a
  bigger sibling I missed: FOUR independent `DebugEnabled` resources sharing one
  hotkey, kept coherent only by matching defaults.

## Lessons / what to do differently

- When N independent resources are kept in phase only by a shared default and a
  shared hotkey, changing ONE default is a breaking change to the whole set -
  treat "flip a default" as "flip all peers or unify them", not a local edit.
  The durable fix is a single source of truth (here, one const for the three
  same-crate states); a cross-crate peer that cannot import it needs a pinning
  test on each side, not just a comment.
- "Acceptable trade" flagged in a review is a smell: if it is worth a paragraph
  of justification, it is worth a follow-up task or a different design. Here the
  trade became a same-day regression.

## Follow-ups

- None. Manual owner check batched at Finish (F11 raises/lowers the whole layer;
  cursor re-locks in flight).
