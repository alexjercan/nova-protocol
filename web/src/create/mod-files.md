# Mod files

A Nova Protocol mod is a folder that contains one bundle manifest, one or more
content files, and any art or audio that the mod owns. Content files can define
eight kinds of reusable item: campaigns, scenarios, ship sections, whole
ships, ship skin styles, training lessons, UI themes, and items.

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

## The installed catalog

`assets/mods.catalog.ron` lists every mod the build ships, in load order, and
is the only index the loader reads: there is no directory scan, so the web
build works from the same file. Each entry is a thin pointer; the mod's
metadata stays in its own bundle's `meta` block.

| field | type | default | meaning |
|---|---|---|---|
| `id` | string | required | The enable key and overlay namespace, and the `<id>` in another mod's `dep://<id>/` refs. |
| `bundle` | path | required | The mod's `*.bundle.ron`, relative to the assets root. |
| `base` | bool | `false` | The base game: enabled by default and locked on in the Mods menu. |

Every cataloged mod gets a row in the Mods menu: there is no way to install
content the player can neither see nor switch off. A tooling or test rig
belongs in the example or test that owns it, not in the catalog.

A fresh install enables `base` and nothing else. Your mod ships installed and
switched off, and the player turns it on from its own row.

Only `base` is mandatory. The game loads every other cataloged mod on its own,
beside the boot load: if one of them will not parse or its content will not
load, that mod is switched off, written off in the saved enabled set, and named
in one `MODS DISABLED` notice on the main menu. The files stay installed, and
the Mods screen is where the player updates, removes or re-enables it. So a mod
you ship broken costs that mod, not the player's game - but nothing tells them
more than the loader's own first line, which is what `content lint` is for.

The load also runs the section checks `content lint` runs, on every section a
mod carries, used or not. One failing section switches off the WHOLE mod, and
the notice names the section and the first fault. Every enabled mod that
depends on it is switched off with it, because its ships name sections that are
no longer there. A mod that overrides a base section with a broken one is
switched off, and the base section stays.

During a scenario, a mod that fails to load, fails these checks, or depends on
a mod that was switched off (after an update or a download, for example) is not
switched off: the game stops with an error that names the mod.

The full packaging, catalog, local installation, and publishing flow is in
[Publish a mod](../publish-a-mod/).

## Balance acknowledgments

`content lint` grades every combat scenario for fairness. A `close-spawn`
warning says a hostile arrives inside its own weapon envelope of the player
spawn. When that is the point - a boss entrance, a scripted ambush - declare it
in a `balance_acks.ron` beside the manifest. The linter reads the file from the
bundle it lints, so the justification travels with the mod:

```ron
[
    (
        scenario: "my_first_mission",
        hostile: "boss_1",
        kind: "close-spawn",
        reason: "The finale entrance. It telegraphs with an 8s engage_delay and a warning line.",
        task: "2026-08-16",
    ),
]
```

The file is optional and is not listed in `content`. An acknowledged finding
still prints, tagged `ACK` with its reason, but stops counting as an open
warning.

| field | meaning |
|---|---|
| `scenario` | The scenario id the finding sits in. |
| `hostile` | The scenario object id of the hostile the finding names. |
| `kind` | `close-spawn`. A `spawned-dead` finding is an ERROR and can never be acknowledged. |
| `reason` | Why this one is intended. Written for the next reader, not for the linter. |
| `task` | Whatever records the decision - a ticket id, a date. |

An ack that matches no live finding is STALE and fails the lint: once the
content is rebalanced, prune the entry.

## Content files

Every `*.content.ron` file is a RON list. One file may contain any mix of the
eight item kinds:

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
            health: 150.0,
        ),
        kind: Hull((
            render_mesh: Some("dep://base/gltf/hull-01.glb#Scene0"),
        )),
    )),
    Ship((
        id: "my_corvette",
        name: "My Corvette",
        hull: (sections: []),
    )),
    Style((
        id: "my_look",
        name: "My Look",
        palette: (
            top: (color: Srgba((red: 0.4, green: 0.4, blue: 0.42, alpha: 1.0)), roughness: 0.8, metallic: 0.2),
            wall: (color: Srgba((red: 0.1, green: 0.1, blue: 0.12, alpha: 1.0)), roughness: 0.9, metallic: 0.2),
        ),
        fixtures: [],
    )),
    UiTheme((
        id: "my-mod/amber",
        name: "Amber CRT",
        inherit: Some("base/phosphor"),
        palette: {
            "primary": "#ffb84a",
        },
        metrics: None,
        roles: (),
    )),
    Item((
        id: "my_mod/survey_core",
        name: "Survey core",
        about: "A sealed sensor core.",
        category: Parts,
        mass_g: 5000,
        ask_cr: 60,
        bid_cr: 45,
    )),
]
```

An `Item` adds a new entry to the item catalog, or replaces one your mod
depends on. Every field is required: `id`, `name` and `about` are not blank;
`category` is one of `Raw`, `Repair`, `Ammo`, `Food`, `Parts`; `mass_g` is
above 0; `bid_cr` is at most `ask_cr`. Any ship's stock, a cargo canister, or
a trade may carry the id, mod items included. The base game must define ten
role ids that the game itself uses: `HullPlate` feeds a repair, the
ammunition feeds a weapon's reload, the ores are mining yield, and generated
ship holds draw the two trade goods `Rations` and `SalvagedParts`. A role id
keeps its category: `HullPlate` is `Repair`; `PdcRound`, `RailSlug` and
`Torpedo` are `Ammo`; `StoneOre`, `IronOre`, `WaterIce` and `CarbonOre` are
`Raw`; `Rations` is `Food`; `SalvagedParts` is `Parts`. A mined ore fits one
cargo canister, so its `mass_g` is at most `200000`. A mod that depends on
the base game may replace a role id whole, with its own `name`, `about`,
`mass_g`, `ask_cr` and `bid_cr`, but not another `category`. A replacement
that breaks either rule is an error. An id two unrelated packs both define is refused in both, whatever
load order; a pack that depends on the id's owner may replace it whole. See
[Scenario objects](../objects/#inventory) for the item table and the stock
format.

Splitting these into `campaign.content.ron`, `scenarios.content.ron`, and
`sections.content.ron` is a readability convention, not a loader requirement.
Large mods can use one scenario per file and list all of them in `content`.

## The eight content chapters

<div id="wiki-children"></div>

- A [campaign](../campaigns/) orders scenarios for the Scenarios menu.
- A [scenario](../scenarios/) defines a playable mission, a menu backdrop or a
  handbook practice range - see [roles](../scenarios/#roles). Events,
  filters, actions, objects, and expressions belong to scenario scripting.
- A [section](../sections/) defines a reusable hull, thruster, controller,
  turret, or torpedo bay.
- A [ship](../ships/) defines a whole hull - its section layout and cladding -
  that any scenario can spawn by id.
- A [style](../styles/) defines the look a ship's derived cladding wears: the
  two plate surfaces, and the destructible decoration scattered over them.
- A [lesson](../lessons/) is one screen of the in-game training handbook: a
  demonstration, a few lines, the actions it names, and sometimes a focused
  range to fly.
- A [UI theme](../ui-themes/) is the look the whole interface paints in: the
  palette, the metrics, and the paint of every control in every state. The
  player picks one in Settings.
- An [item](../objects/#inventory) is one entry in the catalog a ship's stock,
  a cargo canister, or a trade can carry - what it is called, what it is for,
  what it weighs, and what a trader pays for it.

## Paths and dependencies

Content uses explicit asset schemes:

- `self://textures/skybox.png` reads a file from this mod's `resources` list.
- `dep://base/gltf/hull-01.glb#Scene0` uses a base-game resource.
- `dep://art-pack/models/station.glb#Scene0` uses a resource from a mod listed
  in `meta.dependencies`.

A plain path without `self://` or `dep://` is not a valid asset reference. See
the [base content catalog](../base-content/) for reusable base ids and assets.

## Overlay behavior

Content merges by id:

- A new id adds a campaign, scenario, section, ship, style, lesson, or theme.
- An id that already exists replaces that whole entry.
- A duplicate id inside one bundle is a conflict: the load logs it, keeps the
  first entry, and loads the rest of the bundle.

For sections, the key is `base.id`. For campaigns, scenarios, ships, styles,
lessons, and UI themes, the key is `id`. Prefix new ids with your mod id to
avoid accidental collisions.

An item id follows a stricter rule: a new id adds, but an id another pack
already defines is replaced only when your mod depends on that pack, directly
or through others (base counts as a dependency of every mod). Two packs that
share an id with no such one-way chain between them are BOTH refused, in
either load order. A pack that defines one item id twice is refused too. Any
item error refuses the WHOLE pack, not only the item: a mod is switched off
with every mod that depends on it, as for a failing section, and an error in
`base` stops the game.

`content lint` is stricter than the game here. The game checks only the mods
you enable together. Lint checks every mod in the repository at once, so two
repository mods that share an item id fail lint even when no one enables
both. `content lint --target <dir>` checks your mod against every repository
mod the same way. Lint cannot see mods from outside the repository, so such a
mod can still conflict with yours when the game loads them together.
