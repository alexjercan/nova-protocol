# Spike: sharpen the web site visual design (less floaty, more industrial)

- DATE: 20260713-221822
- STATUS: RECOMMENDED
- TAGS: spike, web, design

## Question

The site (`web/`) reads as "floaty" and a bit generic-AI-landing-page. How do we
make it feel **sharper** and more industrial/structured - in the spirit of
factorio.com but explicitly NOT a copy - while keeping the existing Nova
Protocol palette (deep navy, neon cyan, amber)? A good answer is a concrete,
named design direction plus the specific CSS/markup levers to pull, decided well
enough that `/plan` can expand it into steps without re-litigating the look.

## Context

All styling lives in one file, `web/src/style.css` (~617 lines), consumed by the
page templates (`index.html`, `_header.html`, `_footer.html`,
`tutorial.html`, `wiki.html`, `blog.html`, `posts/*.html`). The palette and
fonts are already defined as `:root` tokens (navy `--space-*`, `--panel*`,
`--cyan*`, `--amber*`; fonts Rajdhani display / Inter body / JetBrains Mono).
The palette is good and stays.

What actually makes it "floaty" (the levers), from reading `style.css`:

- **Big rounded corners everywhere**: cards 14px, hero art 16px, buttons 10px,
  figures/video 12px, nav 8px, blockquote 8px, code/kbd 5-6px.
- **Diffuse colored glow**: `--shadow-glow-cyan: 0 0 24px`, `--shadow-panel:
  0 8px 32px` (very blurred), plus extra `0 0 40-60px` cyan halos on the hero
  art, figures and video, and `text-shadow` glows on brand/headings.
- **Hover float**: `transform: translateY(-2px/-3px)` on `.btn` and `.card`.
- **Glassy header**: `backdrop-filter: blur(10px)` on a translucent sticky bar.
- **Gradient buttons**: `linear-gradient(135deg, ...)` fills.
- **Amber radial wash** behind the hero (`.hero::after`).
- **Emoji card icons** (hammer/test-tube/globe/dart/... in `index.html`) - the
  single biggest "generated landing page" tell.

Factorio's site logic worth borrowing (not its beige skin): crisp rectangular
panels, hard 1px borders as the primary structural device, a machined
bevel/inset instead of haze, flat fills, deliberate (non-floaty) states, and
heavy use of small technical labels and framing.

## Options considered

- **A. Tune-down in place.** Just dial back the floaty knobs: shrink radii,
  delete the glow shadows, remove hover-float, flatten buttons. Cheapest, lowest
  risk, fully reversible. But it only subtracts - it leaves the centered-soft
  hero and the emoji cards, so it fixes "floaty" without fixing "looks
  generated". Necessary but not sufficient.

- **B. Industrial HUD-panel language (recommended).** Adopt a small, deliberate
  design system and apply it everywhere. Borrows Factorio's *logic*, keeps our
  colors. Concretely:
  - _Corners_: add a `--radius: 2px` token; panels/buttons 0-2px, pills gone.
  - _Borders as structure_: 1px solid `--border`, brightening to cyan on
    interaction; a 2-tone machined bevel via `inset 0 1px 0
    rgba(255,255,255,.05)` (top highlight) over a hard bottom/right shade -
    definition from edges, not haze.
  - _Shadows not glow_: replace blurred drop shadows with a tight hard shadow
    (e.g. `0 2px 0 rgba(0,0,0,.5)`) or none; delete every `0 0 Npx` color halo;
    keep at most a restrained brand text-glow.
  - _Buttons_: flat solid cyan primary / transparent amber ghost, 1px border,
    a hard "keycap" bottom border that compresses on `:active` (press, not
    hover-lift). No gradient.
  - _Hover_: no lift. Border brightens / background tints / a left or top accent
    bar slides in. Sharp and intentional.
  - _Header_: drop the blur; solid opaque `--space-0` bar, strong 1px bottom
    border + a thin cyan underline accent; nav active state is a bracket or
    underline, not a rounded pill.
  - _Hand-crafted framing_ (kills the "generated" feel): replace emoji card
    icons with **mono index numbers** (`01 / 02 / ...`) or short kicker labels;
    consistent `//`- or `[ ]`-bracketed mono eyebrows (the motif already exists
    as `// Systems online`); hairline section dividers with a labeled tab; a few
    restrained corner-tick / clipped-corner accents on the hero and section
    heads (not on every card - overuse turns it into a gimmick).
  - _Type_: lean into Rajdhani + JetBrains Mono, tighter heading letter-spacing,
    uppercase eyebrows/kickers for a technical hierarchy.
  This is mostly a `style.css` pass plus small markup edits (card kickers,
  header accent, section dividers, dropping the emoji spans). It layers A's
  tune-downs in as its baseline and then adds an identity on top.

- **C. Full Factorio clone.** Beige/orange chrome, sprite bevels, tiled metal.
  Rejected outright: the user said not a copy and to keep our palette; off-brand
  and heavy.

- **D. Brutalist terminal.** Monospace everything, ASCII box-drawing frames, no
  imagery. Genuinely un-generated, but too austere for a cinematic space-shooter
  landing page and it fights the banner art. Not whole-hog - but its mono-label
  and box-framing instincts are worth folding into B.

- **Do nothing.** The site works; this is polish. Cost of deferring is low, but
  the user asked directly and the change is contained to one CSS file plus light
  markup, so the payoff is favorable now.

## Recommendation

Go with **Option B (industrial HUD-panel language)**, folding in A's tune-downs
as its baseline and D's mono-label/framing motifs, explicitly not C.

It beats A because A only removes the floatiness and leaves the generic feel;
B replaces it with a deliberate identity (crisp bordered panels, mono technical
framing, hard states) that reads hand-built. It beats C/D because those break
the palette/brand constraint or the cinematic tone. The work is well-contained:
a focused `style.css` rewrite around a few new tokens (`--radius`, a bevel
shadow, a hard shadow), converting every rounded/glowy surface to a crisp
bordered panel, swapping hover-float for border/accent feedback, de-glassing the
header, flattening the buttons, and small markup edits to drop the emoji cards
for mono index numbers and add section framing. Palette and banner art stay.

## Open questions

- **Card icons**: mono index numbers (zero asset cost) vs a small custom SVG
  icon set (sharper but needs drawing). Recommend numbers first; SVGs are a
  later, optional upgrade. Possible user call.
- **Corner-tick accents**: how much is tasteful? Constrain to hero + section
  heads to avoid gimmickry; settle exact treatment during work.
- **Fonts**: confirm Rajdhani / JetBrains Mono are actually loaded (webfont
  `@font-face`/`<link>`), not silently falling back to system fonts - the
  sharper type hierarchy depends on it. Verify in `/work`.
- **Accessibility**: keep focus-visible states crisp and contrast within AA as
  borders replace glows.

## Next steps

Direction-level task this spike seeded, for `/plan` to break into steps:

- tatr 20260713-222025: sharpen the web visual design to an industrial
  HUD-panel style (implements Option B)

## Fix record

(Single seeded task; entries appended as work lands.)
