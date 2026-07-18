# Document modding meta-conventions: version semantics, dependency merge order, resource rules, and the publish-vs-load validation split

- STATUS: OPEN
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

- [ ] Version semantics in `guide-make-a-mod.md`: the loader accepts any
      non-empty string, but document the convention the shipped mods follow
      (semver-ish: content rework bumps minor, reskin/fix bumps patch), why the
      version matters (portal update detection, changelog association), and
      that tests may pin it (`tests/gauntlet_course.rs` asserts 1.2.0 to catch
      a silent unbump).
- [ ] Dependency semantics in `guide-make-a-mod.md` / `modding-ron.md`:
      document the merge order (manifest order, shipped mods before downloaded
      ones), and that adding or dropping a dependency is a breaking change for
      installed players (Gauntlet dropping its `demo` dep is the example).
- [ ] Resource rules in `guide-make-a-mod.md`: document that `.meta` sidecars
      are exempt from the `resources` list (they ride along automatically -
      `example.bundle.ron` does not list `nebula.png.meta`), so authors stop
      wondering whether the omission is a bug.
- [ ] The publish-vs-load validation split in `mod-portal.md`: the portal
      generator validates what a manifest gate can (parse, meta, files exist,
      id collisions, deps resolve) but does NOT deserialize content - a mod can
      publish clean and still fail in-game. Document the split and the
      recommended pre-publish check (`content lint --target <mod>` plus a local
      load).
- [ ] The bundle filename rule's rationale in `modding-ron.md`: the docs state
      "always `<id>.bundle.ron`, never a bare `bundle.ron`" but the load-bearing
      reason (Bevy's loader resolves by full extension; a bare `bundle.ron`
      resolves to the `ron` loader and the load silently fails - see the
      comment in `crates/nova_modding/src/lib.rs` around line 208 and task
      20260714-163342) lives only in code. Move the why into the doc.
- [ ] `cd web && npm run ci` green.

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
