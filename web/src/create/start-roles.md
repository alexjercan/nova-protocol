# Start roles for mods

A `StartRole` is one New Game start: the catalog hull the player flies, the
kit it starts with, a sum of credits and a hold of items, the job mix its
station boards lean to, and its goal, one finite milestone. Roles are
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
        jobs: Trade,
        goal: (
            title: "A courier network",
            kind: StationNetwork(jobs: 8, stations: 4, rings: 2),
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
| `jobs` | `Trade`, `Salvage` or `Combat` | the job mix every station board leans to for this start, see [Jobs](../stations/#jobs) |
| `goal` | goal | the role's goal, one finite milestone, see [The goal](#the-goal) |
| `lore` | string | background text for the role |

A `kit` has two fields, both required:

| field | type | meaning |
|---|---|---|
| `credits` | whole number | the credits the ship starts with |
| `stock` | stock | the items the hold starts with, in the [stock format](../objects/#inventory). It must fit the hull's hold (100 kg per hull section) |

## The goal

A `goal` is one finite milestone the role plays toward, read off the saved
world. The Map's chart view OBJECTIVES block lists its next unmet line until
it is met, then reads `Goal reached at` the play time it
happened. It is not an ending: the world is an endless sandbox, a met goal
stamps the save and nothing else, and there is no completion screen. It has
two fields, both required:

| field | type | meaning |
|---|---|---|
| `title` | string | the name the New Game role step and the Load screen show |
| `kind` | goal kind | what the save must show, one of the kinds below |

| kind | fields | met when |
|---|---|---|
| `StationNetwork` | `jobs`, `stations`, `rings` | the player handed in `jobs` bring or escort jobs, posted by `stations` distinct stations that stand in `rings` distinct civilization rings |
| `KillList` | `hulls`, `rings` | the player handed in `hulls` neutralize jobs, posted from `rings` distinct civilization rings |
| `Flagship` | `hull` | the player's live hull is worth at least the ship design `hull` names, at the build recipes' value |
| `OwnYard` | none | the player owns a station whose store holds, for every one of the seven ship part families (hull, controller, thruster, weapon, cargo intake, docking, mining), the full build recipe of at least one loaded section of that family |
| `MiningEmpire` | `stations`, `escorts` | the player owns `stations` stations and flies with `escorts` hired escorts in the fleet |
| `Outlaw` | `hulls` | the player's own kills turned a civilization `Enemy` (a seeded `Enemy` does not count) and the player neutralized `hulls` more intact hulls of it while it stands `Enemy`; the kill that flips it does not count, and a settlement or the decay clears the count, see [Civilization stances](../stations/#civilization-stances) |

A civilization ring is one 240 km step out from the start: a station in a
cell whose centre is under 240 km from the origin stands in ring 0, one
between 240 and 480 km in ring 1, and so on. Every count is a whole number
above zero. The base `transport` role asks for a station network of eight
jobs at four stations over two rings; the base `bounty_hunter` asks for a
kill list of six hulls over two rings; the base `scavenger` asks for a yard
(`OwnYard`); the base `military` asks for a hull worth the `line_warship`
(`Flagship`); the base `industrial` asks for a mining empire of two stations
and two escorts (`MiningEmpire`), the fleet's two slots; the base `pirate`
asks for an outlaw's name (`Outlaw`): one civilization its own kills turned
Enemy, then six more intact hulls of it neutralized while it stands Enemy.

Progress is derived from the saved world each frame: the jobs you handed in,
the hull you fly, the stations you own with what you stored there, live or
frozen in a sector you left, the escorts in your fleet, and the
[civilizations](../stations/#civilization-stances) your kills turned Enemy
with the hulls you took from them since. A yard counts
its best single station: two stations each stocked for some families do not
add up. The frame the goal is met, the save records the play time; the
OBJECTIVES block then reads `Goal reached at` that time, and a Load brings
it back. The world does not change on a met goal.

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
- gives its `goal` a count of zero, asks for more `stations` than `jobs` (a
  hand-in serves one station) or more `rings` than `stations` or `hulls` (a
  station stands in one ring), asks for more `escorts` than the fleet's two
  slots, or names a `Flagship` `hull` no visible ship design has.

## In the game

The second step of the New Game wizard lists the roles and selects the
first. The details pane shows the selected role's `name`, `about`, the
hull's design name, the kit, the goal's `title` as its milestone, and the
`lore`. In the game the Map's chart view OBJECTIVES block lists the goal's
next unmet line until it is met. **Create** starts the base
`open_world` scenario, whose
[`SpawnStartRole`](../actions/#spawnstartrole) action spawns the role's hull
with its kit and binds its weapons, docked at the starting station: the
nearest station of a civilization that begins as the player's Ally.

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
checks, docks at a generated station and jettisons an item through its cargo
intake and takes it back in, and Load last save brings the role,
the hull and the kit back from disk.
