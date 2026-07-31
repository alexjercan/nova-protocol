# Web: rework the site onto the PHOSPHOR skin only (drop the hardware material)

- STATUS: CLOSED
- PRIORITY: 52
- TAGS: v0.9.0, ui, web, feedback
- KIND: TASK
- FLOW STEP: DONE
- PLAN STATUS: APPROVED

## Story

Owner feedback on the landed port (20260731-143918): the site adopted the
light-3D HARDWARE skin (moulded `--face` buttons, `--rim`/`--drop`/`--undercut`
bevels). The wanted look is the PHOSPHOR skin only - the widget-zoo terminal
vocabulary in `examples/ui/nova_ui_rework_poc.html`
(`body[data-skin="phosphor"] ...`): flat CRT surfaces, 2px radius, green
hairline borders, glow instead of bevel, inverted solid-phosphor fill for the
selected/primary state.

DECISION.md of 20260731-143918 chose "full material port"; this task supersedes
the SKIN half of that choice. Palette, typography and layout stay as landed.

## Steps

- [x] Extract the phosphor skin vocabulary from the PoC zoo block into `:root`
      site tokens (hairline edges, fills, panel face/shadow, recess, glows).
- [x] Port buttons: `.btn`, `--primary`, `--ghost`, `--download`, nav CTA.
- [x] Port panel surfaces (header, cards, wiki nav/index/child, TOC, callout,
      footer, hero art) off `--face`/`--rim`/`--drop` onto the phosphor panel.
- [x] Port recesses and screens off `--well` onto the phosphor trough/CRT.
- [x] Rework `web/tests/theme.test.ts` check (e): assert the PHOSPHOR
      vocabulary is consumed AND the hardware tokens are consumed nowhere.
- [x] Re-capture the site and eyeball all six page kinds at both widths.

## Definition of Done

1. The reworked theme parity test passes, and its new hardware-consumption
   assertion failed first against the landed sheet, with the failure lines
   recorded in this task (test: `cd web && npm test`).
2. No hardware material token is CONSUMED anywhere in the stylesheet
   (cmd: `grep -nE 'var\(\s*\-\-(face|rim|undercut|well|key-face)' web/src/style.css`;
   expect exit 1). This greps READS only - the `:root` definitions still mirror
   the PoC verbatim for parity check (a).
3. Website verification is green (cmd: `cd web && npm run ci`).
4. The capture rig produces a screenshot for all six page kinds at both widths
   (cmd: `scripts/shoot-web-pages.sh target/web-shots`).
5. Render eyeball: every capture reviewed - flat CRT surfaces, hairline green
   borders, no bevel or moulded key anywhere, prose still legible at both widths.
6. manual: owner confirms the site now reads as the phosphor terminal skin.

## Close-out

WHAT. `web/src/style.css` moved off the PoC's light-3D HARDWARE vocabulary and
onto its PHOSPHOR skin, end to end: buttons, panels (header, cards, post cards,
wiki nav/index/child, TOC, callout, footer, hero art), recesses, tables,
keycaps and the mermaid tint. The hardware tokens stay declared in `:root` as
part of the verbatim PoC mirror, and a new parity check keeps them dead.

WHY. Owner feedback on the landed port: "this uses more of the hardware theme
for buttons, I want to use ONLY phosphor theme from the zoo widgets example".
The site had taken the skin the game offers as the ALTERNATE look; the default
- and the one the game ships with - is phosphor.

ALTERNATIVES. (a) Retint the hardware material green: rejected, it is the same
bevelled construction and the feedback is about construction, not hue.
(b) Redefine `--face`/`--rim`/`--well` to phosphor values so no rule changes:
rejected, it breaks parity check (a) (the `:root` mirror compares by VALUE) and
leaves the sheet claiming a material it no longer draws.

DIFFICULTIES. Two things did not work the first time and were caught by looking
at renders rather than at exit codes:
- The mermaid diagram stayed grey through two edits. `primaryColor` does not
  drive flowchart node fill under the `dark` base theme (`mainBkg`/`nodeBorder`
  do), and rgba() input breaks mermaid's derived-shade maths. Fixed with opaque
  pre-composited tints; see NOTES.md deviation 4.
- Panels double-drew their hairline when `--panel-shadow` carried the PoC's
  `inset 0 0 0 1px` on top of the border every site panel already had.

EVIDENCE.
- Fail-first (check (e), after review round 1 sharpened it): run against
  `master:web/src/style.css` it reports `these surfaces read no phosphor-skin
  token: buttons (.btn), cards (.card), post cards (.post-card), wiki index
  cards (.wiki-index__card), code blocks (.prose pre)`.
- Fail-first (the alias guard): with `--btn-face: var(--face)` planted in
  `:root` and read by `.btn:hover`, it reports `:root aliases hardware material
  into a consumed token: --btn-face: var(--face)`.
- Fail-first: the new check (f) run against `git show HEAD:web/src/style.css`
  reported all ten hardware tokens consumed -
  `actual: ['--face','--face-hot','--rim','--undercut','--well','--case-0',
  '--case-1','--case-2','--case-3','--case-edge'], expected: []`.
- `cd web && npm test` -> site.test.ts + theme.test.ts all assertions passed.
- `cd web && npm run ci` -> format:check, lint, test, webpack build all green.
- DoD 2 grep -> exit 1, no hits.
- `scripts/shoot-web-pages.sh target/web-shots` -> 12 captures, exit 0.
- Eyeball: all 12 reviewed (6 individually, the last 4 mobiles on one appended
  sheet), plus a crop of the architecture mermaid diagram.

NOT RUN. `cargo check` - the only Rust change is a doc comment in
`crates/nova_ui/src/theme.rs`. CI owns the workspace build.

REFLECTION. Review round 1 caught that the FIRST version of check (e) was
vacuous - its token list included `--phosphor` and `--screen`, which the sheet
it was written to reject already read, so it passed on the old styling
unchanged. A consumption check is only worth its line count if it is run against
the thing it is supposed to reject; "it passes on my branch" proves nothing. The
parity test earned its keep twice. Check (d) would have caught a
half-finished rename, and the new check (f) is what makes "phosphor only" a
property of the repository rather than of this diff - the next person to reach
for `var(--face)` on the site gets a failing test, not a review comment. The
grep in DoD 2 is deliberately shaped to match READS (`var(--x`) and not the
`:root` definitions, which is the same trap that bit DoD 2 of 20260731-143918.
