# Web landing site (play gate, blog, tutorial, wiki)

Date: 2026-07-12
Task: 20260712-093048
Branch: feat/web-landing-page

## What changed

The GitHub Pages deploy used to publish the raw Bevy WASM game at the site root.
It now publishes a themed static content site that fronts the game:

- A new self-contained web project under `web/` (TypeScript + Webpack 5
  multi-page + Tailwind v4 via PostCSS), mirroring the stack of
  `~/personal/football-guessr`.
- Pages: `/` (hero landing with a Play gate), `/tutorial/`, `/wiki/`, `/blog/`
  and one seed post at `/blog/building-nova-protocol/`.
- Theme derived from `assets/banner.png`: deep space-navy field, neon-cyan
  "NOVA" glow, warm amber "PROTOCOL"/horizon glow. Tokens live at the top of
  `web/src/style.css`.
- `deploy-page.yaml` now builds both artifacts and combines them: the webpack
  site at the Pages root (`/nova-protocol/`) and the Trunk game under
  `/nova-protocol/play/`.
- `README.md` rewritten (was a 3-line "Bevy Systems" stub) with the banner
  embedded and sections mirroring the site but smaller.

## Decisions and tradeoffs

- **`web/` nests a separate npm project rather than living at the repo root.**
  The repo root is a Cargo workspace; a root-level `package.json` + `node_modules`
  would collide conceptually with that and pollute the Rust project. Nesting
  keeps the two toolchains cleanly separated. Cost: the deploy needs two build
  steps and an assemble step, and `web/` carries its own lint/format config.

- **Game moves to `/play/`, keeps Trunk's relative `public_url = "./"`.**
  Relative asset URLs are position-independent, so the game build works unchanged
  whether it is served from `/` or `/nova-protocol/play/`. That means `Trunk.toml`
  did not need a subpath baked in - only the webpack site needs `PUBLIC_PATH`
  (its links are absolute from `basePath`). Alternative considered: bake
  `--public-url /nova-protocol/play/` into the Trunk build; rejected because the
  relative form is simpler and survives a rename of the Pages path.

- **Combine into one gh-pages artifact rather than two Pages sites.** GitHub
  project Pages serves a single site per repo, so the landing and game must share
  one `dist/`; the workflow copies `web/dist` to `site/` and the Trunk `dist` to
  `site/play/`.

- **`web/`'s CI = `format:check && lint && build`.** football-guessr's `ci` also
  runs Jest, but this site is static content with almost no logic to unit-test,
  so a successful production build is the meaningful smoke test. If real page
  logic grows later, add Jest then. The `web/` checks are not yet wired into the
  repo's Rust `ci.yaml` - see follow-ups.

- **"Try" vs "Play".** The task floated a limited-capabilities demo. Both CTAs
  open the full game for now; a real demo mode needs a game-side (Rust) URL-param
  flag and belongs in its own `nova_*` task. Left as an open follow-up rather
  than half-built.

## Verification

- `npm run build` succeeds (5 pages emitted, header/footer partials injected,
  banner + favicon copied).
- `npm run format:check` and `npm run lint` clean.
- Production build with `PUBLIC_PATH=/nova-protocol/` confirmed every inter-page
  link, the banner `src`, and the brand link resolve under the subpath.
- Visual: rendered `/` and `/tutorial/` under headless chromium (1200px wide).
  Landing shows the banner hero with the amber horizon glow, cyan/amber CTAs and
  the feature-card grid; tutorial shows the prose layout with `kbd`-styled keys
  and the control tables. Both match the banner palette.
- The combined deploy (Trunk game under `/play/`) was NOT run locally - the Rust
  WASM build + wasm-opt happen in CI. The assemble step is plain `cp`; the game
  build itself is unchanged from the previously-working root deploy, only its
  output directory moves.

## Difficulties

- Node is not on the NixOS PATH and this repo's flake is Rust-only. Used a full
  `nodejs` from the nix store directly (`/nix/store/...-nodejs-22.22.3/bin`) to
  install and build. CI uses `actions/setup-node`, so this is a local-only
  wrinkle, but worth noting for the next web session here: `web/` has no Node in
  the default dev shell.

## Self-reflection

- Copying football-guessr's `webpack-partials.js` verbatim was the right call -
  the header/footer injection + `<%= basePath %>` substitution is exactly what a
  multi-page static site wants, and reusing it kept the base-path handling
  consistent between the template-level `htmlWebpackPlugin.options.basePath` and
  the partial-level `basePath`.
- Sourcing all game copy from a content-extraction pass over the real source
  (controls from `input/player.rs`, sections from `docs/sections.md`) kept the
  tutorial/wiki honest - no invented keybinds. Worth repeating for any
  player-facing text.
- One wording bug ("Three ship today") slipped into the wiki draft and was caught
  on re-read, not by any tool. Prose pages have no test; a read-through is the
  only guard.

## Follow-ups

- Wire `web/` format+lint (and build) into repo CI, or add a dedicated web CI
  workflow.
- Optional limited "Try"/demo mode: game-side URL-param feature gate.
- Real screenshots/gifs on the landing page instead of only the banner.
- Blog is single-post + hand-authored HTML; if it grows, consider build-time
  markdown rendering.
