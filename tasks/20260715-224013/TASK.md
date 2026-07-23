# Player docs: getting-started front door, tutorial promotion, glossary, jargon trim

- STATUS: CLOSED
- PRIORITY: 48
- TAGS: docs,web,feature

From the docs review spike 20260715-223147 (player persona). The player wiki is
strong and accurate but has no "start here" front door, the tutorial is only
reachable via inline links, terms/units are undefined, and a few pages leak
control-theory jargon.

## Goal

Give a new player an on-ramp: a getting-started page, a surfaced tutorial, a
small glossary (incl. the `u`/`u/s` unit), and player-appropriate prose.

## Steps

- [x] Write `web/src/wiki/getting-started.md` ("Your first flight"): launch ->
      New Game (loads Shakedown Run) -> the first two minutes (burn with `W`,
      hold `Ctrl` to sweep/lock, `G` to GOTO, raise weapons + fire), then "where
      to go next". Prominently link the tutorial (`../../tutorial/`) and Keybinds.
- [x] Write `web/src/wiki/glossary.md`: define the recurring terms (prograde/
      retrograde, standoff, sphere of influence, hysteresis, fine-lock, "hot"
      weapons, diegetic) and state the distance/speed unit: distances are units
      (`u`), speeds `u/s`. Keep it short.
- [x] Register both pages: add entries to `WIKI_DOC_PAGES` in
      `web/webpack.config.js` (`{slug:"getting-started", md:"getting-started.md",
      ...}`, likewise glossary) and `WikiPage` entries in `web/src/wiki-pages.ts`.
- [x] Put them first: add a "Start here" category at the FRONT of the
      "For players" band in `WIKI_SECTIONS` (`web/src/wiki-pages.ts`) holding
      getting-started + glossary, so they lead the player nav and index.
- [x] Surface the tutorial in the nav. The tutorial is a separate page at
      `/tutorial/` (`web/src/tutorial.html`), not a wiki manifest page.
      verify-first: read `renderSidebar` in `web/src/wiki.ts` and decide -
      either (a) let a `WikiPage` carry an optional external `href` and render it
      as a plain nav link, or (b) keep the tutorial out of the manifest but make
      getting-started its prominent front door + a "Tutorial" see-also. Prefer
      (b) unless (a) is a two-line change.
- [x] Trim control-theory jargon (keep the behavior, drop the nouns) in
      `web/src/wiki/flight-autopilot.md` ("PD attitude loop", "solves every tick
      a small allocation", "nulling the net torque") and
      `web/src/wiki/sections/controller.md` ("proportional-derivative (PD)
      controller") - e.g. "an asymmetric ship still flies straight" stays, the
      control-theory framing moves to a brief aside or is cut.
- [x] Define `u`/`u/s` at first use in `web/src/wiki/hud.md` and
      `web/src/wiki/flight-autopilot.md` (one clause, or a link to the glossary).
- [x] (separable, image work) Annotate `web/src/assets/wiki-hud.png` and
      `wiki-radar.png` with callout labels so the visual pages are
      self-explanatory. Can be split to its own follow-up if image tooling is a
      blocker. DEFERRED to follow-up task 20260715-231500.
- [x] Verify: `npm run ci` green; serve + headless-eyeball that getting-started
      and glossary render and lead the For-players nav/index, the tutorial link
      resolves, and the trimmed pages still read well; check the deploy subpath.

## Notes

- Manifest model (from 195621 / 204358 / 215924): pages live in `WIKI_PAGES`
  (`wiki-pages.ts`) with a `category`; categories are grouped into audience bands
  by `WIKI_SECTIONS`; each renderable page is also registered in `WIKI_DOC_PAGES`
  (`webpack.config.js`). Follow that pattern for the two new pages.
- All player figure screenshots already exist and auto-upgrade (site.ts); no
  capture work needed except the optional annotation step.
- Keep the sharp house style; player voice, not engine jargon.
