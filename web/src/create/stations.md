# Stations for mods

A `Station` is a hull that never flies, authored once and parked by id. It
owns a [ship design](../ships/) for its hull, the services a docked ship may
use, a storage cap, the stock its hold opens with and the credits it trades
with. A [Station](../objects/#station) spawn names it and adds what differs
per spawn: where it sits, which side it is on, whether it is a derelict.

Stations are content like ships and sections. Create a new id to add a
station, or reuse a base id to rebuild the base waystation everywhere it is
parked.

This page is the field-by-field station reference. For general RON spelling
rules such as double parentheses, `Some(...)`, and asset schemes, see
[RON spelling rules](../reference/#how-ron-content-is-written).

## Why a station is its own kind

A station is not a ship with the thrusters left off. It has no pilot, it is
never ordered anywhere, and it keeps a store beside its own hold. Giving it
its own kind lets the lint refuse a station that could fly, lets a save
freeze it as a station, and keeps a ship's spawn fields (a controller, a
capability set) off it.

## The Station item

```ron
[
    Station((
        id: "my_outpost",
        name: "My Outpost",
        about: "A collar, a bridge and four plates. Dock to trade.",
        hull: (
            sections: [
                (
                    id: "collar",
                    position: (-1.0, 0.0, 0.0),
                    rotation: (0.0, 0.70710677, 0.0, 0.70710677),
                    source: Prototype(id: "docking_port_section"),
                ),
                (
                    id: "bridge",
                    position: (0.0, 1.0, 0.0),
                    rotation: (0.0, 0.0, 0.0, 1.0),
                    source: Prototype(id: "basic_controller_section"),
                ),
                // ... hull plates ...
            ],
        ),
        services: [Trade, Storage],
        storage_cap_g: 50000,
        stock: {"HullPlate": 4},
        credits: 1000,
    )),
]
```

Every field is required.

| field | type | meaning |
|---|---|---|
| `id` | string | what a spawn names this design by. A mod reusing a base id REPLACES that station everywhere |
| `name` | string | the name a picker shows. A parked station is named by the scenario object that placed it |
| `about` | string | what this station is, for a reader |
| `hull` | design | the same fields a [ship design](../ships/#the-design) takes: `sections`, `integrity`, `presentation`. It must hold a `Docking` section and a `Controller` section and no `Thruster` section |
| `services` | list | what a docked ship may do here, from the closed set `Rebuild`, `Refit`, `Trade`, `Storage`, `Jobs`, `Roster`. At least one |
| `storage_cap_g` | whole number | how many grams the station's storage takes. Above zero exactly when `services` lists `Storage` |
| `stock` | stock | what the hull's own hold opens with, in the [stock format](../objects/#inventory). It must fit the hull's hold (100 kg per hull section) |
| `credits` | whole number | the credits the station trades with at spawn |

The hull's `Controller` section is what holds the station's attitude: a
nudged station turns back to the pose it was parked in, with no thruster to
move it. The `Docking` section is what a ship docks to.

Only the hull, the stock and the credits do anything in play today: a docked
ship trades with a station as it trades with a ship. The service list and
the storage cap are checked and carried, and the docked-ship service screens
that read them are their own task, as are a station's invulnerability and the
tuning of how firmly it levels itself.

## Spawning one

A scenario parks a station with a [Station](../objects/#station) object,
naming the design by id:

```ron
SpawnScenarioObject((
    base: (id: "outpost", name: "OUTPOST", position: (0.0, 0.0, -1500.0), rotation: (0.0, 0.0, 0.0, 1.0)),
    kind: Station((
        design: "my_outpost",
        allegiance: Some(Neutral),
        derelict: false,
        initial_velocity: (0.0, 0.0, 0.0),
    )),
)),
```

A station spawn takes no section patches and no inline design: a station is
the design, and a one-off station is one more `Station` item.

## Overlay and lint

Overlay is last-wins by id, as for every content kind: a mod that defines a
`Station` with a base id replaces it whole.

The static lint and the load gate both refuse a station that:

- names a section prototype no loaded content declares, or gives two
  sections one id, or a reserved id;
- has no `Docking` section (nothing could dock with it);
- has any `Thruster` section (a station never flies);
- has no `Controller` section (nothing would hold its attitude);
- lists no service;
- lists `Storage` with `storage_cap_g: 0`, or sets `storage_cap_g` above zero
  without `Storage`;
- seeds its `stock` with an unknown item, or more than the hull's hold takes.

A service listed twice is a warning. A spawn naming a station id no loaded
content declares is an error, like an unknown ship design.

## Base stations

| id | what it is |
|---|---|
| `block_waystation` | Waystation - a plated two-deck block, a bridge set into the top deck, a docking collar off the port flank (`port_collar`). Offers Rebuild, Refit, Trade and Storage; its hold opens with hull plates, rations and salvaged parts |
