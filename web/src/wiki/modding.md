# Modding

Nova Protocol is data-driven: scenarios, ship sections and whole campaigns are
authored as **RON data files**, not code. Everything the game ships - the
mainline campaign, the menu backdrops, the portal mods - is content loaded
through the same pipeline your own content uses, so anything the base game
does, a mod can do too. A text editor is everything you need.

**New here? Start with [Author a scenario](../dev/guide-author-scenario/).**
The guides below are a ladder - each one assumes the one before it - and every
construct they use is cataloged in the [modding reference](../modding/reference/)
when you need the full field list.

## Learn to mod (the guides)

Work through these in order; each ends with something you can run.

1. **[Author a scenario](../dev/guide-author-scenario/)** - write a mission in
   RON out of events, filters and actions: the file shape, a worked objective
   loop, and how to load and test it. After this you can build and play your
   own scenario.
2. **[Author a section](../dev/guide-author-section/)** - write a ship part
   (hull, thruster, controller, turret, torpedo bay) and overlay or extend the
   base catalog. After this you can put your own hardware on a ship.
3. **[Make and publish a mod](../dev/guide-make-a-mod/)** - package your
   content as a bundle, test it locally, and publish it to the mod portal.
   After this other players can install what you made.

## Look it up (the reference)

The **[modding reference](../modding/reference/)** is the catalog: every
event, filter, action, object kind and expression node, one anchor per
construct, with field tables (types, defaults, units) and copyable RON.

- [The scenario file](../modding/scenario/) - `Scenario` and `Campaign` items
  and the handler shape.
- [Events](../modding/events/) - everything that can fire a handler, and what
  each event carries.
- [Filters](../modding/filters/) - the three filter kinds and the fail-closed
  rules.
- [Actions](../modding/actions/) - everything a handler can do, grouped by
  purpose.
- [Scenario objects](../modding/objects/) - everything a scenario can spawn.
- [Variables & expressions](../modding/expressions/) - the expression grammar,
  node by node.
- [Base content catalog](../modding/base-content/) - the section prototype
  ids, scenario ids and `dep://base/` assets you can reference.
- [Modding data format (RON)](../dev/modding-ron/) - RON syntax gotchas,
  bundles, the catalog and the local cache.
- [Mod portal](../dev/mod-portal/) - how published mods are served and
  installed.

## Extend the engine

New event kinds, filters, actions or scenario-object types need a small Rust
change - that is a contributor task, covered in
[Extend the scenario engine](../dev/guide-extend-scenarios/).
