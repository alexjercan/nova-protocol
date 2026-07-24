# Add per-OS release download buttons to the landing page hero

- STATUS: OPEN
- PRIORITY: 88
- TAGS: v0.8.1,web,feature

## Story

As a visitor who cannot or does not want to play in the browser, I want native
download buttons for Windows, macOS and Linux on the landing page, so that I can
grab the latest release build for my platform without hunting through GitHub.

The hero already has "Play in browser" and "How to play" CTAs
(`web/src/index.html` lines 40-51). The downloads sit directly below them.

## Steps

- [ ] Add a downloads block in `web/src/index.html` immediately after the
  `.hero__cta` div (below "Play in browser"/"How to play"). Three anchors -
  Windows, macOS, Linux - each with a distinct class (e.g.
  `btn btn--download` plus a `data-os="windows|macos|linux"` hook) and a static
  `href="https://github.com/alexjercan/nova-protocol/releases/latest"` fallback
  so the buttons work with JS disabled. Wrap them in a `.hero__downloads`
  container with a short label ("Or download the native build:").
- [ ] Add `.hero__downloads` and `.btn--download` styles in `web/src/style.css`
  next to the existing `.hero__cta` / `.btn--ghost` rules (reuse the button
  bevel/press idiom; a muted tertiary style so it reads below the primary CTA).
- [ ] Add `web/src/downloads.ts`: progressively enhance the buttons. On load,
  `fetch("https://api.github.com/repos/alexjercan/nova-protocol/releases/latest")`,
  match each asset by suffix (`_windows.zip`, `_macOS.dmg`, `_linux.tar.gz` -
  NOT `_web.zip`), and set each button's `href` to the matched asset's
  `browser_download_url`. On any failure (network, rate limit, missing asset)
  leave the static `releases/latest` fallback in place. Keep it dependency-free
  and defensive, mirroring the style of `web/src/webgpu.ts`.
- [ ] Import and call the enhancer from `web/src/index.ts` (alongside
  `initSite()` / `warnIfNoWebGpu()`).
- [ ] Write `tasks/20260724-074940/DECISION.md` recording why progressive
  enhancement over a runtime GitHub API fetch (not static `/latest/download/`
  links, not a release-pipeline change) - see the pre-seeded record.

## Definition of Done

- The rendered landing page shows three download buttons (Windows, macOS, Linux)
  below the existing CTAs (cmd: `grep -n 'data-os' web/src/index.html`).
- With JS disabled, every download button still points at the latest release
  (cmd: `grep -c 'releases/latest' web/src/index.html`).
- The enhancer is wired into the index bundle
  (cmd: `grep -n downloads web/src/index.ts`).
- The web build succeeds with the new module
  (cmd: `cd web && npm run build`).
- manual: with JS enabled on a released build, clicking each button starts the
  correct-platform asset download from the latest GitHub release (verified after
  the next release, or against v0.8.0 by temporarily loading the built page).

## Notes

- Repo: `alexjercan/nova-protocol`. Latest release assets are named
  `nova-protocol_v<VERSION>_windows.zip`, `_macOS.dmg`, `_linux.tar.gz`,
  `_web.zip` (verified via `gh release view --json assets`). The version is
  embedded in the filename, which is exactly why static `/latest/download/<name>`
  links cannot work across releases - see DECISION.md.
- The web app is a webpack multi-page build; one `HtmlWebpackPlugin` per page,
  index entry is `web/src/index.ts` (`web/webpack.config.js:283`).
- `basePath` is templated in HTML via `<%= htmlWebpackPlugin.options.basePath %>`;
  external GitHub URLs are absolute and need no templating.
- Precedent for DOM/progressive-enhancement JS on the hero: `web/src/webgpu.ts`
  (idempotent, defensive, feature-tested).
- Assumption: unauthenticated GitHub API (60 req/hr/IP) is acceptable for a
  landing page; failures degrade gracefully to the release page.
