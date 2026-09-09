# Season-first browsing

## Delta

The user clarified and approved a multi-page hierarchy:

1. `/story/` lists seasons only. Each season has one illustrated row with its
   title, summary, selected episode count, and labelled draft count.
2. `/story/<season>/` lists that season's episodes only, in authored order.
   Story returns to the season library. There is no repeated season cover or
   Preview, Start, Latest, or Season contents shortcut.
3. The reader returns to the episode list through the season title, such as
   Season 1. Pages, Panels, Transcript, exports, and reading controls remain.

This replaces the grouped-library design in `NAVIGATION-REVIEW.md`. That report
and its proof remain frozen. Season redirects and the library anchor-height
observer are removed, rather than kept as a second navigation path.

The source guide, developer chapter, and documentation map describe the new
hierarchy. The existing 184-character Story changelog entry already covers
season/episode browsing and remains unchanged.

## Verified

New evidence is under [proof/season-browser/](proof/season-browser/). The new
build, browser, and source-check harnesses do not write earlier proof.

- Targeted format, lint, TypeScript (`--skipLibCheck`), catalog/route, and
  DSL/build checks pass. Full website CI and Rust checks were not run.
- Catalog tests cover numeric season order, authored episode order independent
  of ids, one row per destination, selected counts/covers, mixed publication,
  draft-only season exclusion, cross-season episode links, and escaped titles.
- Static local builds pass at root and `/nova-protocol/`. Browser checks at
  1440 and 390 px cover season selection, episode selection, keyboard activation,
  browser Back/Forward at both levels, labelled return links, and page reload.
- Screenshots show seasons only in the library, episodes only on the season
  page, no duplicate promotional controls, and no horizontal overflow.
- All seven full-page SVG exports remain pixel-identical to the accepted
  renders. The 19 panel exports, modal focus and scrolling, measured layout,
  automatic wrapping, and unsafe SVG rejection still pass.
- Root and prefixed public builds show the empty archive. No draft season page,
  episode route, Story asset, or private script data enters them.
- mdBook passes with the existing Mermaid 0.5.0/0.5.2 version warning.

No server or debugging port was started. Inspection browsers were stopped by
owned PID. No commit, push, publication, or live server reload was performed.
The episode remains a draft with seven of eighteen pages illustrated. Story
writing and the remaining illustration work stay open under the same task.

## Next

Restart an existing development server to load the changed build templates.
