# Hidden NOVA OS terminal easter egg in the web app

- PRIORITY: 8
- TAGS: v0.9.0, feature, web, ui
- KIND: TASK
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE

## Story

The `examples/ui/nova_os_terminal_poc.html` is a fully self-contained NOVA OS
terminal HUD prototype (inline CSS + one inline `<script>`, no external or
relative asset refs). Surface it in the marketing web app as a hidden easter
egg: ship it at a secret, unlinked route and let a curious visitor open it by a
secret gesture. The source of truth stays in `examples/ui/`; the web build
copies it in at build time so the file is never duplicated in source.

Decisions (see DECISION.md):

- Secret route: `/nova-os/` (thematic, not linked from any nav).
- Trigger: clicking the site brand/logo 5 times rapidly. The brand is a link to
  home, so the handler only arms when the brand already points at the current
  page (i.e. you are on the landing page, where re-clicking home is otherwise a
  no-op reload); there it swallows the clicks and counts them, and the 5th
  within the rolling window navigates to `<basePath>nova-os/`. On any other page
  the brand keeps its normal "go home" behavior untouched.

## Steps

- [x] Add a CopyPlugin pattern in `web/webpack.config.js` copying
  `../examples/ui/nova_os_terminal_poc.html` to `nova-os/index.html` in the
  build output (source truth stays in `examples/`).
- [x] Add a dev-server `historyApiFallback` rewrite for `^/nova-os` so the route
  resolves without a trailing slash during `webpack serve`.
- [x] Implement the secret brand-click hotspot in `web/src/site.ts` (a small
  `initEasterEgg`-style helper wired from `initSite`): only arm when the brand
  resolves to the current path, count clicks within a rolling window, navigate
  to the basePath-aware `/nova-os/` on the 5th.
- [x] Verify the built `dist/nova-os/index.html` matches the examples source and
  the PoC renders standalone (headless chromium screenshot); the click gesture
  (arming guard, 5-click trigger, basePath, off-page no-op) is covered by the
  `npm test` node harness against a fake DOM.
- [x] `npm run ci` in `web/` passes (format:check + lint + build). Added an
  `npm test` script (pure `registerHit` + `initEasterEgg` wiring) per the
  `ci-skips-client-render` lesson - CI stays build-only.

## Definition of Done

1. `examples/ui/nova_os_terminal_poc.html` remains the only source copy; grep
   shows no second copy checked into `web/src/`
   (cmd: `git ls-files web | grep -i nova_os_terminal_poc; test $? -ne 0`).
2. A production build emits the PoC at the secret route
   (cmd: `cd web && npm run build && test -f dist/nova-os/index.html && diff -q dist/nova-os/index.html ../examples/ui/nova_os_terminal_poc.html`).
3. Web CI is green (cmd: `cd web && npm run ci`).
4. Clicking the brand 5x on the landing page opens `/nova-os/`; the brand still
   navigates home normally from other pages (manual: click-test in `npm run serve`).
