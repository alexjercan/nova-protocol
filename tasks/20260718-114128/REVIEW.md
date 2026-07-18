# Review - Fix horizontal scroll overflow on wiki guide-make-a-mod page

- VERDICT: APPROVE
- Round: 1
- Branch: fix/wiki-guide-scroll-x
- Commit: b095c2eb

## What the change does

Adds `min-width: 0;` to `.wiki__body.prose` in `web/src/style.css`. The wiki
article column is the `1fr` track of the `.wiki` CSS grid; as a grid item it
defaults to `min-width: auto`, which refuses to shrink below the intrinsic
width of its widest child. The guide page has a 164-char unbreakable line in a
RON `pre`, so the column grew past the viewport and forced page scroll-X even
though `.prose pre` already has `overflow-x: auto`. Constraining the item with
`min-width: 0` lets the track stay at its grid size, so the `pre` now scrolls
locally instead.

## Assessment

- **Correctness (fix level):** Correct. `.wiki__body` is the direct grid child
  of `.wiki` (verified in `web/src/wiki.html`), so `min-width: 0` is applied at
  exactly the right element. No intermediate wrapper needs it.
- **Idiom / consistency:** Matches the existing overflow-guard already used in
  this stylesheet at `.wiki-child__body { ... min-width: 0 }` (~line 1285), so
  the fix follows an established local convention rather than inventing one.
- **Scope of the fix vs the whole page:** Checked the page for other overflow
  sources. No tables, no long bare URLs. `pre` blocks are covered by their own
  `overflow-x: auto` and now scroll locally. So the reported desktop scroll-X
  (the "a lot" overflow from the 164-char line) is fully resolved.
- **Build:** `npm ci && npm run build` compiles successfully; no CSS/build
  regression.
- **Blast radius:** `min-width: 0` on this item only changes shrink behavior of
  the article column; it cannot widen anything or affect other pages' layout
  beyond letting over-wide children scroll/clip within the column. Low risk.

## Findings

### [low] Long inline code tokens still lack word-break (pre-existing, not this bug)

`.prose code` has no `word-break`/`overflow-wrap`. A ~46-char inline code file
path exists on the page (`crates/nova_assets/tests/webmods_validation.rs`,
line 309). At desktop widths this fits the column comfortably, but at very
narrow (mobile) widths a single long inline-code token could still poke past
the column. This is a pre-existing, separate concern from the reported bug (the
`pre` block overflow), and the remedy (breaking code tokens mid-string) has its
own readability tradeoff. Out of scope for this task; noted for a possible
future mobile-polish task rather than blocking here.

## Verdict rationale

The change cleanly and minimally fixes the reported bug at the correct level,
follows an in-file convention, builds green, and has a small blast radius. The
one finding is low severity, pre-existing, and out of scope. APPROVE.

Caveat carried from the work phase: no headless browser was available in the
build environment, so pixel-level browser confirmation was not run. The
mechanism, build, and precedent give high confidence; a human eyeball on
localhost after rebuild is the final confirmation.
