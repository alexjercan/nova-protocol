# Campaign files

A campaign groups scenarios under one heading in the Scenarios menu and defines
their replay order. It does not contain mission logic. Each member is the id of
a separate [`Scenario`](../scenarios/) item.

## File shape

A campaign can have its own content file:

```ron
[
    Campaign((
        id: "freight_wars",
        name: "Freight Wars",
        scenarios: [
            "freight_wars_arrival",
            "freight_wars_ambush",
            "freight_wars_escape",
        ],
    )),
]
```

List that file in the mod's bundle manifest:

```ron
content: [
    "campaign.content.ron",
    "arrival.content.ron",
    "ambush.content.ron",
    "escape.content.ron",
],
```

See [Mod files](../mod-files/) for the complete folder and bundle shape.

## Fields

| field | type | default | meaning |
|---|---|---|---|
| `id` | string | required | Stable campaign key. Prefix it with your mod id to avoid collisions. |
| `name` | string | required | Campaign heading shown in the Scenarios menu. |
| `scenarios` | list of scenario ids | required | Members in display and replay order. Every id must resolve after all enabled mods merge. |

A missing member id is a lint error. A duplicate member is a warning because it
would show the same chapter more than once.

## How campaign order works

The list controls grouping and replay order. It does not automatically move the
player between missions. Add a
[`NextScenario`](../actions/#nextscenario) action to a scenario when campaign
play should continue directly into the next member.

A scenario may set `hidden: true` and still appear under its campaign heading.
This is useful for continuation chapters that should not appear in the flat
scenario list.

```ron
NextScenario((
    scenario_id: "freight_wars_ambush",
    linger: true,
)),
```

## Adding or replacing a campaign

- A new campaign id adds a campaign alongside existing campaigns.
- Reusing an existing campaign id replaces its full name and member list.
- Campaign replacement does not replace the member scenarios. Overlay those
  scenario ids separately if required.

The base campaign and scenario ids are listed in the
[base content catalog](../base-content/#scenario-ids).

## Check it

From the repository root:

```sh
nix develop --command cargo run content lint --target path/to/your-mod
```

Then enable the mod and open the Scenarios menu. Confirm the campaign heading,
chapter names, order, and direct transitions.
