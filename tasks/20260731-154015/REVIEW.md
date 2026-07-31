# Review: Web: rework the site onto the PHOSPHOR skin only (drop the hardware material)

- TASK: 20260731-154015
- BRANCH: fix/web-phosphor-only

## Round 1

- REVIEWER: out-of-context general-purpose subagent (prompt carried only the
  task ID, branch/worktree, dimensions and record format)
- VERDICT: REQUEST_CHANGES

The reviewer independently re-ran `npm test`, `npm run ci`, the DoD 2 grep and
the capture rig, and checked every Close-out and NOTES claim. All of them held.
The primary re-derived R1.1 and R1.3 before accepting them.

- [x] R1.1 (MAJOR) `web/tests/theme.test.ts:197` - reworked check (e) was
  VACUOUS. Its `PHOSPHOR` list included `--phosphor`, `--screen` and
  `--screen-0`, which the retired hardware sheet already read on all five
  surfaces, so the check passed unchanged against `master:web/src/style.css` -
  it could not tell the port from the thing it replaced.
  RESPONSE: trimmed the list to CONSTRUCTION tokens only (`--edge*`, `--fill*`,
  `--panel-face*`, `--panel-shadow`, `--recess`, `--glow-*`) and recorded the
  fail-first. Re-derived by the primary: `master`'s `.btn--ghost` reads
  `var(--phosphor)` at style.css:482, which is what was satisfying the check.
  The trimmed check now fails on the old sheet for all five surfaces:
  `these surfaces read no phosphor-skin token: buttons (.btn), cards (.card),
  post cards (.post-card), wiki index cards (.wiki-index__card), code blocks
  (.prose pre)`.

- [x] R1.2 (MINOR) `web/tests/theme.test.ts:262` - check (f) skips `:root`
  entirely, and check (d) forces every read to resolve there, so `:root` was
  the one place an alias (`--btn-face: var(--face)`) could smuggle the material
  back in past both checks.
  RESPONSE: added an alias assertion - a site token that reads a hardware token
  AND is itself read after `:root` fails. Verified it BITES by planting exactly
  the reviewer's alias: `:root aliases hardware material into a consumed token:
  --btn-face: var(--face)`.

- [x] R1.3 (MAJOR) `web/src/style.css` (`.controls`, `.wiki-search`,
  `.prose pre`, `.prose table`, `.prose pre.mermaid`) - each carried BOTH a 1px
  `--edge-faint` border and an `inset 0 0 0 1px var(--edge-faint)`, drawing the
  hairline twice - the exact defect NOTES deviation 1 claims was avoided for
  panels.
  RESPONSE: dropped the inset from all five. Also dropped it from
  `.figure__placeholder` and `.wiki-child__icon`, which the reviewer listed as
  border-less but in fact carry a dashed `--phosphor-muted` border. The inset
  survives only on `.prose blockquote` and `.post-card__ph`, which have no full
  border. Re-derived by the primary before fixing.

- [x] R1.4 (MINOR) `web/src/style.css` - `.prose thead th` read
  `var(--panel-shadow)` on a table cell: its 20px inset vignette covered the
  whole ~34px cell and its outer shadows were clipped by the table's
  `overflow: hidden`.
  RESPONSE: `background: var(--fill-hot)`, no shadow. Confirmed on a re-captured
  crop of the crate-map table.

- [x] R1.5 (MINOR) `web/src/style.css` - `.wiki-nav__link:hover` darkened into
  `--recess`; that is the hardware "hover cuts a well" idiom with the token
  swapped. The PoC's `body[data-skin="phosphor"] .row:hover` LIGHTENS, as do the
  sibling ports here.
  RESPONSE: now `--fill-hot` + `--phosphor-hot`.

- [x] R1.6 (MINOR) `web/src/style.css` - the `.prose kbd` comment claimed a
  match with the PoC's `.btn .key`, but used `--fill-amber` where the PoC uses a
  dark trough (`rgba(0,0,0,0.3)`).
  RESPONSE: background is `--recess`, so the claim is now true.

- [x] R1.7 (MINOR) `web/src/style.css` - `--text-hot` was documented as a glyph
  colour but also consumed as the FILL of a hovered inverted control.
  RESPONSE: renamed `--phosphor-hot` and documented for both roles.

Pending user checks (do not block APPROVE):

- DoD 6 `manual:` - owner confirms the site now reads as the phosphor terminal
  skin.

## Round 2

- REVIEWER: primary (verification-only round: every R1 finding was accepted and
  none disputed, so this round re-runs the checks and confirms the fixes rather
  than re-judging the design)
- VERDICT: APPROVE

All seven responses verified as recorded above - two of them by forcing the new
assertions to FAIL (R1.1 against the old sheet, R1.2 against a planted alias)
rather than by observing green. No fix regressed another surface: `npm run ci`
green, 12/12 captures, and the changed surfaces (table head, keycaps)
re-eyeballed on fresh crops.
