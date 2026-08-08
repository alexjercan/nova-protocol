# Spike: multi-file scenario bundles - folder + manifest, id namespacing/overlay, directory loader

- STATUS: CLOSED
- PRIORITY: 45
- TAGS: v0.6.0, modding, scenario, spike

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

## Absorbs whole-ship prototypes (folded from 20260714-113414, 20260714)

The standalone ship-catalog task (113414) was CLOSED and folded here: since no
built-in ship is reused, a standalone ship catalog would relocate rather than dedup,
and "ships as data" is naturally a content KIND in this bundle model. So this spike
also owns the ship-prototype mechanism:

- `ShipSource = Inline(SpaceshipConfig) | Prototype(ShipId)` on the scenario's
  `ScenarioObjectKind::Spaceship`, resolved at spawn against a `GameShips` catalog -
  exactly mirroring the section model (113411's `SectionSource` + `GameSections`), one
  level up. Ships become a "ship" kind in a bundle, loaded into `GameShips`.
- Ship-level modifications reuse 113411's component-modification model
  (`SectionModification` -> a `ShipModification` analogue applied by observers on the
  ship root, inert where not applicable): controller/speed-cap/infinite-ammo-style
  deltas. Design the starter set in the spike.
- The base game's ships (player_ship, pirate_ship, ...) become ship-kind files in the
  base bundle; a scenario references them by id. This only pays off once ships are
  reused (a fleet) or the bundle relocation is done - so the port is part of the
  bundle work, not a standalone step.

Net: the section catalog (113408) proved the pattern at the section level; this spike
generalizes prototype+catalog+modifications to ALL content kinds (sections, ships,
scenarios) inside the typed-bundle loader, instead of one bespoke catalog per kind.
