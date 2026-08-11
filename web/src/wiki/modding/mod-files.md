# Mod files

A Nova Protocol mod is a folder that contains one bundle manifest, one or more
content files, and any art or audio that the mod owns. Content files can define
three kinds of reusable item: campaigns, scenarios, and ship sections.

Use this page to choose the right file. Then open the detailed reference for the
item you want to author.

## Folder structure

A small mod can use this layout:

```text
my-mod/
|- my-mod.bundle.ron
|- campaign.content.ron
|- scenarios.content.ron
|- sections.content.ron
|- icon.png
|- screenshots/
|  `- mission.png
`- textures/
   `- skybox.png
```

Only the bundle manifest has a fixed role. The content filenames and folder
layout are your choice, but every content file must end in `.content.ron` and be
listed by the manifest.

The manifest filename must use the mod id as its stem:
`my-mod.bundle.ron`. Do not name it only `bundle.ron`.

## The bundle manifest

The bundle tells the loader which files belong to the mod:

```ron
(
    content: [
        "campaign.content.ron",
        "scenarios.content.ron",
        "sections.content.ron",
    ],
    resources: [
        "icon.png",
        "screenshots/mission.png",
        "textures/skybox.png",
    ],
    meta: (
        name: "My Mod",
        description: "A short campaign with custom ship parts.",
        author: "Your Name",
        version: "0.1.0",
        dependencies: [],
        icon: Some("icon.png"),
        screenshots: ["screenshots/mission.png"],
    ),
)
```

| field | type | default | meaning |
|---|---|---|---|
| `content` | list of paths | required | Content files loaded from this folder. Paths are relative to the manifest. |
| `resources` | list of paths | `[]` | Images, models, sounds, and other binary files owned by the mod. |
| `meta.name` | string | empty | Player-facing mod name. Required for portal publishing. |
| `meta.description` | string | empty | Player-facing summary. |
| `meta.author` | string | empty | Author or team name. |
| `meta.version` | string | empty | Release identifier. Required for portal publishing; change it for each release. |
| `meta.dependencies` | list of mod ids | `[]` | Other mods whose content and `dep://<id>/` resources this mod uses. Base is always available. |
| `meta.icon` | `Option` path | `None` | Mod icon relative to this folder. Use `Some("icon.png")`. |
| `meta.screenshots` | list of paths | `[]` | Mod screenshots relative to this folder. |
| `new_game_scenario` | `Option` scenario id | `None` | Base-game-only setting. The game warns and ignores it in ordinary mods. |

`resources` lists files, not folders. A `.meta` sidecar next to a listed image
travels with that image automatically and is not listed separately.

The full packaging, catalog, local installation, and publishing flow is in
[Publish a mod](../publish-a-mod/).

## Content files

Every `*.content.ron` file is a RON list. One file may contain any mix of the
three item kinds:

```ron
[
    Campaign((
        id: "my_campaign",
        name: "My Campaign",
        scenarios: ["my_first_mission"],
    )),
    Scenario((
        id: "my_first_mission",
        name: "First Mission",
        description: "Clear the shipping lane.",
        cubemap: "self://textures/skybox.png",
        events: [],
    )),
    Section((
        base: (
            id: "my_mod_hull",
            name: "My Hull",
            description: "A custom armor block.",
            mass: 1.0,
            health: 150.0,
        ),
        kind: Hull((
            render_mesh: Some("dep://base/gltf/hull-01.glb#Scene0"),
        )),
    )),
]
```

Splitting these into `campaign.content.ron`, `scenarios.content.ron`, and
`sections.content.ron` is a readability convention, not a loader requirement.
Large mods can use one scenario per file and list all of them in `content`.

## The three content chapters

<div id="wiki-children"></div>

- A [campaign](../campaigns/) orders scenarios for the Scenarios menu.
- A [scenario](../scenarios/) defines a playable mission or backdrop. Events,
  filters, actions, objects, and expressions belong to scenario scripting.
- A [section](../sections/) defines a reusable hull, thruster, controller,
  turret, or torpedo bay.

## Paths and dependencies

Content uses explicit asset schemes:

- `self://textures/skybox.png` reads a file from this mod's `resources` list.
- `dep://base/gltf/hull-01.glb#Scene0` uses a base-game resource.
- `dep://art-pack/models/station.glb#Scene0` uses a resource from a mod listed
  in `meta.dependencies`.

A plain path without `self://` or `dep://` is not a valid asset reference. See
the [base content catalog](../base-content/) for reusable base ids and assets.

## Overlay behavior

Content merges by item id:

- A new id adds a campaign, scenario, or section.
- An id that already exists replaces that whole item.
- A duplicate id inside one bundle is a conflict; the first item is kept.

For sections, the key is `base.id`. For campaigns and scenarios, the key is
`id`. Prefix new ids with your mod id to avoid accidental collisions.
