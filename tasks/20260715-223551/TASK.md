# Fix stale modding-ron doc: scenarios are *.content.ron not *.scenario.ron

- STATUS: OPEN
- PRIORITY: 55
- TAGS: bug,docs,web

From the docs review spike 20260715-223147. The creators front door
(`web/src/wiki/modding.md`) advertises `web/src/wiki/dev/modding-ron.md` as "the
RON data format reference", but that page is stale-WRONG on the core format: a
creator who follows it authors a file the loader never loads.

## The bug (verified)

`web/src/wiki/dev/modding-ron.md` says scenarios are `*.scenario.ron` files under
`assets/scenarios/` at lines 7, 13, 36, 38, 93, 292. Reality:
- `find assets -name '*.scenario.ron'` -> 0 files.
- Every shipped scenario is a `Scenario((...))` item inside a `*.content.ron`
  (`assets/base/scenarios/{asteroid_field,shakedown_run,menu_ambience,...}.content.ron`),
  loaded as `Content::Scenario` (see the accurate `guide-author-scenario.md`).

The page also reads as a changelog (serde rationale, `AssetRef` trait bounds,
`VisitAssetDependencies`) rather than a field reference.

## Steps

- [ ] Rewrite the format claims in `modding-ron.md` to the real
      `*.content.ron` + `[Content]` (`Section(...)` / `Scenario(...)`) shape;
      remove every `*.scenario.ron` / `assets/scenarios/*.ron` reference. Ground
      each statement in a real file (asteroid_field.content.ron, demo mod).
- [ ] Decide the page's role: make it a true creator reference (bundle /
      content / catalog / naming / RON gotchas), OR move the contributor
      rationale to a note and drop the "reference" framing on `modding.md`.
      Prefer: keep it as the data-format reference but reference-shaped.
- [ ] Sweep the other creator pages for the same stale `.scenario.ron` framing.
- [ ] Verify: `npm run ci` green; the page's file names match the tree; links
      resolve.

## Notes

Docs-only. Do not invent format details - copy shapes from the shipped
`*.content.ron` files and `crates/nova_modding` / `nova_mod_format`.
