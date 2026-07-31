# Notes: porting the NOVA OS phosphor theme to the web app

Design record for task 20260731-143918. The WHAT and the alternatives are in
DECISION.md; this is what the implementation actually did, and where it departed
from the plan's intended mapping.

## Shape of the change

`web/src/style.css` was rewritten rather than patched. The port touches nearly
every rule (the plan's own estimate: 1635 lines, 19 tokens, 238 `var()` reads),
and the whole point of the change is that surfaces stop being flat 1px-bordered
panels and become moulded material - so a token-value swap would have been the
retint the owner explicitly rejected.

The `:root` palette + vocabulary block is copied VERBATIM from the PoC. Not
"transcribed": `web/tests/theme.test.ts` parses `examples/ui/nova_ui_rework_poc.html`
at test time and compares values, so a hand-edit on either side fails the build.

## Deviations from the plan's intended mapping

The plan said to adjust the mapping against the rendered result and record the
deviations. Three.

### 1. Small labels use `--phosphor-dim`, not `--phosphor-muted`

The plan mapped "eyebrows / labels / tags / meta" onto `--phosphor-muted`. At
small text sizes that fails contrast:

| Foreground | On `--case-1` (#161b20) | On `--space` (#03060b) |
| --- | --- | --- |
| `--phosphor-muted` #0d6e35 | 2.73:1 | 3.03:1 |
| `--phosphor-dim` #19a64f | 5.47:1 | 6.07:1 |

WCAG AA for body-size text is 4.5:1, so `--phosphor-muted` is unusable for
anything a reader has to read. It works in the game because there it labels
large, glanceable readouts, not 0.74rem prose furniture.

So `--phosphor-muted` is kept for pure ORNAMENT - the `[ ]` brackets around a
section eyebrow, the `#` on a wiki tag, the rule after a card number, the dashed
edge of a placeholder recess - and every label that carries meaning
(`.section__eyebrow`, `.post-card__excerpt`, `.card__body`, nav links, meta
lines, captions) is `--phosphor-dim`.

### 2. `.callout--breaking` is `--red`, not `--amber`

The plan's mapping says `--red` and the previous CSS comment said amber. Went
with `--red` (the plan and the new palette's fault colour); the stale comment was
rewritten to match. Amber is now the "physical control" colour on this site
(keycaps, PromptFont glyphs, download keys, wiki band headings), so reusing it
for a breaking-change warning would have blurred the two meanings.

### 3. Two extra site-only tokens

`--ink` (#03060b, dark glyphs on a bright phosphor or amber fill) and `--screen`
(the `--screen-1 -> --screen-0` gradient, as a fill) have no PoC equivalent
because the PoC has no bright-filled buttons or code blocks. `--radius: 2px`
survives from the old sheet for small controls; only panels take the PoC's
`--panel-radius: 10px`. All three are declared under a "site-only additions"
comment so the parity test's exception surface stays visible.

## Mono legibility

This was the accepted risk in DECISION.md, and the mitigation is metrics only.

- Body `line-height` 1.6 -> 1.7; `.prose` 1.75.
- `.prose` measure 74ch -> 68ch, `.news__body` 72ch -> 70ch. Under a mono face
  `ch` is EXACTLY one character, so 74ch was literally a 74-character line - the
  top of the 60-75 readable band. Under the old proportional face the same 74ch
  measured narrower, which is why the number was fine before and is not now.
- `.wiki__body` had no measure at all (it fills its grid column). Left the block
  furniture full-width but capped running text (`> p`, `> ul`, `> ol`,
  `> blockquote`) at 74ch, so a wide desktop column no longer runs mono prose to
  the full 800px.
- Heading `line-height` 1.1 -> 1.25 and tracking +0.01em -> -0.01em: mono caps
  are wide and flat-sided, so they need more leading and no extra tracking.
  Heading sizes come down a step across the board for the same reason.
- Under 640px, `.prose` drops to 0.88rem with tighter gutters - mono at the old
  size on a 390px viewport gives ~34-character ribbons.

Confirmed on the captures, not by reasoning: `target/web-shots` at 1440 and 390.

## Verification

`scripts/shoot-web-pages.sh` is new because no web capture rig existed and this
is a readability change (lesson `render-output-eyeball`). It builds, serves
`web/dist` on a free ephemeral port, drives headless chromium over six page
kinds at two widths, and kills the server by recorded PID.

BEFORE and AFTER sets were captured on the same rig and compared at identical
crop and scale (lesson `compare-crops-at-one-zoom`). That comparison is what
established the two layout oddities below are PRE-EXISTING, not regressions:

- **Mobile hero:** the `SCROLL` affordance overlaps the LINUX download key at
  390px. Present in the BEFORE shot too; the port slightly improves it (the two
  CTAs now fit on one row instead of stacking).
- **Wiki index cards:** the whole card is an `<a>` inside `.prose`, so
  `.prose a { text-decoration: underline }` underlines the title AND the summary,
  and `.prose h3` (0,1,1) outranks `.wiki-index__cardtitle` (0,1,0) so the title
  renders amber rather than `--text`. Both behaviours are identical in the BEFORE
  shot. Left alone - fixing them is a layout change, not this task's port.

Also out of frame by design (DECISION.md): the hero banner and the tutorial /
news figures still show the retired navy game UI, because they are captured
IMAGES. Re-capturing them is backlog 20260724-082856.

## A defect in the task's own DoD

DoD 2's grep can never be clean as written: `\-\-panel\b` also matches the NEW
`--panel-radius` token (in ERE, `\b` sits between `l` and `-`). Corrected in
TASK.md to `\-\-panel([^-]|$)` with `--panel-2` listed separately. The parity
test's own retired-token check was already correct - it uses a JS
`(?![\w-])` lookahead.

## Doc surfaces

`npm run ci` gained a `npm test` step, so every doc that spelled out its
composition was stale: `README.md`, `web/src/wiki/dev/keeping-docs-in-sync.md`,
`web/src/wiki/dev/development.md` and `.claude/skills/release/SKILL.md`.
`development.md` also gains two subsections - how to eyeball the site with the
new rig, and the fact that the theme is now shared with the game via the PoC.
`crates/nova_ui/src/theme.rs`'s claim that "the web app keeps its own CSS" was
the sentence this task falsified; it now points at the shared source and the
parity test.
