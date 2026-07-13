# Marketing landing site: play gate, blog, tutorial, wiki

- STATUS: CLOSED
- PRIORITY: 30
- TAGS: v0.5.0, web, landing, site, docs

## Goal

Stop shipping the raw WASM game as the whole site. Instead publish a themed
static marketing/content site that fronts the game: a hero landing page with a
"Play" / "Try" call-to-action that opens the game, plus content pages (blog,
tutorial, wiki). The game stays exactly as it is; it just moves to a sub-path
and gets a proper front door.

The site should feel like the `assets/banner.png` key art: deep space-navy
background, neon-cyan "NOVA" glow, warm amber "PROTOCOL" / horizon glow.

## Reference stack

Mirror `~/personal/football-guessr` (the user's preferred web stack) as closely
as makes sense here. That project is: **TypeScript + Webpack 5** (multi-page via
`html-webpack-plugin`, shared `_header.html` / `_footer.html` partials via a
small `webpack-partials.js`), **Tailwind CSS v4** through PostCSS
(`@tailwindcss/postcss` + autoprefixer), **Prettier + ESLint (typescript-eslint)**,
optional **Nix flake** dev shell, `npm run serve` / `npm run build` / `npm run ci`.
Read its `webpack.config.js`, `package.json`, `src/_header.html`, `src/index.html`
and `README.md` before scaffolding, and copy the shape.

## Architecture / decisions

- **Location:** the site is a self-contained npm project under a new `web/`
  directory at the repo root (the repo root itself is a Rust/Cargo workspace, so
  the JS toolchain must be nested, not at root). `web/` gets its own
  `package.json`, `webpack.config.js`, `tsconfig.json`, `tailwind`/`postcss`
  config, `eslint`/`prettier` config, `.gitignore` (node_modules, dist), and
  `src/`.
- **Pages (multi-page webpack build), each an entry + `HtmlWebpackPlugin`:**
  - `/` - hero landing: banner art, tagline, "Play" + "Try" CTAs, feature
    highlights, screenshots/gifs, links to blog/tutorial/wiki, GitHub link.
  - `/blog/` - blog index (list of posts) + at least one seed post
    (e.g. a devlog / "What is Nova Protocol"). Posts can be individual HTML
    pages or markdown rendered at build time; pick the lower-friction option
    and keep it consistent with football-guessr's `asset/resource` md handling
    if markdown is used.
  - `/tutorial/` - how to play: controls, the editor -> scenario loop,
    autopilot verbs (GOTO / ORBIT), HUD tiers. Pull accurate content from the
    game (AGENTS.md, CHANGELOG.md, docs/) - do not invent controls.
  - `/wiki/` - reference pages: ship sections (hull, controller, thruster,
    turret, torpedo bay), scenarios, gravity wells, factions. Source from
    `docs/sections.md`, `docs/scenario-system.md`, `docs/architecture.md`.
    A single wiki index with anchored sections is fine for v1; multiple sub-pages
    are a stretch.
- **Game integration / "Play":** the "Play" button links to the existing Bevy
  WASM game served at `/play/`. The Trunk build (`index.html` + `Trunk.toml`)
  stays the game's page; it just publishes under `/play/` instead of root.
  - "Try" vs "Play": treat both as the same entry into the game for v1. The
    "limited capabilities" demo idea is an OPEN QUESTION below - do not block on
    it. If pursued, it is a query param (e.g. `/play/?mode=demo`) the game reads
    to gate features, which is game-side (Rust) work and should be its own task.
- **Theme (from `assets/banner.png`):** define these as Tailwind theme tokens /
  CSS custom properties and use them consistently:
  - space background: near-black navy `#070a14` -> `#0b0f1c`
  - surfaces/panels: `#141a2e`, borders `#233052`
  - nova-cyan (primary/glow): `#5cc8ff`, bright `#8fe0ff`, deep `#2a9fd6`
  - protocol-amber (secondary): `#ffb877`, horizon `#ff7a3c`
  - text: `#e8eefc` primary, `#8b95b0` muted
  - signature touches: cyan neon `text-shadow` on headings, a warm amber radial
    horizon glow at the bottom of the hero, subtle starfield, glassy panels.
    Ship `assets/banner.png` as the hero image (copy it into the web build).

## Steps

### 1. Scaffold `web/`
- [x] Create `web/` with the football-guessr toolchain: `package.json` (scripts:
      `build`, `serve`, `format`, `format:check`, `lint`, `lint:fix`, `ci`),
      `webpack.config.js` (multi-page, `PUBLIC_PATH` env for project-pages
      subpath, partials plugin, dev server with `historyApiFallback` rewrites),
      `tsconfig.json`, `tailwind.config.js` (v4), `postcss.config.js`,
      `eslint.config.mjs`, `prettier.config.js`, `.gitignore`.
- [x] `src/style.css` = Tailwind entry + the theme tokens above.
- [x] `src/_header.html` / `src/_footer.html` partials: nav (Home, Play, Blog,
      Tutorial, Wiki, GitHub) and footer.
- [x] Verify `npm install && npm run build` produces a working `web/dist/`, and
      `npm run serve` renders locally.

### 2. Landing page (`/`)
- [x] Hero with `banner.png`, tagline ("A 3D space shooter: build modular ships,
      fly them through scenarios"), prominent Play / Try CTAs -> `/play/`.
- [x] Feature highlights (modular ship editor, autopilot verbs, scenarios,
      gravity wells, combat juice) - accurate to the game.
- [x] Section links into Blog / Tutorial / Wiki. Responsive + mobile-friendly.

### 3. Content pages
- [x] Blog index + one seed post.
- [x] Tutorial page (controls + core loop, accurate).
- [x] Wiki page(s) (sections / scenarios / mechanics, sourced from docs/).

### 4. Game -> `/play/` and deploy integration
- [x] Move the game under `/play/`: set Trunk `public_url` and build the game
      into a `play/` subdir (e.g. `trunk build --release --public-url <base>/play/
      --dist dist/play`). Keep the game's `index.html` as the game page.
- [x] Update `.github/workflows/deploy-page.yaml` to build BOTH artifacts and
      combine into one `dist/` for gh-pages: build the landing site
      (`web/`, webpack, with the project-pages `PUBLIC_PATH`) into `dist/`, and
      the Trunk game into `dist/play/`. Coordinate the base path so the project
      Pages subpath (e.g. `/nova-protocol/`) resolves for both the webpack site
      (`PUBLIC_PATH`) and the game (`public_url`). wasm-opt still runs on the
      game wasm.
- [x] Confirm inter-page links and asset URLs resolve under the subpath: built
      with `PUBLIC_PATH=/nova-protocol/` and verified every nav/CTA link, the
      banner `src`, and the brand link point under `/nova-protocol/`, with the
      Play CTA at `/nova-protocol/play/`. NOTE: the Trunk WASM game build + the
      `cp` assemble into `site/play/` run in CI, not locally (AGENTS.md: skip the
      heavy Rust/WASM build locally); the game build is unchanged from the
      previously-working root deploy, only its output directory moves, and the
      relative `public_url` makes it position-independent under the subpath.

### 5. README + docs
- [x] Rewrite `README.md` (currently a 3-line stub): embed `assets/banner.png`
      at the top, then sections mirroring the site but smaller - what Nova
      Protocol is, features, "Play in your browser" (link to the deployed site),
      build/run (native + `trunk serve`), the `web/` landing site (how to dev it),
      and a short project-structure / crate map. Link out to blog/tutorial/wiki.
- [x] Add a `docs/` note per AGENTS.md reflection guidance: what changed, why
      `web/` nests a separate npm project, the deploy dual-build decision,
      difficulties, and self-review.
- [x] CHANGELOG.md Unreleased: add the landing site + game-at-`/play/` entries.

## Open questions
- **Limited "Try" mode:** does "Try" launch a restricted demo (fewer sections /
  a single scenario) vs full "Play"? That needs a game-side flag read from a URL
  param and is out of scope for this task - split into a separate `nova_*` task
  if wanted. For now both CTAs open the full game.
- **Blog content format:** hand-authored HTML pages vs build-time markdown.
  Default to whichever matches football-guessr's existing pattern with least
  glue.
- **CI:** should `web/` get its own CI workflow (format/lint) like
  football-guessr's, or fold into the existing `ci.yaml`? Lightweight separate
  job is probably cleanest; decide during step 4.

## Notes
- Reference art / palette source: `assets/banner.png`.
- Reference stack to copy: `~/personal/football-guessr`.
- Do not touch game (Rust) behavior; this is web + deploy + docs only.
- Content must be accurate to the actual game - source from AGENTS.md,
  CHANGELOG.md, and `docs/` (sections.md, scenario-system.md, architecture.md),
  not invented.

## Close record (2026-07-12)

What shipped: a self-contained `web/` site (TypeScript + Webpack 5 multi-page +
Tailwind v4, mirroring the football-guessr stack) with a hero landing page + Play
gate, tutorial, wiki, and a blog (index + one seed post), themed from
`assets/banner.png`. The deploy (`deploy-page.yaml`) now builds both artifacts
and combines them: the webpack site at the Pages root (`/nova-protocol/`) and the
Trunk game under `/nova-protocol/play/`, with `.nojekyll`. README rewritten (banner
embedded), CHANGELOG entry added, `nodejs_22` added to the flake dev shell, and a
reflection note at docs/retros/20260712-web-landing-site.md.

Verification: `npm run build` (5 pages, partials injected, banner + favicon
copied), `format:check` and `lint` clean; a `PUBLIC_PATH=/nova-protocol/` build
confirmed every link/asset resolves under the subpath; headless-chromium
screenshots of `/` and `/tutorial/` confirmed the banner-derived theme renders;
the active-nav fix was verified by simulation across all five pages. The Trunk
WASM game build + `cp` assemble run in CI (AGENTS.md: skip the heavy Rust/WASM
build locally); the game build is unchanged from the previously-working root
deploy, only its output directory moves.

Review: APPROVE (round 1). R1.1 (active-nav wrong under subpath) and R1.2
(missing `.nojekyll`) fixed; R1.3 (NIT, duplicate one-line entries) left as-is to
match the reference stack. See REVIEW.md.

Not merged: left on `feat/web-landing-page` per the user's "continue on the same
branch" instruction; landing to master + push is the user's call.

Follow-ups (see docs/retros/20260712-web-landing-site.md): wire `web/` checks into CI;
optional limited "Try"/demo mode (game-side URL-param gate); real screenshots/gifs
on the landing page; build-time markdown for the blog if it grows.
