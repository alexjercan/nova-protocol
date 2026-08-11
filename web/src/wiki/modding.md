# Modding

Nova Protocol supports data-driven mods. A mod can add campaigns, playable
scenarios, ship sections, menu backdrops, story messages, objectives, enemies,
and custom art or audio. The base campaign and portal mods use the same content
system available to mod authors.

You need a text editor and a local copy of the repository. Start with one small
scenario, play it, then publish it when it works.

## Learn to mod

Follow this path in order:

1. **[Create your first scenario](../modding/author-a-scenario/)** - edit the
   working Example Mod, follow one objective from setup to victory, and launch
   it from the Scenarios menu. This introduces events, filters, actions, and
   variables without requiring the full reference.
2. **[Publish a mod](../modding/publish-a-mod/)** - turn finished content into a
   release, run the lint and loader checks, preview the portal, and make it
   installable by other players.
3. **[Open the modding reference](../modding/reference/)** - learn the complete
   format when you are ready to build larger maps, campaigns, custom ship parts,
   and more complex mission logic.

## What can a mod contain?

- **Campaigns** group scenarios into an ordered story.
- **Scenarios** create playable maps and script them with events, filters,
  actions, objects, and variables.
- **Ship sections** add or replace hulls, thrusters, controllers, turrets, and
  torpedo bays.
- **Resources** provide your own images, models, sounds, thumbnails, and
  skyboxes.

The [Mod files](../modding/mod-files/) page shows how those pieces fit into one
folder and bundle.

## Look it up

The [modding reference](../modding/reference/) is the exhaustive catalog:

- [Mod files](../modding/mod-files/) - folder, bundle, content files, resources,
  dependencies, and overlays.
- [Campaign files](../modding/campaigns/) - campaign fields and chapter order.
- [Scenario files](../modding/scenarios/) - scenario metadata and handler shape.
- [Ship sections for mods](../modding/sections/) - every section kind and field.
- [Base content catalog](../modding/base-content/) - reusable ids and
  `dep://base/` assets.

Scenario scripting then branches into [Events](../modding/events/),
[Filters](../modding/filters/), [Actions](../modding/actions/),
[Scenario objects](../modding/objects/), and
[Variables and expressions](../modding/expressions/).

## Extend the engine

The reference lists everything RON content can use. Adding a new event, filter,
action, object kind, or section kind requires a game-code change. Contributors
can start with [Extend the scenario engine](../dev/guide-extend-scenarios/) or
[Add a ship section](../dev/guide-add-section/).
