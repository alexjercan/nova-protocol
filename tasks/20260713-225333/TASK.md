# Trim the tutorial to first-scenario onboarding only

- STATUS: CLOSED
- PRIORITY: 25
- TAGS: web,wiki,content

## Goal

Trim `web/src/tutorial.html` to first-scenario onboarding only: what a brand-new
player needs to clear the Shakedown Run and start playing. Keep the intro, the
game-loop/menu overview, and the beat-by-beat Shakedown walkthrough. Move the
reference tables (full keybind reference, targeting/radar rules, weapons
reference, interface reference) out to the corresponding wiki pages (Keybinds,
Targeting & radar, Combat & weapons, HUD); leave a short "full reference lives in
the wiki" pointer. The tutorial should read as a guided first-hour, not a manual.

Done when: the tutorial covers only getting-started + Shakedown, links to the
wiki for reference, `cd web && npm run ci` is green.

## Steps

- [x] Keep the onboarding: intro, main-menu figure, the game-loop/menu
      overview, the full Shakedown Run beat-by-beat (Parts 1-4 with figures),
      and the closing.
- [x] Remove the four reference sections and their tables (Flight controls,
      Targeting and camera, Weapons, Interface) - that reference lives in the
      wiki Keybinds page now.
- [x] Replace them with a short "where to go next" pointer to the wiki
      (Keybinds + the wiki generally), and fix the blockquote wording that
      referenced "every verb on this page".
- [x] Update the <title>/<meta description>/prose__meta so the page reads as a
      first-hour guide, not a manual.
- [x] Build + verify: cd web && npm run ci green; render-check the tutorial.

## Notes

- Spike: tasks/20260713-225157/SPIKE.md.
- Independent of the wiki infrastructure task, but the reference content it
  removes should land somewhere in the wiki (coordinate with 225338/225353).
