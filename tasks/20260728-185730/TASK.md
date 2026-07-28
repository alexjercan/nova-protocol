# Web easter egg: 5x-click -> reworked main menu -> HUD -> NOVA OS CRT

- STATUS: OPEN
- PRIORITY: 30
- TAGS: v0.9.0,ui,web,easter-egg

## Story

Owner request (2026-07-28, during the UI-rework spike): the reworked main-menu
HTML PoC (`examples/ui/nova_ui_rework_poc.html`) looks good enough to ship as a
hidden web easter egg. Today the landing-page easter egg (5 brand-clicks within
1.5s, `web/src/site.ts` `initEasterEgg`) jumps straight to the NOVA OS terminal
PoC copied to `/nova-os/` (webpack `CopyPlugin`). Make the 5x-click open the new
main-menu PoC instead, and chain it like the real game does: main menu -> New
Game -> the flight HUD PoC (`hud_rework_poc.html`) -> a "NOVA OS" button on the
HUD (the Tab-computer affordance) that opens the CRT terminal. Full chain:
brand 5x -> `/nova-menu/` -> New Game -> `/nova-hud/` -> NOVA OS button ->
`/nova-os/`.

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

- [ ] Webpack `CopyPlugin`: copy `../examples/ui/nova_ui_rework_poc.html` ->
      `nova-menu/index.html` and `../examples/ui/hud_rework_poc.html` ->
      `nova-hud/index.html` (mirror the existing nova-os entry). The HUD PoC
      references relative icon assets, so ALSO copy `../examples/ui/assets` ->
      `nova-hud/assets` (the menu + CRT PoCs are asset-free). Add
      `historyApiFallback` rewrites for `/nova-menu` and `/nova-hud`.
- [ ] Point the easter egg at the new route: change `EGG_ROUTE` in
      `web/src/site.ts` from `nova-os` to `nova-menu` so 5 brand-clicks open the
      menu. Keep `registerHit` pure; update the two `site.test.ts` assertions
      (`/nova-os/` -> `/nova-menu/`, and the basePath variant).
- [ ] Menu PoC buttons (immersive mode): "New Game" -> `../nova-hud/`;
      Scenarios / Mods / Settings / Pause open their existing overlay screens in
      place (wire the corner buttons to `showScreen`); Exit -> site root (`../`).
      Relative navigation so it works under the project-pages base path.
- [ ] HUD PoC: add a "NOVA OS" button in the flight HUD (styled as the game's
      Tab-computer affordance, top-right near the objective hint) that navigates
      to `../nova-os/`. Add a small "Menu" back affordance -> `../nova-menu/`.
- [ ] CRT return (like the game's Esc-resumes-flight): decided at gate -
      either forward-only (browser back returns to the HUD) or a param-guarded
      exit that navigates `../nova-hud/`, leaving `nova_os_terminal_poc.html`'s
      default (paramless) behaviour untouched.
- [ ] Clean-immersive mode: hide the demo topbar/deck chrome when served on the
      `/nova-menu/` and `/nova-hud/` routes (detect the standalone route), keep
      it for the in-repo `file://` review copies. Default the menu to phosphor.
- [ ] Add a "UI skin" setting (Phosphor / Hardware segmented) to the menu PoC
      Settings under a Display/Interface section; drives `body[data-skin]` and
      persists to localStorage (survives the menu -> HUD -> CRT -> back hops,
      same origin).
- [ ] Docs: note the hidden routes wherever the nova-os easter egg is noted, if
      anywhere player-facing; otherwise leave unlinked like `/nova-os/`.

## Definition of Done

1. cmd: `npm run build` in `web/` emits `dist/nova-menu/index.html`,
   `dist/nova-hud/index.html` (+ its `assets/`), and still `dist/nova-os/index.html`.
2. test: `web/tests/site.test.ts` covers the egg route change (5th hit ->
   `/nova-menu/`, basePath variant), still green.
3. manual: on a local `webpack serve`, 5 quick brand-clicks open the main menu;
   New Game opens the HUD; the HUD's NOVA OS button opens the CRT; menu overlays
   (Settings/Mods/Scenarios/Pause) work in place; the demo chrome is hidden on
   the deployed routes but present in the repo files; the Settings skin switch
   persists.
4. Not pushed/deployed; owner runs the web deploy separately.

## Flow State

- FLOW STEP: PLANNED
- PLAN STATUS: APPROVED

## Notes

Single source of truth stays the `examples/ui/*.html` PoCs (same pattern as the
NOVA OS PoC). This is an extra on top of the epic's Done Means, not one of its
acceptance criteria.
