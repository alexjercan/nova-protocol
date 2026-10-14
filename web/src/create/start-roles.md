# Start roles for mods

A `StartRole` is one New Game start: the catalog hull the player flies and
the kit it starts with, a sum of credits and a hold of items. Roles are
content like ships and stations. Create a new id to add a role, or reuse a
base id to replace that role. The New Game wizard lists
the roles in merge order.

This page is the field-by-field start role reference. For general RON
spelling rules, see
[RON spelling rules](../reference/#how-ron-content-is-written).

## The StartRole item

```ron
[
    StartRole((
        id: "my_courier",
        name: "Courier",
        about: "A light freighter with no weapons. Carry cargo between stations.",
        hull: "light_freighter",
        kit: (
            credits: 2000,
            stock: {
                "Rations": 50,
                "WaterIce": 50,
            },
        ),
        lore: "Stations need what they cannot make. Couriers carry it.",
    )),
]
```

Every field is required.

| field | type | meaning |
|---|---|---|
| `id` | string | the id the role is picked by. A mod reusing a base id REPLACES that role |
| `name` | string | the name a picker shows |
| `about` | string | how the role plays, for a reader |
| `hull` | string | the id of a [ship design](../ships/) the player starts in: a base ship id, one of your mod's own, or one from a mod you depend on |
| `kit` | kit | what the ship starts with, see below |
| `lore` | string | background text for the role |

A `kit` has two fields, both required:

| field | type | meaning |
|---|---|---|
| `credits` | whole number | the credits the ship starts with |
| `stock` | stock | the items the hold starts with, in the [stock format](../objects/#inventory). It must fit the hull's hold (100 kg per hull section) |

## Overlay and lint

Overlay is last-wins by id, as for every content kind: a mod that defines a
`StartRole` with a base id replaces it whole. Roles keep their load order, so
a replaced role keeps its place in the list and a new role comes after the
base roles.

`content lint` and the content check at load both report an error for a
role that:

- names a `hull` no visible ship design has (the base ships, your own, and
  those of the mods you depend on);
- seeds its kit `stock` with an unknown item;
- seeds a kit heavier than the hull's hold.

## In the game

The second step of the New Game wizard lists the roles and selects the
first. The details pane shows the selected role's `name`, `about`, the
hull's design name, the kit and the `lore`. **Create** starts the base
`open_world` scenario, whose
[`SpawnStartRole`](../actions/#spawnstartrole) action spawns the role's hull
with its kit and binds its weapons.

The world save records the role id. A Load refuses a world whose role id
the loaded content does not define, with
`its start role '<id>' is not in the loaded content`. The content check runs
first, so a role that a mod change removed shows `content changed`.

## Base start roles

Each base kit is worth 3,000 cr at ask: the credits plus the items at their
`ask_cr`.

| id | hull | kit |
|---|---|---|
| `industrial` | `mining_tug` | 2,200 cr, 15 `HullPlate`, 25 `Rations` |
| `scavenger` | `salvage_hauler` | 1,800 cr, 200 `PdcRound`, 10 `HullPlate` |
| `transport` | `light_freighter` | 2,000 cr, 50 `Rations`, 50 `WaterIce` |
| `bounty_hunter` | `interceptor` | 1,000 cr, 2 `Torpedo`, 200 `PdcRound`, 10 `HullPlate` |
| `military` | `patrol_picket` | 1,600 cr, 250 `PdcRound`, 10 `HullPlate` |
| `pirate` | `raider` | 1,200 cr, 15 `RailSlug`, 200 `PdcRound`, 10 `HullPlate` |

The roles are generated into `assets/base/roles/base.content.ron`.

The proof is `system_start_roles`: for each base role, New Game spawns the
role's hull with its kit and its weapons bound, the hull passes the post-build
checks and docks at a generated station, and Load last save brings the role,
the hull and the kit back from disk.
