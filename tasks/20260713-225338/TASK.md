# Author the gameplay-system wiki pages (sections, keybinds, HUD, flight, targeting, combat, gravity)

- STATUS: OPEN
- PRIORITY: 20
- TAGS: web,wiki,content

## Goal

Author the gameplay-system wiki sub-pages with real content pulled from the
code (not guessed): Sections, Keybinds, HUD, Flight & autopilot, Targeting &
radar, Combat & weapons, Gravity wells. Each: full reference content, a manifest
entry (category/tags/related/headings), `.figure` image placeholders, and the
webpack registration via the `wikiPage()` helper.

Done when: all seven pages are live through the manifest (sidebar/search/tags/
see-also working), content is code-accurate, `cd web && npm run ci` is green.

## Notes

- Spike: tasks/20260713-225157/SPIKE.md. Depends on 20260713-225324 (infra).
- Pull the real data from the code: keybinds from `nova_gameplay` input rig,
  sections from `nova_assets`/section modules, damage types + resistances,
  radar/lock rules, HUD tiers. Verify against source, do not invent.
