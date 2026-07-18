# Story campaign mod (The Ledger) polish + extension: longer, more interesting narrative arc; re-publish the portal mod

- STATUS: OPEN
- PRIORITY: 48
- TAGS: v0.8.0,content,modding,scenario

## Goal

Polish and lengthen the story-campaign mod "The Ledger" (the alternative
storyline you install from the portal). It already ships 5 chapters
(`webmods/the-ledger/ledger_ch1..ch4 + ch2b.content.ron`) - a four-chapter
salvage story with comms-driven beats and a two-ending finale. Make it richer
and longer, and re-publish the portal bundle. Data/scenario + mod-resource work
only; no new engine features.

## Steps

- Playtest the full Ledger chain; note pacing/narrative/difficulty weak spots
  and any dangling or unclear beats (ch2b checkpoint, the ch4 branch).
- Deepen the arc: additional chapter(s) or expanded existing ones, stronger
  narrative beats, more encounter variety, and payoff on the two-ending choice.
  Lean on the v0.7.0 authoring stack (Outcome frames, allegiance, comms).
- If the asset-variety pack + mod-relative resources landed in v0.7.0, give
  chapters their own look (skybox/texture variety) via mod-carried resources.
- Re-run `content lint --target the-ledger` + audit (20260718-152240); fix
  findings; bump the mod version and re-publish through the portal generator
  (Rust bin or the new Python script 20260718-152247).

## Notes

- Dogfoods the portal pipeline end to end (multi-file bundle), which is also
  the best test of the modding platform.
- Keep it installable from the Explore-online tab; verify the published catalog
  entry and thumbnails.

