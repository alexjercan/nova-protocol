# Document modding meta-conventions: version semantics, dependency merge order, resource rules, and the publish-vs-load validation split

- STATUS: CLOSED
- PRIORITY: 44
- TAGS: v0.8.0,docs,web,modding

## Story

As a mod author preparing a bundle for the portal, I want the conventions
around versioning, dependencies, resources and validation written down, so that
I can publish updates without tripping over rules that today live only in code
comments, tests and shipped-mod history.

The pre-v0.7.0 documentation review (2026-07-18) found five meta-conventions
the modding docs state incompletely or not at all. None of them block a first
mod, but each one bites on the second release of a mod - exactly where Gauntlet
(1.0.0 -> 1.2.0) and The Ledger (1.0.0 -> 1.5.0) have already been.

## Steps

- [x] Version semantics in `guide-make-a-mod.md`: the loader accepts any
      non-empty string, but document the convention the shipped mods follow
      (semver-ish: content rework bumps minor, reskin/fix bumps patch), why the
      version matters (portal update detection, changelog association), and
      that tests may pin it (`tests/gauntlet_course.rs` asserts 1.2.0 to catch
      a silent unbump).
- [x] Dependency semantics in `guide-make-a-mod.md` / `modding-ron.md`:
      document the merge order (manifest order, shipped mods before downloaded
      ones), and that adding or dropping a dependency is a breaking change for
      installed players (Gauntlet dropping its `demo` dep is the example).
- [x] Resource rules in `guide-make-a-mod.md`: document that `.meta` sidecars
      are exempt from the `resources` list (they ride along automatically -
      `example.bundle.ron` does not list `nebula.png.meta`), so authors stop
      wondering whether the omission is a bug.
- [x] The publish-vs-load validation split in `mod-portal.md`: the portal
      generator validates what a manifest gate can (parse, meta, files exist,
      id collisions, deps resolve) but does NOT deserialize content - a mod can
      publish clean and still fail in-game. Document the split and the
      recommended pre-publish check (`content lint --target <mod>` plus a local
      load).
- [x] The bundle filename rule's rationale in `modding-ron.md`: the docs state
      "always `<id>.bundle.ron`, never a bare `bundle.ron`" but the load-bearing
      reason (Bevy's loader resolves by full extension; a bare `bundle.ron`
      resolves to the `ron` loader and the load silently fails - see the
      comment in `crates/nova_modding/src/lib.rs` around line 208 and task
      20260714-163342) lives only in code. Move the why into the doc.
- [x] `cd web && npm run ci` green.

## Definition of Done

- Each of the five conventions above is stated in the wiki page where an author
  would look for it, with the why, not just the rule.
- A hypothetical "publish my second version" walkthrough (bump version, adjust
  deps, add a resource, pre-publish check) can be followed using only the wiki.

## Notes

- Source findings: pre-v0.7.0 docs review (2026-07-18), mods audit sections C
  and D.
- Keep alignment with 20260718-152247 (portal generator Python port): if the
  generator moves, `mod-portal.md` wording changes with it - land whichever is
  second against the other's text.

## Close-out (2026-07-20)

What was added, page by page (all verified against the tree first):

- `web/src/wiki/dev/guide-make-a-mod.md`
  - "Versioning your mod" (new subsection under Bundle anatomy): opaque
    non-empty string, semver-ish convention (rework -> minor, reskin/fix ->
    patch), Gauntlet 1.0.0->1.1.0->1.2.0 and Ledger 1.0.0->1.5.0, why version
    matters (`<id>/<version>/` republish distinction + changelog anchor), and
    the test pin `bundle_ships_the_bumped_version` (asserts `version: "1.2.0"`).
  - "Dependencies and merge order" (new subsection under Overlay semantics):
    the exact `register_bundles` order (catalog order, base first; then
    downloaded; then topological sort with catalog-then-download tiebreak), and
    that adding/dropping a dep is breaking for installed players (the Gauntlet
    `demo`-drop story, health 200<->400).
  - `.meta` sidecar exemption: a new bullet in the resources list stating the
    `.png.meta` rides along automatically and is NOT listed, with the example
    mod as proof, so the omission is not mistaken for a bug.
- `web/src/wiki/dev/mod-portal.md`
  - "The publish-vs-load split" (new subsection): the generator is a manifest
    gate and never deserializes content, so publish-clean != runs-in-game; the
    two-part pre-publish check (`content lint --target <mod>` with `--report`,
    plus a local in-game load / `webmods_validation`).
- `web/src/wiki/dev/modding-ron.md`: NO edit needed. Convention 5 (the
  stemmed-extension why) was ALREADY fully documented here in the "File naming
  ... load-bearing" section (full-extension resolution, bare `bundle.ron` ->
  `ron` loader -> "Could not find an asset loader"), and again in
  guide-make-a-mod.md's "The stemmed-extension rule". The `lib.rs:~208` comment
  and task 20260714-163342 confirm the why is accurate; the doc already carries
  it. Left as-is to avoid redundant duplication.

Where the code/history disagreed with the task's premise:

- The task and the gauntlet bundle COMMENT both say "2.0 drops that coupling",
  implying a 2.0.0 bump. The actual shipped `version` is `1.2.0` (confirmed in
  webmods/gauntlet + the test pin). The `demo` dep was dropped at the
  1.0.0 -> 1.1.0 step, not at a 2.0. I wrote the doc to the REAL versions
  (1.2.0, minor bump) and did not repeat the informal "2.0" shorthand, so the
  wiki matches the tree, not the comment's loose phrasing.
- The task framed convention 5 as "the why lives only in code" needing a move.
  In fact the why was already in TWO wiki sections. Corrected the premise here;
  no doc change made for it.

`cd web && npm run ci`: GREEN (prettier format:check clean, eslint clean,
webpack build compiled successfully). Wiki `.md` files are not prettier-checked
(that targets ts/html/js) but they are built by webpack, so a broken page would
fail the build; it did not.

Self-reflection: the two docs were already dense and high-quality, so the real
work was gap-analysis, not writing - most of the value was confirming which
conventions were genuinely missing (1, 2, 4 + the .meta framing) vs already
present (5, and .meta partially). Verifying every "why" against code caught the
1.2.0-vs-"2.0" mismatch, which a paraphrase-from-task approach would have
shipped. Next time: run `npm ci` up front, since `npm run ci` fails opaquely
("prettier: command not found") when deps are absent.
