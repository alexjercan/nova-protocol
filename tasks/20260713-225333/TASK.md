# Trim the tutorial to first-scenario onboarding only

- STATUS: OPEN
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

## Notes

- Spike: tasks/20260713-225157/SPIKE.md.
- Independent of the wiki infrastructure task, but the reference content it
  removes should land somewhere in the wiki (coordinate with 225338/225353).
