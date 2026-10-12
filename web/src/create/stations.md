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

The `Docking` section is what a ship docks to. The `Controller` section is
what levels the station, see below.

Only the hull, the stock and the credits do anything in play today: a docked
ship trades with a station as it trades with a ship. The service list and
the storage cap are checked and carried, and the docked-ship service screens
that read them are their own task.

## How a station behaves

A station is a dynamic body, never a fixed one. Gravity pulls it, a ram
shoves it, and nothing holds it where it was placed: where a hit leaves it is
where it stays, and a world save keeps that pose and motion.

| behaviour | rule |
|---|---|
| self-levelling | the `Controller` section parks its target attitude on the spawn rotation and drives the hull with torque alone. Spun by a hit, the hull comes to rest within `STATION_LEVEL_TOLERANCE_RAD` (0.03 rad, about 1.7 degrees) of that attitude. No thrust is ever applied: only a `Thruster` section writes thrust, and the lint refuses one |
| invulnerable sections | a hit anywhere under a station root spends no health, so a station is never disabled, neutralized or destroyed. The rule is the station body's, living or dead alike: a derelict station is lootable, not breakable. A living station that can be fought into a derelict is a later rule |
| orbit | a station spawned in a planet's well rides a circular orbit: its `initial_velocity` is `sqrt(mass / r)` on the tangent of its radial with +Y (with +X when the radial is near +Y), in engine units, the speed the gravity pull integrates. The generator sets it; a scenario that parks a station in a well authors the same number or watches it fall |
| orbit tolerance | over three orbits, with a hit, the radius stays within `STATION_ORBIT_TOLERANCE` (15 percent) of where it started. The pull is held constant over each physics step, so every orbit widens a little (about 1.5 percent of its energy per orbit on the proof's fast 30 s orbit, less on a real planetoid's), and a kick of one percent of orbital speed swings the radius about four percent; past the tolerance the orbit is walking out of the well or down onto the planet |

The proofs are `system_station_body` (a nudged station in a void) and
`system_station_orbit` (a torpedo, a shove and three orbits of a small
planet).

## Docking at a station

A ship docks with a station through its `Docking` section as it docks with a
ship: a free port on each hull, inside the capture envelope, and `DOCK`. What
differs is who the station lets in and who flies the pair.

| rule | what happens |
|---|---|
| admission | a living station reads its `allegiance`: `Some(Enemy)` refuses every dock, any other side (or `None`) admits. A derelict admits whatever its side; it is a hull to loot |
| the refusal | while the travel lock is on a station that refuses, the HUD mode chip reads `<NAME> REFUSES DOCKING` with the station object's `name`, the DOCK chip reads `DOCK REFUSED`, and `DOCK` builds nothing |
| one connection | a hull holds one connection. A station with several ports still admits one ship; a second ship meets `<NAME> DOCK OCCUPIED` until the first lets go |
| a living station's helm | the station owns the pair's helm. `HELM` is refused, the HUD reads `STATION HELM`, and the docked ship's drive, RCS and maneuvers are held. The station has no thruster, so the pair never moves; its bridge holds the pair level on the station's parked attitude |
| a dead station's helm | the docked player holds the helm from the capture, as after `HELM` on a ship, because the derelict's bridge is inactive. The player's burn tows the pair |
| leaving | `DOCK` again, from the ship, as always |

The proof is `system_station_docking`: a living station holds the helm under
a held burn, a dead station hands it over, an Enemy station refuses with the
line on the HUD.

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

## Generated-world stations

The open world places stations itself. Every civilization keeps its stations
at the clusters nearest its centroid: a living one keeps one, plus one per
half of its advancement (one to three), and an extinct one left exactly one.
A cluster holds at most one station; when two civilizations want one, the
nearer centroid keeps it. The choice is a function of the world seed and the
civilization alone, so a cluster holds the same station from every cell and
in any visit order.

Where it stands follows the cluster. In a cluster with a planetoid the
station rides a circular orbit around one world, at that world's radius plus
the 500 m clearance margin plus the station's own clearance, with the orbit
velocity the [behaviour table](#how-a-station-behaves) names. In a cluster
without one it stands at rest beside one of the cluster's rocks, 500 m
clear of it, on the anchor-facing side when that side is free. A station is
never placed inside another body, never across a cell face, and never
where its cluster placed no rock or planetoid in its cell.

Every civilization builds `block_waystation` today. The generator pins each
design it places from the loaded catalog when the open world arms; a base
bundle that authors none of those ids fails `content -- lint`, and a mod that
redefines `block_waystation` rebuilds every generated station with it.

A generated station spawns with a typed generated id and no scenario
address, freezes as a station record when its cell retires, and thaws from
that record: what a docked ship bought or stored is kept for the session,
and across a save on the desktop build.

## Dead stations

An extinct civilization's station is the SAME design spawned dead: its
`Station` spawn carries `derelict: true`, so its hull spawns inactive and
lootable, with `DerelictShipMarker` and `LootableShipMarker`, and no service
is offered on it. There is no second catalogue entry for a dead station; the
strip is the spawn flag, the same one a scenario's own `Station` object
sets. A dead station keeps the Neutral side, and admits a dock on any side:
see [Docking at a station](#docking-at-a-station).

## Base stations

| id | what it is |
|---|---|
| `block_waystation` | Waystation - a plated two-deck block, a bridge set into the top deck, a docking collar off the port flank (`port_collar`). Offers Rebuild, Refit, Trade and Storage; its hold opens with hull plates, rations and salvaged parts |
