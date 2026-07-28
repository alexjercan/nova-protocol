# Web easter egg: 5x-click opens the reworked main menu -> NOVA OS CRT

- STATUS: OPEN
- PRIORITY: 30
- TAGS: v0.9.0,ui,web,easter-egg

## Story

Owner request (2026-07-28, during the UI-rework spike): the reworked main-menu
HTML PoC (`examples/ui/nova_ui_rework_poc.html`) looks good enough to ship as a
hidden web easter egg. Today the landing-page easter egg (5 brand-clicks within
1.5s, `web/src/site.ts` `initEasterEgg`) jumps straight to the NOVA OS terminal
PoC copied to `/nova-os/` (webpack `CopyPlugin`). Make the 5x-click open the new
main-menu PoC instead, and wire its "New Game" button to navigate on to the CRT
terminal - a two-step chain: brand 5x -> main menu -> New Game -> NOVA OS CRT.

Depends on the UI-rework spike (20260728-175726) landing first so
`nova_ui_rework_poc.html` is on master. Extends the UI-rework epic
(20260728-175719). Owner decisions (2026-07-28):

- Scope is WIRE-INTO-SOURCE ONLY: add the webpack copy + site.ts change + working
  buttons so it builds and ships on the next normal web deploy. Do NOT push or
  run the deploy (owner drives that via /release).
- Deployed presentation is CLEAN IMMERSIVE: hide the demo topbar (screen switcher,
  POC tag, skin toggle) so it is just the live scene + corner menu; the corner
  buttons navigate.
- The Phosphor/Hardware skin switch does NOT vanish - it moves INTO the PoC's
  Settings screen as a real setting (a "UI skin" segmented control under a
  Display/Interface section), so visitors can still flip the look the way a
  player would, without any dev chrome.

## Steps

- [ ] Add a webpack `CopyPlugin` pattern copying
      `../examples/ui/nova_ui_rework_poc.html` -> `nova-menu/index.html` (mirror the
      existing `nova_os_terminal_poc.html` -> `nova-os/index.html` entry), plus the
      `webpack serve` `historyApiFallback` rewrite for `/nova-menu`.
- [ ] Point the easter egg at the new route: change `EGG_ROUTE` (or add wiring) in
      `web/src/site.ts` so 5 brand-clicks navigate to `/nova-menu/` instead of
      `/nova-os/`. Keep `registerHit` pure; update its unit test expectations.
- [ ] Make the PoC's buttons work: "New Game" -> `/nova-os/` (CRT terminal);
      Scenarios/Mods/Settings/Pause open their existing overlay screens in place;
      Exit -> back to the site root. Use relative navigation so it works under the
      project-pages base path (`/nova-protocol/...`) as well as local dev.
- [ ] Clean-immersive mode in the PoC: hide the demo topbar when served as the
      easter egg (e.g. a body flag / `?app` param the copied route sets, or detect
      the standalone route), defaulting to the phosphor skin. Keep the topbar for
      the in-repo review copy so the spike demo stays usable.
- [ ] Add a "UI skin" setting (Phosphor / Hardware segmented control) to the PoC
      Settings screen under a Display/Interface section; it drives the same
      `body[data-skin]` the topbar toggle used, and persists the pick in
      localStorage so it survives the New Game -> CRT -> back hop.
- [ ] Docs: note the new hidden route where the NOVA OS easter egg is documented,
      if anywhere player-facing; otherwise leave it unlinked like `/nova-os/`.

## Definition of Done

1. cmd: `npm run build` in `web/` emits `dist/nova-menu/index.html` and still
   emits `dist/nova-os/index.html`.
2. test: `web/tests/site.test.ts` covers the egg route change (5th hit ->
   `/nova-menu/`), still green.
3. manual: on a local `webpack serve`, 5 quick brand-clicks open the new main
   menu; New Game opens the CRT terminal; other buttons open their overlays;
   the demo topbar is hidden in the deployed route but present in the repo file;
   Settings has a working Phosphor/Hardware skin switch whose pick persists.
4. Not pushed/deployed; owner runs the web deploy separately.

## Notes

Single source of truth stays `examples/ui/nova_ui_rework_poc.html` (same pattern
as the NOVA OS PoC). This is an extra on top of the epic's Done Means, not one of
its acceptance criteria.
