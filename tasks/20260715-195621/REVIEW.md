# Review: publish developer docs as a markdown-first wiki

- TASK: 20260715-195621
- BRANCH: 20260715-195621-wiki-docs

## Round 1

- VERDICT: REQUEST_CHANGES

Out-of-context review pass plus reviewer's own load-bearing checks
(slug parity webpack<->manifest<->files, deploy-subpath build resolving all
basePath links under /nova-protocol/, headless render of graph/sequence/flow
diagrams + highlight.js + sidebar). Pipeline design is sound; the flagged
hazards (the `$` replace-pattern, the mermaid verbatim-fence contract, CJS/ESM
interop) are handled correctly. Findings below.

- [x] R1.1 (MAJOR) web/src/wiki/dev/architecture.md:16,19,21 - intra-doc links
  still use old relative `.md` filenames (`[scenario-system.md](scenario-system.md)`,
  `[modding-ron-format.md](modding-ron-format.md)`, `[mod-portal.md](mod-portal.md)`).
  Served at `/wiki/dev/architecture/`, `href="scenario-system.md"` resolves to
  `/wiki/dev/architecture/scenario-system.md` -> 404 (and `modding-ron-format.md`
  no longer exists - renamed to `modding-ron.md`). Rewrite to wiki URLs:
  `[Scenario engine](../scenario-system/)`, `[Modding data format (RON)](../modding-ron/)`,
  `[Mod portal](../mod-portal/)`.
  - Response: Fixed. Rewrote all three to the wiki URLs. Built output confirms
    `href="../scenario-system/"`, `"../modding-ron/"`, `"../mod-portal/"`.
- [x] R1.2 (MAJOR) web/src/wiki/dev/scenario-system.md:150 - same bug:
  `[sections.md](sections.md)` -> 404. Point at the dev internals page
  `[Ship sections (internals)](../sections/)`.
  - Response: Fixed. Now `[Ship sections (internals)](../sections/)`; build
    confirms `href="../sections/"`.
- [x] R1.3 (MAJOR) assets/mods/demo/README.md:32 - a LIVE (non-tasks) file the
  sweep missed still says "see `docs/modding-ron-format.md`" (the `docs/`-prefixed
  grep matched, but this bare mod README slipped the review of results). Update to
  `web/src/wiki/dev/modding-ron.md`.
  - Response: Fixed. Points at `web/src/wiki/dev/modding-ron.md`.
- [x] R1.4 (MINOR) web/src/wiki.ts:initMermaid + web/src/style.css `.prose
  pre.mermaid { color: transparent }` - fire-and-forget `void initMermaid()` with
  no error handling: if the mermaid chunk fails to load or `mermaid.run` throws,
  the `<pre class="mermaid">` never gets `data-processed`, so the source stays
  `color: transparent` permanently (invisible) plus an unhandled rejection. Wrap
  import+run in try/catch and reveal the source on failure.
  - Response: Fixed. `initMermaid` wraps import+run in try/catch; on failure it
    adds `.mermaid--failed` to every unprocessed block, and CSS reveals the raw
    source (`color: inherit; text-align: left`) for that class.
- [x] R1.5 (NIT) web/src/style.css `.prose pre.mermaid` - inherits the generic
  `.prose pre` code-block frame (cyan left border, panel bg, shadow) and only
  partially overrides it. Reset background/box-shadow/border for `pre.mermaid` so
  the diagram is framed as a figure, not a code block.
  - Response: Fixed. `pre.mermaid` now sets its own `background`/`border`/
    `box-shadow` (figure frame) rather than partially inheriting the code frame.
- [x] R1.6 (MINOR) [scope, user feedback] web/src/wiki/dev/bevy-migration.md - the user
  redirected: the Bevy migration notes are historical and should stay in `docs/`,
  not the wiki. Move it back to `docs/bevy-0.19-migration.md`, drop its
  WIKI_DOC_PAGES + manifest entries, and re-list it under docs/README.md and
  AGENTS.md.
  - Response: Done. `git mv` back to `docs/bevy-0.19-migration.md`; removed its
    WIKI_DOC_PAGES + manifest entries; re-listed under docs/README.md and
    AGENTS.md. Build now emits 6 dev pages (no bevy).

## Round 2

- VERDICT: APPROVE

All Round 1 findings resolved and verified: no bare `*.md` links remain in
`web/src/wiki/dev/` (grep clean); the built architecture/scenario pages emit the
corrected relative wiki URLs; the only surviving old-path reference is
`LESSONS.md:517`, a dated historical ledger entry, left as-is by design.
`npm run ci` (format:check + eslint + build) green; 6 `dev/*` pages emit. Mermaid
now degrades gracefully on load/parse failure. Merge-ready.
