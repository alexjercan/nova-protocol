# Spike: multi-file scenario bundles - folder + manifest, id namespacing/overlay, directory loader

- STATUS: OPEN
- PRIORITY: 35
- TAGS: v0.6.0,modding,scenario,spike

Spike: tasks/20260714-110502/SPIKE.md

Goal (step 4, its OWN spike): a scenario as a folder + manifest ("bundle of files")
- separate files for sections, ships, objectives, events - loaded via a directory
`AssetLoader` (or a manifest that `load_context.load`s the parts). The real
uncertainty is id namespacing/overlay (scenario-local vs catalog, collisions, load
order), so this is a spike, not a plan yet. Gated on the prototype/catalog model
(steps 1-3) being proven first - do not attempt before them.

## Note (user, 20260714): files are TYPED, merged by kind (Wesnoth model)

The bundle is not "one scenario split into parts" - it is a pile of typed content
files that each DECLARE their kind (a ship, a section, a scenario, a map/level, an
objective set, ...). Loading a bundle = read ALL the files, and merge each into the
appropriate collection by its declared type, exactly like Wesnoth defines units,
scenarios, maps, etc. and folds them into the right registries. So the spike must
design:

- A per-file kind tag / discriminant (e.g. a top-level `Section(( .. ))` /
  `Ship(( .. ))` / `Scenario(( .. ))` wrapper, or a manifest that maps each file to
  a kind) - so a loose `*.ron` self-identifies without relying on its folder.
- A router that dispatches each loaded file to its collection: sections ->
  `GameSections`, ships -> a ship-prototype registry, scenarios -> `GameScenarios`,
  etc. Adding a new content kind = one new arm.
- How this composes with the catalog (steps 1-3): the base game IS such a bundle
  (its sections/ships/scenarios), and a mod is another bundle merged on top by kind,
  with the same id namespacing/overlay rules. This makes "a mod" and "the base game"
  the same shape - the real payoff.

So the tagging/merge-by-kind design is the heart of this spike, not just the folder
layout.
