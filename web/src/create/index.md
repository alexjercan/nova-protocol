# Create

Nova Protocol supports data-driven mods. A mod can add campaigns, playable
scenarios, ship sections, whole ships and skin styles, menu backdrops, story
messages, objectives, enemies, and custom art or audio. The base campaign and
portal mods use the same content system available to mod authors.

You need a text editor and a local copy of the repository. Start with one small
scenario, play it, then publish it when it works.

## Learn to mod

Follow this path in order:

1. **[Create your first scenario](author-a-scenario/)** - edit the
   working Example Mod, follow one objective from setup to victory, and launch
   it from the Scenarios menu. This introduces events, filters, actions, and
   variables without requiring the full reference.
2. **[Publish a mod](publish-a-mod/)** - turn finished content into a
   release, run the lint and loader checks, preview the portal, and make it
   installable by other players.
3. **[Open the modding reference](reference/)** - learn the complete
   format when you are ready to build larger maps, campaigns, custom ship parts,
   and more complex mission logic.

## What can a mod contain?

- **Campaigns** group scenarios into an ordered story.
- **Scenarios** create playable maps and script them with events, filters,
  actions, objects, and variables.
- **Ship sections** add or replace hulls, thrusters, controllers, turrets, and
  torpedo bays.
- **Ships and styles** author whole hulls once, spawn them by id, and restyle
  the cladding they wear.
- **Impact rows** voice a hit by pairing a damage type with the material it
  struck.
- **Resources** provide your own images, models, sounds, thumbnails, and
  skyboxes.

The [Mod files](mod-files/) page shows how those pieces fit into one
folder and bundle.

## Look it up

The [modding reference](reference/) is the exhaustive catalog:

- [Mod files](mod-files/) - folder, bundle, content files, resources,
  dependencies, and overlays.
- [Campaign files](campaigns/) - campaign fields and chapter order.
- [Scenario files](scenarios/) - scenario metadata and handler shape.
- [Ship sections for mods](sections/) - every section kind and field.
- [Ships for mods](ships/) - whole hulls authored once and spawned by id.
- [Ship skin styles for mods](styles/) - the look a hull's cladding wears.
- [The impact table for mods](impacts/) - what a round sounds like against what
  it hit.
- [Base content catalog](base-content/) - reusable ids and
  `dep://base/` assets.

Scenario scripting then branches into [Events](events/),
[Filters](filters/), [Actions](actions/),
[Scenario objects](objects/), and
[Variables and expressions](expressions/).

## Extend the engine

The reference lists everything RON content can use. Adding a new event, filter,
action, object kind, or section kind requires a game-code change. Contributors
can start with the [developer book](../dev/).
