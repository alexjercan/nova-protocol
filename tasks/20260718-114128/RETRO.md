# Retro - Fix horizontal scroll overflow on wiki guide-make-a-mod page

- Task: 20260718-114128
- Verdict: APPROVE (round 1)
- Landed: cb0c6b58 on master

## What went well

- Root-caused before touching CSS. The tempting "fix" is to add `word-break`
  to the code block, but that only papers over long lines and changes how code
  reads. The real cause was the grid item's default `min-width: auto` letting a
  wide `pre` grow the `1fr` column past the viewport - so `overflow-x: auto` on
  `pre` never got a chance to scroll locally. One line (`min-width: 0`) at the
  right element fixed it.
- Reused an in-file precedent. `.wiki-child__body { min-width: 0 }` already
  existed ~150 lines below; the fix matches an established local convention
  instead of inventing one, which also raised confidence it was correct.
- Checked the whole page for other overflow sources (tables, long URLs, long
  inline code) rather than assuming the one reported symptom was the only one.

## What went wrong / friction

- No headless browser in the build environment, so the fix could not be
  confirmed by measuring `scrollWidth` vs `clientWidth` on the real page. Had
  to rely on mechanism + build + precedent. Recorded honestly as a `[~]` step
  and a review caveat; a human eyeball on localhost is the final check.
- Task bookkeeping churn: `tatr new` wrote the stub into the shared checkout
  (bg-isolation guard blocks edits there), so the stub had to be removed from
  the main checkout and the real TASK.md recreated inside the sprout worktree.

## What to do differently next time

- For web/layout bugs, decide up front whether the environment can run a
  headless browser; if not, say so early and lean on mechanism + precedent
  rather than pretending a visual check happened.
- When `tatr new` runs before sprouting, expect the stub to land in the shared
  checkout and plan to move/recreate it in the worktree (or run `tatr new`
  after sprouting).
