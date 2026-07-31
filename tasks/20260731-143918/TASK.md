# Port the NOVA OS phosphor theme to the web app

- STATUS: CLOSED
- PRIORITY: 50
- TAGS: v0.9.0, ui, web
- KIND: TASK
- FLOW STEP: DONE
- PLAN STATUS: APPROVED

## Story

The game's UI rework (epic 20260728-175719, CLOSED) moved the ENTIRE in-game UI
onto the NOVA OS language - green phosphor on a near-black screen inside a dark
moulded casing, with light-3D physical controls. `crates/nova_ui/src/theme.rs`
now carries the PoC `:root` tokens verbatim and its doc comment records that the
flat navy/cyan palette "has been fully retired".

The web app never followed. `web/src/style.css` is still the navy/cyan
industrial-HUD theme the game USED to mirror (`--space-1 #0b0f1c`, `--cyan
#5cc8ff`, `--panel #141a2e`, Rajdhani/Inter type), so the site now advertises a
game that no longer looks like it. v0.9.0 cannot ship with the two out of sync.

Owner direction (2026-07-31, /flow), recorded in DECISION.md: **full material
port plus mono typography** - not a palette retint. The site adopts the same
light-3D vocabulary as the game (case-face gradients, rim/undercut/drop/well
bevels, CRT screen surfaces, 10 px panel radius) and goes terminal-first on
type. Owner accepted the denser long-prose reading that mono brings; the
mitigation is metrics (line-height, measure, size), never a fallback to a
proportional body face.

## Current state (understood 2026-07-31)

- Canonical token source: the `:root` block of
  `examples/ui/nova_ui_rework_poc.html` (lines 8-38). It defines both the
  palette (`--space`, `--case-0..3`, `--case-edge`, `--screen-0/1`,
  `--phosphor` / `-dim` / `-muted`, `--amber`, `--orange`, `--red`, `--blue`,
  `--text #b9ffc9`) and the light-3D vocabulary (`--face`, `--face-hot`,
  `--rim`, `--undercut`, `--drop`, `--well`, `--panel-radius: 10px`). This is
  the SAME block `crates/nova_ui/src/theme.rs` mirrors, so porting it to CSS
  makes site and game share one source.
- The web palette is fully centralized. `web/src/style.css` is 1635 lines with
  238 `var(--...)` reads over 19 tokens; the only colour literals outside it are
  five mermaid fallbacks in `web/src/wiki.ts:342-348`. No inline colour in the
  HTML (2 `style=` attributes total, neither a colour), no Tailwind colour
  utilities (`tailwind.config.js` extends nothing).
- Component families in `style.css`, in file order: header/nav, layout +
  section eyebrows, hero, buttons (`.btn--primary/--ghost/--download`),
  cards/grids, feature rows, prose (code, kbd, blockquote, tables, `hljs-*`,
  `pre.mermaid`), figures, post footer, PromptFont glyph chips (`.pf-*`),
  controls table, post cards, wiki (nav, index, children, tags, search),
  footer, callouts (`--breaking`), news TOC.
- Typography today: `@import` fetches Rajdhani + Inter + JetBrains Mono;
  `--font-display` Rajdhani, `--font-body` Inter, `--font-mono` JetBrains Mono.
  The PoC's face is JetBrains Mono (the game ships Iosevka Term; matching the
  exact face in-browser is backlog task 20260714-214329, not this task).
- Verification surface: `web/package.json` has `build` / `serve` / `test` /
  `ci` (`format:check && lint && build` - no test). `npm test` compiles
  `src/site.ts` + `tests/site.test.ts` with an explicit `tsc` file list, so a
  new test file must be added to that list. `chromium` is on PATH, so headless
  page screenshots are available; no web page-capture rig exists yet.

## Steps

- [x] Write the failing test FIRST: `web/tests/theme.test.ts`, stdlib-only
      (`node:fs`, `node:assert`), added to the `test` script's `tsc` file list
      in `web/package.json` and run after `site.test.js`. It parses the `:root`
      block of BOTH `web/src/style.css` and
      `examples/ui/nova_ui_rework_poc.html` and asserts:
      (a) every palette + vocabulary token the PoC defines is defined in
      `style.css` with an IDENTICAL value (independent source, not a constant
      copied into the test);
      (b) no legacy token name survives (`--cyan*`, `--space-0`, `--space-1`,
      `--panel`, `--panel-2`, `--border-bright`, `--amber-horizon`);
      (c) `--font-display`, `--font-body` and `--font-mono` all name
      `"JetBrains Mono"` first, and the Google Fonts `@import` requests neither
      Rajdhani nor Inter;
      (d) every `var(--x)` read anywhere in `style.css` resolves to a `:root`
      definition (catches a half-finished rename);
      (e) the vocabulary is actually CONSUMED, not just declared: `.btn`,
      `.card`, `.post-card`, `.wiki-index__card` and `.prose pre` each read at
      least one of `--face` / `--case-*` / `--screen-*` / `--rim` / `--drop` /
      `--well` / `--panel-radius`.
      Run it and record the failure lines in this task before editing CSS.
- [x] Build the eyeball rig: `scripts/shoot-web-pages.sh` - `npm run build` in
      `web/`, serve `web/dist` on a free port, drive headless `chromium
      --screenshot` over the six page kinds (landing `index.html`, `news.html`,
      one news post, `tutorial.html`, the wiki index, one wiki dev page with
      code + a table + mermaid) at desktop and mobile widths into a given output
      dir. Record the helper PID and kill by PID. Capture the BEFORE set on the
      base branch so the port has a comparison.
- [x] Port `:root` in `web/src/style.css`: replace the navy/cyan palette with
      the PoC tokens verbatim, add the light-3D vocabulary (`--face`,
      `--face-hot`, `--rim`, `--undercut`, `--drop`, `--well`,
      `--panel-radius`), keep `--radius: 2px` for small controls, point all
      three `--font-*` at the JetBrains Mono stack, and cut Rajdhani + Inter
      from the `@import`. Add the pointer back to
      `examples/ui/nova_ui_rework_poc.html` and `crates/nova_ui/src/theme.rs` in
      the header comment that currently explains the banner-derived palette.
- [x] Port the component families in file order, each one adopting the
      MATERIAL, not only the colour. Intended mapping (adjust against the
      rendered result, record deviations in NOTES.md):
      page field `--space`; header/footer/cards on `--face` + `--rim` +
      `--drop` at `--panel-radius`, hover to `--face-hot`; recessed surfaces
      (wiki search, figures, mermaid, placeholders) on `--well`; `.prose pre`
      and `.prose code` as CRT screens (`--screen-0` / `--screen-1`, phosphor
      text); `.btn--primary` the bright phosphor gradient `#7dffab -> #12b552`
      with dark `--ink` glyphs; `.btn--ghost` a case face with a phosphor
      border; `.btn--download` and `.pf` keycaps amber; body copy `--text`,
      links/live values `--phosphor`, secondary `--phosphor-dim`, eyebrows /
      labels / tags / meta `--phosphor-muted`; `.callout--breaking` `--red`;
      `.glow-cyan` / `.glow-amber` helpers renamed to the phosphor/amber pair
      with every call site updated.
- [x] Recolour the `hljs-*` block onto the phosphor/amber/orange/blue family so
      code reads as terminal output rather than a foreign editor theme.
- [x] Update the five mermaid fallback literals in `web/src/wiki.ts:342-348` to
      the new tokens and hexes.
- [x] Tune mono legibility, since this is the accepted risk: set body
      line-height, `max-width` measure and heading sizes for a mono face on the
      long surfaces (`.prose`, `.wiki__body`, `.news__body`) and confirm against
      the captures, not by reasoning.
- [x] Re-run the rig and EYEBALL every capture at both widths. Check contrast on
      muted-on-case text, that no element kept a navy hue, and that mobile
      layout still holds with wider mono glyphs.
- [x] Docs in the same task: `CHANGELOG.md` line under Unreleased (web section);
      refresh the `web/src/style.css` header comment; correct the stale sentence
      in `crates/nova_ui/src/theme.rs` that says the web app keeps its own
      separate CSS. Check `web/src/wiki/dev/keeping-docs-in-sync.md` for a web
      styling surface and update it if one is listed.
- [x] Add `npm test` to the `ci` script in `web/package.json` so the theme test
      is covered by the one command AGENTS.md names for website verification.

## Definition of Done

1. The theme parity test passes, and failed first with its failure lines
   recorded in this task (test: `cd web && npm test`).
2. No legacy navy/cyan token name or hex survives anywhere in the web sources
   (cmd: `grep -rn --include='*.css' --include='*.ts' --include='*.html' -E '\-\-(cyan|cyan-bright|cyan-deep|space-0|space-1|panel-2|border-bright|amber-horizon)\b|\-\-panel([^-]|$)|#(5cc8ff|8fe0ff|2a9fd6|141a2e|1a2138|233052|3a4d7a|e8eefc|8b95b0|0b0f1c|070a14|0f1424|ffb877|ff7a3c)' web/src`; expect exit 1).
   CORRECTED during work: the planned pattern used `\-\-panel\b`, which in ERE
   also matches the NEW `--panel-radius` token, so it could never be clean. See
   NOTES.md.
3. Website verification is green, now including the theme test (cmd: `cd web &&
   npm run ci`).
4. The capture rig produces a screenshot for all six page kinds at both widths
   (cmd: `scripts/shoot-web-pages.sh target/web-shots`).
5. Render eyeball: every capture reviewed - phosphor/case material on all six
   page kinds, no surviving navy element, mono prose legible at both widths.
6. manual: owner confirms the ported site against the game's look, and accepts
   the mono reading of a long wiki page.

## Notes

- Out of scope, recorded in DECISION.md: re-capturing the site's game
  screenshots (still the retired navy UI - backlog 20260724-082856); shipping
  the game's Iosevka face to the browser (backlog 20260714-214329);
  `crates/nova_ui` itself (already NOVA OS).
- CI does not run any web check (`.github/workflows/ci.yaml` has no node job;
  the pages workflow only runs `npm ci && npm run build`), so DoD 1-3 are local
  gates. Adding a web CI job is a separate decision, not this task.
- Lesson `render-output-eyeball` applies directly: this is a readability task,
  so it is unverified until the pages are SEEN, and the capture rig is step 2
  precisely because none exists.
- Fonts: the site keeps JetBrains Mono (already fetched, and the PoC's own
  face). Matching Iosevka Term exactly is the separate backlog task.

## Close-out (2026-07-31)

### What and why

The site now speaks NOVA OS. `:root` carries the PoC's palette AND its light-3D
vocabulary verbatim, every component family adopts the MATERIAL (case faces with
lit rims and drop shadows, CRT screens for code, recesses for wells), and all
three type slots resolve to JetBrains Mono. Site and game are no longer two
hand-synced lists: both mirror `examples/ui/nova_ui_rework_poc.html`, and
`web/tests/theme.test.ts` parses it as an independent source and fails on drift.

Design record, deviations from the planned mapping, and the mono-legibility
metrics: NOTES.md. WHAT and alternatives: DECISION.md.

### Test first - the recorded failure

`web/tests/theme.test.ts` was written and run BEFORE any CSS edit. It failed on
check (a), listing all 22 NOVA OS tokens absent from `style.css`:

```
AssertionError [ERR_ASSERTION]: style.css :root is missing NOVA OS tokens:
  actual: [
    '--space (PoC: #03060b)',
    '--case-0 (PoC: #0a0d10)',   '--case-1 (PoC: #161b20)',
    '--case-2 (PoC: #232a31)',   '--case-3 (PoC: #2f383f)',
    '--case-edge (PoC: #05070a)',
    '--screen-0 (PoC: #001304)', '--screen-1 (PoC: #002b0f)',
    '--phosphor (PoC: #36ff79)', '--phosphor-dim (PoC: #19a64f)',
    '--phosphor-muted (PoC: #0d6e35)',
    '--orange (PoC: #ff7b2d)',   '--red (PoC: #ff4e42)',
    '--blue (PoC: #36a3ff)',     '--shadow (PoC: rgba(0, 0, 0, 0.78))',
    '--face (PoC: linear-gradient(180deg, var(--case-3) 0%, ...))',
    '--face-hot (PoC: linear-gradient(180deg, #3a444c 0%, ...))',
    '--rim (PoC: inset 0 1px 0 rgba(255, 255, 255, 0.14))',
    '--undercut (PoC: inset 0 -2px 5px rgba(0, 0, 0, 0.7))',
    '--drop (PoC: 0 2px 4px rgba(0, 0, 0, 0.55), ...)',
    '--well (PoC: inset 0 2px 5px rgba(0, 0, 0, 0.85), ...)',
    '--panel-radius (PoC: 10px)'
  ]
  expected: []
```

The assertion order means (a) short-circuits the run, so (b)-(e) surfaced as the
port progressed rather than all at once - which is what caught the two things a
value swap would have missed: `--panel`/`--panel-2` reads left behind in the wiki
and callout rules (check b + d), and surfaces that had taken the new COLOURS but
kept flat backgrounds (check e).

### Alternatives considered during implementation

- **Patch `style.css` in place instead of rewriting it.** Rejected: the port
  changes what a surface IS (border+flat-fill -> face+rim+drop), so nearly every
  rule's body changes anyway and a patch series would have been harder to read
  than the result.
- **Alias the retired names (`--panel: var(--case-1)`) to shrink the diff.**
  Rejected: it would leave the navy vocabulary in the file as a permanent
  synonym layer, and DoD 2 exists precisely to prevent that.
- **`--font-mono: var(--mono)` so the parity test needs zero exceptions.**
  Rejected: the test's check (c) has to see a literal face name, and one
  documented exception is cheaper than a var-resolving parser.

### Difficulties and diagnosis

- **DoD 2's grep could never pass.** `\-\-panel\b` matches `--panel-radius`,
  because ERE puts a word boundary between `l` and `-`. Diagnosed by running the
  planned command on a fully-ported file and getting 15 hits, all of them the new
  token. Corrected in the DoD above.
- **`--phosphor-muted` fails contrast as a label colour.** The planned mapping
  put eyebrows, tags and meta on it. Computed the WCAG ratio before capturing:
  2.73:1 on `--case-1`, against the 4.5:1 floor. Moved every load-bearing label
  to `--phosphor-dim` (5.47:1) and kept `--phosphor-muted` for ornament only.
  Table in NOTES.md.
- **Two odd-looking things in the AFTER shots were not mine.** The mobile
  SCROLL/LINUX overlap and the underlined amber wiki-index cards both looked like
  port damage. Cropping BEFORE and AFTER at identical geometry showed both are
  pre-existing. Left alone rather than widening scope.
- **Lint on the new test.** A zero-width space inside a CSS-comment example in a
  doc comment tripped `no-irregular-whitespace`; two `as string` casts were
  redundant after the `assert.ok` narrowing.

### Evidence

| Proof | Result |
| --- | --- |
| `cd web && npm test` | GREEN (`site.test.ts` + `theme.test.ts`); failed first, above |
| DoD 2 grep (corrected) | exit 1, no hits |
| `cd web && npm run ci` | GREEN - format:check, lint, test, build |
| `scripts/shoot-web-pages.sh target/web-shots` | 12/12 captures + manifest.txt |
| `nix develop -c cargo check -p nova_ui` | GREEN (doc-comment edit) |

One unrelated fix rides along, because DoD 3 could not otherwise be green:
`web/src/index.html` was already unformatted at HEAD (a paragraph reflow prettier
wanted), so `npm run ci` was RED on master before this task. `npm run format`
fixed it; verified pre-existing by running `prettier --check` on `git show
HEAD:web/src/index.html`.

Also swept the rgba forms the DoD grep cannot see (`rgba(92,200,255,...)`,
`rgba(232,238,252,...)`, `rgba(255,184,119,...)`, `rgba(255,122,60,...)`) and the
retired `--border` / `--text-muted` / `--bevel` / `--shadow-hard` tokens: no hits.

Render eyeball (DoD 5): all 12 captures reviewed at both widths, plus two ad-hoc
crops for surfaces below the 2400px fold - the `.controls` keybind table with its
PromptFont glyphs, and the tutorial's in-prose CTA. Phosphor/case material reads
on all six page kinds; no styled element kept a navy hue; mono prose is legible
at 1440 and 390. The only navy left on the site is inside captured IMAGES of the
old game UI (hero banner, tutorial and news figures) - out of scope by
DECISION.md, backlog 20260724-082856.

DoD 6 (owner confirms the ported site against the game, and accepts the mono
reading of a long wiki page) is `manual:` and stays PENDING.

### Docs shipped in this task

`CHANGELOG.md` (new Web & Platform entry under Unreleased); `README.md`,
`web/src/wiki/dev/keeping-docs-in-sync.md`, `web/src/wiki/dev/development.md` and
`.claude/skills/release/SKILL.md` (all four spelled out `npm run ci` = format +
lint + build, now stale since it runs `npm test` too); `development.md` also
gains "Eyeballing the site" and "The theme is shared with the game";
`crates/nova_ui/src/theme.rs` header (its "the web app keeps its own CSS"
sentence is exactly what this task falsified).

### Reflection

The plan's step order was right and load-bearing. Writing the parity test first
turned a 1600-line restyle into a checklist that could not be half-finished:
check (d) caught leftover `var(--panel)` reads in rules I had not reached, and
check (e) caught surfaces that had taken the new colours while keeping a flat
background - which is precisely the "green but flat" outcome DECISION.md rejected
the retint for. A colour-only test would have passed on exactly the wrong result.

Building the capture rig before touching CSS paid twice: once for the eyeball
DoD, and once for the BEFORE set, without which I would have reported two
pre-existing layout quirks as regressions I had caused.

Next time: run the planned DoD commands against the current tree during PLANNING,
not at verification. The `--panel\b` defect was visible the moment the new token
name was chosen, and cost a round of confusion at the end of the work instead of
ten seconds at the start.
