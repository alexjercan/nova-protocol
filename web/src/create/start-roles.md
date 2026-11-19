# Start roles for mods

A `StartRole` is one New Game start: the icon its row shows, the catalog
hull the player flies, the kit it starts with, a sum of credits and a hold
of items, the job mix its station boards lean to, and its goal, one finite
milestone. Roles are content like ships and stations. Create a new id to
add a role, or reuse a base id to replace that role. The New Game wizard
lists the roles in merge order.

This page is the field-by-field start role reference. For general RON
spelling rules, see
[RON spelling rules](../reference/#how-ron-content-is-written).

## The StartRole item

```ron
[
    StartRole((
        id: "my_courier",
        name: "Courier",
        icon: Container,
        about: "A light freighter with no weapons. Carry cargo between stations.",
        hull: "light_freighter",
        kit: (
            credits: 2000,
            stock: {
                "Rations": 50,
                "WaterIce": 50,
            },
        ),
        jobs: Trade,
        goal: (
            title: "Courier runs",
            kind: CargoRuns(jobs: 3),
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
| `icon` | `Drill`, `Claw`, `Container`, `Star`, `Shield` or `Blades` | the shape drawn beside the name in the New Game role list and on the Load list. Lint refuses any other value |
| `about` | string | how the role plays, for a reader |
| `hull` | string | the id of a [ship design](../ships/) the player starts in: a base ship id, one of your mod's own, or one from a mod you depend on |
| `kit` | kit | what the ship starts with, see below |
| `jobs` | `Mining`, `Salvage`, `Trade`, `Bounty`, `Patrol` or `Raid` | the job mix every station board leans to for this start, see [Jobs](../stations/#jobs) |
| `goal` | goal | the role's goal, one finite milestone, see [The goal](#the-goal) |
| `lore` | string | background text for the role |

A `kit` has two fields, both required:

| field | type | meaning |
|---|---|---|
| `credits` | whole number | the credits the ship starts with |
| `stock` | stock | the items the hold starts with, in the [stock format](../objects/#inventory). It must fit the hull's hold (100 kg per hull section) |

## The goal

A `goal` is one finite milestone the role plays toward, read off the saved
world. Every role's progress opens with the goal's own line, the count its
`kind` names, and closes with the same shared line, `Stations owned`, which
needs one station the player owns, live or kept in a sector left behind.
The Map's chart view OBJECTIVES block lists the next unmet line, the goal's
own line first, until both are met, then reads `Goal reached at` the play time it happened.
It is not an ending: the world is an endless sandbox, a met goal stamps the
save and nothing else, and there is no completion screen. It has two
fields, both required:

| field | type | meaning |
|---|---|---|
| `title` | string | the name the New Game role step and the Load screen show |
| `kind` | goal kind | the goal's own line, before the shared station line, one of the kinds below |

| kind | fields | met when |
|---|---|---|
| `OreDeliveries` | `jobs` | the player handed in `jobs` Procure jobs whose item is in the `Raw` category: ore and ice |
| `Recoveries` | `jobs` | the player handed in `jobs` Recover jobs |
| `CargoRuns` | `jobs` | the player handed in `jobs` Procure, Supply or Escort jobs |
| `KillList` | `hulls` | the player handed in Neutralize jobs naming `hulls` distinct hulls |
| `Patrols` | `jobs` | the player handed in `jobs` Distress, Supply or Escort jobs |
| `Outlaw` | `hulls` | the player or a fleet escort neutralized `hulls` trading hulls, the hulls a kill is an [offence](../stations/#civilization-stances) on, of any civilization at any stance. A raider or a caller a job put in the world does not count |

Every count is a whole number above zero. Each base role asks for three:
`industrial` three ore deliveries, `scavenger` three recoveries, `transport`
three cargo runs, `bounty_hunter` a kill list of three hulls, `military` a
patrol record of three jobs and `pirate` three hulls taken (`Outlaw`).

Progress is derived from the saved world each frame: the jobs you handed
in, the trading hulls you neutralized, a count the save keeps, and the
stations you own, live or frozen in a sector you left. The frame both lines
are met, the save records the play time; the OBJECTIVES block then reads
`Goal reached at` that time, and a Load brings it back. The world does not
change on a met goal.

## Overlay and lint

Overlay is last-wins by id, as for every content kind: a mod that defines a
`StartRole` with a base id replaces it whole. Roles keep their load order, so
a replaced role keeps its place in the list and a new role comes after the
base roles.

`content lint` and the content check at load both report an error for a
role that:

- names a `hull` no visible ship design has (the base ships, your own, and
  those of the mods you depend on);
- names a `hull` with no cargo intake section: an open-world ship jettisons
  and takes cargo in through one;
- names a `hull` whose cargo intake reaches as far out as a docking port's
  face, where a berthed ship parks;
- seeds its kit `stock` with an unknown item;
- seeds a kit heavier than the hull's hold;
- gives its `goal` a count of zero.

## In the game

The second step of the New Game wizard lists the roles and selects the
first. The details pane shows the selected role's `name`, `about`, the
hull's design name, the kit, the goal's `title` as its milestone, and the
`lore`. In the game the Map's chart view OBJECTIVES block lists the goal's
next unmet line, the goal's own line first, until all are met. **Create** starts
the base `open_world` scenario, whose
[`SpawnStartRole`](../actions/#spawnstartrole) action spawns the role's hull
with its kit and binds its weapons, docked at the starting station: the
nearest station of a civilization that begins as the player's Ally.

The world save records the role id. A Load refuses a world whose role id
the loaded content does not define, with
`its start role '<id>' is not in the loaded content`. The content check runs
first, so a role that a mod change removed shows `content changed`.

## Base start roles

<!-- Kits: crates/nova_authoring/src/base_content/start_roles.rs
     `start_role_catalog`; prices: assets/base/items/base.content.ron. -->

A kit's worth at ask is its credits plus its items at their `ask_cr` per
`price_lot`. An unarmed role starts with 2,200 cr, an armed one with
1,800 cr and the scavenger, whose hull is the dearest, with 1,500 cr. A
PDC-armed hull also carries 3,000 `PdcRound` in its shared hold, not per
mount: 30 whole lots of 100 at 10 cr each, so 300 cr at ask. Every kit also
carries 3 `RepairKit`, 360 cr at ask, and 6 `SectionPanel`, 120 cr at ask.

| id | hull | kit |
|---|---|---|
| `industrial` | `mining_tug` | 2,200 cr, 15 `HullPlate`, 25 `Rations`, 6 `SectionPanel`, 3 `RepairKit` |
| `scavenger` | `salvage_hauler` | 1,500 cr, 3,000 `PdcRound`, 10 `HullPlate`, 6 `SectionPanel`, 3 `RepairKit` |
| `transport` | `light_freighter` | 2,200 cr, 50 `Rations`, 50 `WaterIce`, 10 `HullPlate`, 6 `SectionPanel`, 3 `RepairKit` |
| `bounty_hunter` | `interceptor` | 1,800 cr, 3,000 `PdcRound`, 10 `HullPlate`, 6 `SectionPanel`, 3 `RepairKit` |
| `military` | `patrol_picket` | 1,800 cr, 2 `Torpedo`, 3,000 `PdcRound`, 10 `HullPlate`, 6 `SectionPanel`, 3 `RepairKit` |
| `pirate` | `raider` | 1,800 cr, 3,000 `PdcRound`, 10 `HullPlate`, 6 `SectionPanel`, 3 `RepairKit` |

The roles are generated into `assets/base/roles/base.content.ron`.

The proof is `system_start_roles`: for each base role, New Game spawns the
role's hull with its kit and its weapons bound, the hull passes the post-build
checks, docks at a generated station and jettisons an item through its cargo
intake and takes it back in, and Load last save brings the role,
the hull and the kit back from disk.
