# Review: wiki infrastructure (20260713-225324)

- DATE: 20260713-225324
- VERDICT: APPROVE (round 2)

## Round 2

- Finding 1 fixed: the seven unbuilt pages are now `comingSoon` (8 total with
  modding); the sidebar renders coming-soon entries as non-link spans and See
  also skips them, so no dead links. Content tasks flip the flag as pages land.
- Finding 2 fixed: `aria-current="page"` set on the active sidebar link/home.
- Finding 3: accepted.
- `npm run ci` green; TS compiles (types check via ts-loader).
- Caveat: the chrome is client-rendered and no headless-DOM tooling is
  installed, so runtime was not browser-verified in CI - recommend an eyeball on
  the served site. Logic reviewed by hand (placeholder ids match, slug/base
  parsing handles "/" and "/nova-protocol/").

APPROVE.

---

## Round 1

- Round 1 VERDICT (superseded by the round 2 APPROVE above): REQUEST_CHANGES

Reviewed the manifest (`wiki-pages.ts`), the chrome injector (`wiki.ts`), the
wiki CSS, the manifest-rendered index, the two sample sub-pages, and the
`wikiPage()` webpack wiring against the task Goal and spike Option B. The
architecture is right and builds green, but one correctness bug blocks approve.

## Findings

### 1. [Major] Sidebar + See also link to unbuilt pages -> 404
Only `sections` and `keybinds` have HTML this task, but the manifest lists all
10 pages and only `modding` is flagged `comingSoon`. So the sidebar renders
navigable links to `hud`, `flight-autopilot`, `targeting-radar`,
`combat-weapons`, `gravity-wells`, `factions`, `scenarios` - all 404 - and
`sections`' See also links `combat-weapons` / `hud` the same way. Fix: mark
every not-yet-authored page `comingSoon: true` (all but sections/keybinds), and
make the chrome treat coming-soon pages as non-navigable - render them as
non-link spans in the sidebar and omit them from See also - so there are no dead
links. The content tasks (225338/225353) flip `comingSoon` off as each page's
HTML lands.

### 2. [Minor] Active sidebar link has no `aria-current`
The active page is styled but not marked for assistive tech. Add
`aria-current="page"` on the active sidebar link/home entry.

### 3. [Note, accept] `WIKI_SLUGS` duplicates the manifest slugs
`webpack.config.js` can't import the TS manifest, so the built-page slug list is
repeated there. Accepted tradeoff (documented with a "keep in sync" comment);
the coming-soon flag means a missing registration shows as a stub, not a crash.

## Verdict

REQUEST_CHANGES: fix 1 and 2, rebuild. 3 is accepted.
