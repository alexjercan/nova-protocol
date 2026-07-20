# Document the release + code->docs sync workflow (wiki dev page + AGENTS.md rule + LESSONS)

- STATUS: CLOSED
- PRIORITY: 65
- TAGS: docs,web,process

## Outcome

Wrote the release + code->docs sync workflow as a published dev wiki page and
wired it into the process rules:

- New page `web/src/wiki/dev/keeping-docs-in-sync.md` (`/wiki/dev/keeping-docs-in-sync/`):
  the documentation surfaces (CHANGELOG, News, player wiki, dev wiki, tutorial),
  the "a code change isn't done until its docs are" rule with a three-question
  check, a code-area -> docs **dependency map** table, the release flow, and the
  page/post registration reminder. Registered in `web/webpack.config.js`
  (WIKI_DOC_PAGES) and the manifest `web/src/wiki-pages.ts` (For developers ->
  Get started), cross-linked with `dev/development.md`.
- AGENTS.md: rewrote the stale "The website" section (it still described the old
  blog/devlog flow and `tutorial.html`) to the News workflow + the docs-sync
  rule + a pointer to the new page; added the page to the "Docs, tasks,
  versioning" list.
- development.md: fixed the release checklist step 8 ("devblog" -> News post +
  wiki sync).
- docs/README.md: indexed the new page.
- LESSONS.md: added `keep-docs-in-sync-with-code`.

Verified: `npm run build` green, the page emits with both tables and appears in
the manifest-driven dev sidebar; all 22 wiki pages and 11 crates the map names
exist (no dead references); ASCII clean. This task was itself an instance of the
documented problem - the /news/ merge had left "devblog" wording stale in
AGENTS.md and development.md.

## Steps

- [x] 1. Write `web/src/wiki/dev/keeping-docs-in-sync.md` (surfaces, code-change
      rule, dependency map, release flow, registration reminder).
- [x] 2. Register it: `WIKI_DOC_PAGES` (webpack) + `wiki-pages.ts` manifest.
- [x] 3. Update AGENTS.md: rewrite the stale website section to News + docs-sync
      rule + pointer; list the page under Docs/versioning.
- [x] 4. Fix development.md release step 8; index the page in docs/README.md.
- [x] 5. Add the LESSONS ledger entry.
- [x] 6. Build green; verify page renders, sidebar lists it, no dead refs, ASCII.

## Notes

Follows up the /news/ merge (20260716-111557) and TOC/expansion (20260716-114245).
Convention: durable dev reference lives in the wiki (`web/src/wiki/dev/`), the
enforcement rule lives in AGENTS.md; `docs/` stays transient (per docs/README.md).
