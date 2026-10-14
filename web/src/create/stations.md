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
            integrity: (collapse_threshold: Some(0.5)),
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
| `hull` | design | the same fields a [ship design](../ships/#the-design) takes: `sections`, `integrity`, `presentation`. It must hold a `Docking` section and a `Controller` section and no `Thruster` section, and author `integrity.collapse_threshold`: the share of its hull a station collapses at, see below |
| `services` | list | what a docked ship may do here, from the closed set `Rebuild`, `Refit`, `Trade`, `Storage`, `Jobs`, `Roster`. At least one |
| `storage_cap_g` | whole number | how many grams the station's storage takes. Above zero exactly when `services` lists `Storage` |
| `stock` | stock | what the hull's own hold opens with, in the [stock format](../objects/#inventory). It must fit the hull's hold (100 kg per hull section) |
| `credits` | whole number | the credits the station trades with at spawn |

The `Docking` section is what a ship docks to. The `Controller` section is
what levels the station, see below.

A docked ship trades with a living station that offers `Trade` as it trades
with a ship, and keeps items in the storage of one that offers `Storage` (see
[Trade and storage](#trade-and-storage)). `Rebuild` and `Refit` are in play
too (see [Rebuild](#rebuild), [Swap](#swap), [Add](#add), [Buy a hull](#buy-a-hull) and [Hire an escort](#hire-an-escort)); `Jobs` and `Roster` are
checked and carried, and the screens that read them are their own task.

## How a station behaves

A station is a dynamic body, never a fixed one. Gravity pulls it, a ram
shoves it, and nothing holds it where it was placed: where a hit leaves it is
where it stays, and a world save keeps that pose and motion.

| behaviour | rule |
|---|---|
| self-levelling | the `Controller` section parks its target attitude on the spawn rotation and drives the hull with torque alone. Spun by a hit, the hull comes to rest within `STATION_LEVEL_TOLERANCE_RAD` (0.03 rad, about 1.7 degrees) of that attitude. No thrust is ever applied: only a `Thruster` section writes thrust, and the lint refuses one |
| damage and collapse | a station takes damage, collapses and is destroyed as a ship is. At or below `collapse_threshold` of the pool it was built with (0.5 on every base station) it is peeled apart section by section and its root is destroyed, a dead station included. A sever counts: the pieces cut away leave its pool, so a cut that leaves the kept half at or below the threshold collapses it. A destroyed station drops its hold and the player's storage as cargo canisters, releases a docked ship and is gone for good: its cell's record no longer holds it |
| turning dead | a living station whose last turret or last `Controller` section is lost (an unarmed station: its last `Controller`) turns into a dead station on the spot: services stop, it goes Neutral and lootable, its sections other than structure and docking ports switch off, and a docked player gets the helm. Hold and storage stay. See [Dead stations](#dead-stations) |
| the guns | a living station is AI-driven. Its turrets aim, its point defense covers it, and it answers fire; with no thruster the AI never moves it. An `Enemy` or `Player` station fires on hostile sides; a `Neutral` one only answers a ship that has dealt it 30 hit points in quick succession, so a scrape or a stray round does not provoke it. Primary fire holds while a ship is docked, and point defense never fires through the docked hull |
| severed pieces | a piece cut off a station leaves at the separation speed while the station keeps its own velocity, so a cut never puts a station on a new orbit |
| orbit | a station spawned in a planet's well rides a circular orbit: its `initial_velocity` is `sqrt(mass / r)` on the tangent of its radial with +Y (with +X when the radial is near +Y), in engine units, the speed the gravity pull integrates. The generator sets it; a scenario that parks a station in a well authors the same number or watches it fall |
| orbit tolerance | over three orbits, with a hit, the radius stays within `STATION_ORBIT_TOLERANCE` (15 percent) of where it started. The pull is held constant over each physics step, so every orbit widens a little (about 1.5 percent of its energy per orbit on the proof's fast 30 s orbit, less on a real planetoid's), and a kick of one percent of orbital speed swings the radius about four percent; past the tolerance the orbit is walking out of the well or down onto the planet |

The proofs are `system_station_body` (a nudged station in a void) and
`system_station_orbit` (the yard station with a tender docked at each port
but one: a hit that damages it without collapsing it, a shove and three
orbits of a small planet).

## Docking at a station

A ship docks with a station through its `Docking` section as it docks with a
ship: a free port on each hull, inside the capture envelope, and `DOCK`. What
differs is who the station lets in and who flies the pair.

| rule | what happens |
|---|---|
| admission | a living station reads its `allegiance`: `Enemy` refuses every dock, and any side refuses a ship it is hostile to (a `Player` station and an Enemy-flagged ship, or a station answering that ship's fire). A derelict admits whatever its side; it is a hull to loot. A station you [own](#buy-the-station) admits you whatever your side |
| the refusal | while the travel lock is on a station that refuses, the HUD mode chip reads `<NAME> REFUSES DOCKING` with the station object's `name`, the DOCK chip reads `DOCK REFUSED`, and `DOCK` builds nothing |
| one ship per port | a ship holds one connection; a station holds one per docking port. A ship docks while any port is free; once every port is held the next ship meets `<NAME> DOCK OCCUPIED` until one lets go. The station and every ship docked at it move as one body |
| a living station's helm | the station owns the helm of everything docked at it. `HELM` is refused, the HUD reads `STATION HELM`, and each docked ship's drive, RCS and maneuvers are held. The station has no thruster, so the assembly never moves; its bridge holds it level on the station's parked attitude |
| a dead station's helm | the docked player holds the helm from the capture, as after `HELM` on a ship, because the derelict's bridge is inactive, and gets it the moment a living station it is docked to turns dead. The player's burn tows the station and every ship docked at it |
| losing the port | a docking port destroyed while a ship is docked to it releases the dock |
| leaving | `DOCK` again, from the ship, as always |

The proof is `system_station_docking`: a living station holds the helm under
a held burn, a dead station hands it over, an Enemy station refuses with the
line on the HUD.

## Trade and storage

A ship docked to a living station uses the station from the Inventory pane.
Every service below reads one gate: the partner is the docked station, it is
not a derelict, and its `services` list the service.

| rule | what happens |
|---|---|
| trade | with `Trade`, the station's column lists its hold: the station's row opens Buy, the player's row Sell, at the item's reference `ask_cr` and `bid_cr`, against both purses. Without `Trade`, Buy and Sell refuse with `<NAME> does not trade` |
| storage | with `Storage`, a third column lists the station's storage and its used mass against `storage_cap_g`. The player's row opens Store, the storage row Withdraw. No credits move |
| one store | a station keeps one storage, for the player alone. Nothing else stores or takes from it |
| refusals | Store and Withdraw refuse in this order, on the form with Confirm disabled, and on the note line: not docked, no `Storage` service, unknown item, no quantity, quantity zero, fewer items than the quantity, then `storage full, <N> kg free` for a Store or `<SHIP> has room for <N> more` for a Withdraw |
| persistence | the storage freezes with the station when its cell retires, thaws with it, and is in the world save; a save checks it against `storage_cap_g`. A destroyed station drops it as cargo canisters with its hold |
| events | `OnTraded` for a trade; `OnStored` and `OnWithdrawn` for a store or withdraw, the station as `id` and the player as `other_id`, see [Hold events](../events/#hold-events) |
| a derelict | a dead station offers no service. Its hold is loot: Take and Give, as for a derelict ship |

The proof is `system_station_storage`: on a generated world, the player docks
at a living waystation, buys, sells and stores through the pane, leaves until
the station's cell retires, comes back, withdraws the same stacks, meets the
over-cap refusal, and finds the storage in the leave save and after a Load.

## Rebuild

A ship docked to a living station that offers `Rebuild` can rebuild a section
it lost. The Ship pane lists each lost section under its old code (`PDC-1`)
with a price or the reason it cannot be built. A lost section is one the
ship's design places and the ship no longer has.

| rule | what happens |
|---|---|
| the gate | docked to a living station whose `services` list `Rebuild`. A dead station and a ship offer no rebuild |
| the recipe | the section's [build recipe](../sections/#build-recipe): its `credits` plus every part |
| parts | taken from the ship's hold first; the rest are bought from the station's stock at ask |
| credits | the ship pays the recipe's `credits` plus the bought parts. The station receives all of it |
| refusals | in this order: not docked, no `Rebuild` service, section not lost, no live section to attach to where the design places it, a part neither hold can cover, too few credits |
| the result | the section spawns where the design places it, at full health, with the key binding it had when it was lost |
| events | `OnRebuilt` and one `OnCreditsChanged` per party with `cause: Rebuild`, see [Hold events](../events/#hold-events) |

A section code reads the ship's design, not its live sections, so a lost
section keeps its code and a rebuilt one gets it back. A key binding the
player set on a section is kept when the section is lost and across a save.

The proof is `system_station_rebuild`: a ship loses a rebound PDC, docks,
rebuilds it from hold and bought parts, and keeps it with its binding across
a freeze and thaw.

## Swap

A ship docked to a living station that offers `Refit` can swap a live
section for another part of the same family. Select the section in the Ship
pane: the panel lists each part it can take, with a price or the reason it
cannot be paid.

| rule | what happens |
|---|---|
| the gate | docked to a living station whose `services` list `Refit`. A dead station and a ship offer no swap |
| the family | the same kind of section (a turret for a turret) that fills the same cells, read from its `collider`, and not the part the section already is. Parts marked `hide_in_editor` are not offered |
| the recipe | the new part's [build recipe](../sections/#build-recipe): its `credits` plus every part, paid as a rebuild pays it |
| the old part | it is removed. Nothing is refunded |
| the build checks | the ship the swap leaves must still join into one ship, keep every firing and exhaust lane clear, and keep enough thrust and flight computers for its mass. A swap is refused only for a check it breaks, not for one the ship already failed |
| refusals | in this order: not docked, no `Refit` service, another kind of section, another size, the part the section already is, a part neither hold can cover, too few credits, a build check |
| the result | the new part spawns where the old one stood, at full health, under the same code and with the old part's key binding. The ship keeps the new part across a save |
| events | `OnSwapped` and one `OnCreditsChanged` per party with `cause: Swap`, see [Hold events](../events/#hold-events) |

The proof is `system_station_swap`: a ship docks, a thruster swap for a PDC is
refused, a damaged and rebound PDC is swapped for the pierce PDC, and the new
part keeps its binding and its health across a freeze and thaw.

## Add

A ship docked to a living station that offers `Refit` can add a part at a
free attachment point. A free point is any socket of a live section that no
section of the ship's design mates, so every hull has them and no content
marks them. Select the section in the Ship pane: the panel shows its free
points as buttons, the first one chosen, and lists each part that fits at the
chosen point with a price or the reason it cannot be added.

| rule | what happens |
|---|---|
| the gate | docked to a living station whose `services` list `Refit` |
| the point | a socket no design section mates. A lost section's socket stays reserved for its rebuild |
| the fit | the part mates the point with the socket that keeps it as drawn, and must not overlap a section it does not mate. Parts marked `hide_in_editor` are not offered |
| the recipe | the part's [build recipe](../sections/#build-recipe), paid as a rebuild pays it |
| the build checks | the ship the add leaves must still join into one ship, keep every firing and exhaust lane clear, and keep enough thrust and flight computers for its mass. Only a check the add breaks refuses it |
| the binding | the key of the first live section of its kind that has one. With none, a weapon takes an empty binding to set with Rebind and a thruster none. A mining emitter takes the first free key of `V`, `B`, `N`, `C`, refused when all are held |
| refusals | in this order: not docked, no `Refit` service, a taken point, a part with no socket, an overlap, a build check, no free mining key, a part neither hold can cover, too few credits |
| the result | the part spawns at full health under the next code of its kind. The ship keeps it across a save |
| events | `OnAdded` and one `OnCreditsChanged` per party with `cause: Add`, see [Hold events](../events/#hold-events) |

The proof is `system_station_add`: an add at a taken socket is refused, a
kinetic PDC is added to a base hull through the pane and to a generated hull,
and each keeps its binding and its health across a freeze and thaw.

## Buy a hull

A living station that a generated world places sells one whole hull per
docking port but one, so one port is always free for the player. A hull
that fits at no free port without touching the station or another parked
hull is not offered. The world lays them out for the station's civilization
from the world seed and the station's id, so a station sells the same list on
every visit and keeps it across a save. Each hull's roles follow what the
civilization can build, and its size grows with the civilization's
advancement: a station near the origin sells small hulls only. A station a
scenario places sells none.

Every hull for sale is a real ship parked at one of the station's ports. A
parked hull is inert: nobody flies it, it is Neutral, it does not shoot, and
it cannot be looted or stripped. Docked to a living station that offers
`Refit`, the Station pane lists the parked hulls under **Ships for sale**:
each hull's name, length, section count and kinds, the price and the
trade-in, and a Buy button or the reason it is refused.

| rule | what happens |
|---|---|
| the gate | docked to a living station whose `services` list `Refit`, with a hull parked |
| the price | every section's [build recipe](../sections/#build-recipe) at ask: its `credits` plus its parts at the item's `ask_cr`. A rebuy costs the trade-in the station paid |
| the trade-in | the station buys the old hull: each live section's `strip_yield` share of its recipe parts at the item's `bid_cr`, times its health share, rounded down per section. A lost section pays nothing |
| the swap | the player takes the controls of the parked hull, which undocks and redocks with the station. The old ship parks where it stood, listed as `(rebuy)` at the trade-in |
| the hold | the old hold stays in the old ship. While docked at that station, its hold lists as the player's storage. The new hold opens empty |
| the credits | the ship pays the price less the trade-in. When the trade-in is worth more, the station pays the difference |
| the binding | the start-role keys for the new hull: turrets the trigger, railguns `R`, torpedoes `F`, mining emitters `V` then `B`, `N`, `C`. No old binding carries over |
| refusals | in this order: not docked, no `Refit` service, too few credits with the trade-in, a station that cannot pay the trade-in |
| the result | the new hull docks where it was parked, at full health. The ship and the parked hulls keep across a retire, a save and a Load |
| events | `OnShipBought` and one `OnCreditsChanged` per party with `cause: ShipBuy`, see [Hold events](../events/#hold-events) |

The proof is `system_station_buy`: on a New Game, the station parks its hulls
for sale; the player buys one through the pane, the old ship parks for
rebuy, the station's cell retires and thaws, the player buys the old ship
back, and the player's hull and the parked hulls come back after a save and
a Load.

## Hire an escort

After its hulls for sale, a living station that a generated world places
hires out one to three armed escorts: scavenger and armored hulls its
civilization can build, laid out from the world seed and the station's id. A
civilization that builds no armed role hires none, and a station a scenario
places hires none. The Station pane lists them under **Escorts for hire**: each
escort's name, length, section count and kinds, the price, and a Hire button
or the reason it is refused.

| rule | what happens |
|---|---|
| the gate | docked to a living station whose `services` list `Refit`, that hires out escorts |
| the price | half again the hull's price: every section's [build recipe](../sections/#build-recipe) at ask, times 3/2 |
| the fleet | two slots, starboard then port. A third hire is refused |
| the credits | the ship pays the price and the station takes it |
| refusals | in this order: not docked, no `Refit` service, a full fleet, too few credits, a station balance that would overflow |
| the result | the escort leaves the station's list and spawns on your side, beside your ship, with an empty hold. It flies on its own drive to its place off your side and behind your travel, matches your speed, holds where it is while you stop, and fights what attacks you |
| sectors and saves | an escort flies with you across sectors and the world save keeps it in your fleet. A neutralized escort leaves the fleet and stays in its sector as a wreck |
| events | `OnEscortHired` and one `OnCreditsChanged` per party with `cause: EscortHire`, see [Hold events](../events/#hold-events) |

The proof is `system_station_hire`: on a New Game, the player hires an escort
through the pane, crosses sectors with it, sees it fly formation through a
turn and finds it beside the ship after a save and a Load; a killed escort is gone from the fleet after the next save
and Load.

## Buy the station

A living station that a generated world places is for sale to the ship docked
with it. The Station pane shows it under **This station**: its name, its price
and a **Buy station** button or the reason it is refused. A station a
scenario places and a derelict are not for sale.

| rule | what happens |
|---|---|
| the gate | docked to a living station a generated world placed. No `services` entry is needed |
| the price | twice what the station is worth at ask: every live section's [build recipe](../sections/#build-recipe), every stack of its hold at the item's `ask_cr`, and its credits. Your storage there is already yours and is not priced |
| the credits | the ship pays the price to the civilization. The station keeps its own credits, stock and services |
| refusals | in this order: not docked, not for sale, already yours, too few credits |
| what becomes yours | the station leaves its civilization: it takes the Player side, so its guns fire on your foes, and it is named `Owned station`. It admits you whatever your side. Its hold sells to you at the item's `bid_cr`; storage withdraw stays free. Its services still price at ask, and Take does not reach its credits |
| what stays | the station keeps its stock, credits, services and the hulls it sells. It earns no income yet, and you cannot set its stock targets yet |
| the Map | the chart holds it as yours: the chart view shows `OWNED STATION` in the ally colour |
| sectors and saves | the station stays yours, on your side and under its new name, when its sector retires and thaws, and the world save keeps it so |
| events | `OnStationBought` and one `OnCreditsChanged` for your ship with `cause: StationBuy`, see [Hold events](../events/#hold-events) |

The proof is `system_station_own`: on a New Game, the player docks at a
Neutral generated station, buys it through the pane and sees it take the
Player side and the owned name, leaves its sector and comes back, saves and
Loads, and finds it still the player's, on the chart and the Map too. An
Enemy-flagged ship, which its Player side would refuse, is admitted.

## Spawning one

A scenario parks a station with a [Station](../objects/#station) object,
naming the design by id:

```ron
SpawnScenarioObject((
    base: (id: "outpost", name: "OUTPOST", position: (0.0, 0.0, -1500.0), rotation: (0.0, 0.0, 0.0, 1.0)),
    kind: Station((
        design: "my_outpost",
        allegiance: Neutral,
        derelict: false,
        initial_velocity: (0.0, 0.0, 0.0),
    )),
)),
```

A station spawn takes no section patches and no inline design: a station is
the design, and a one-off station is one more `Station` item.

The Sandbox editor writes the same pair. A ship marked **Station** in its
inspector saves as a `Station` item under the ship's node id and a `Station`
object that spawns it. The editor writes the hull and its side and fills the
rest: the four services, a 100 kg storage cap, 5000 credits, and the base
waystation's hull plates, rations and salvaged parts cut to fit the hull's
hold. Driver **None** saves `derelict: true`. The content lint checks the
saved station like any other.

## Overlay and lint

Overlay is last-wins by id, as for every content kind: a mod that defines a
`Station` with a base id replaces it whole.

The static lint and the load gate both refuse a station that:

- names a section prototype no loaded content declares, or gives two
  sections one id, or a reserved id;
- has no `Docking` section (nothing could dock with it);
- has any `Thruster` section (a station never flies);
- has no `Controller` section (nothing would hold its attitude);
- authors no `integrity.collapse_threshold`, or one outside `0..=1` (a
  station states the share it collapses at);
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

A living station's hold and credits scale with its civilization's
advancement: every stock line and the credits are multiplied by a share that
runs from 0.5 at advancement 0 to 1.0 at advancement 1. A line floors but
never below one, so every item the design stocks is still stocked, and the
credits round. The share never passes 1, so the stock always fits the hold
the lint checked. A dead station keeps the design's stock and credits as
loot.

Each civilization builds one of `ring_station`, `spire_station` and
`yard_station`, drawn from the world seed and the civilization alone, so all
of its stations share a look and a cluster shows the same station from every
cell. The generator pins those designs from the loaded catalog when the open
world arms; a base bundle that authors none of those ids fails
`content -- lint`, and a mod that redefines one rebuilds every generated
station drawn as it. `block_waystation` is for scenarios; the generator never
places it.

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

A living station fought down to its last turret or last flight computer
turns into the same dead station in place (see the
[behaviour table](#how-a-station-behaves)), and freezes, thaws and saves as
one.

## Base stations

| id | what it is |
|---|---|
| `block_waystation` | Waystation - a plated two-deck block about 40 m across, unarmed, with a bridge set into the top deck, a second flight computer (`lower_bridge`) in the far corner of the lower deck and a docking collar off the port flank (`port_collar`). Offers Rebuild, Refit, Trade and Storage; its hold opens with hull plates, rations, salvaged parts and a few of each hull, drive, controller and PDC part |
| `ring_station` | Ring Station - a plated hub on four truss spokes inside a ring, about 230 m across, in the `station` style: 12 PDC turrets, 5 flight computers, 4 docking collars, solar masts |
| `spire_station` | Spire Station - three armoured decks on truss columns, about 205 m tall, in the `armoured` style: 8 PDC turrets, 3 flight computers, 4 collars round the middle deck, solar wings |
| `yard_station` | Yard Station - a truss spine between two end blocks, about 270 m long, rows of solar panels either side, in the `industrial` style: 8 PDC turrets, 3 flight computers, 4 collars |

The three armed stations offer the waystation's services with about four
times its stock, plus 1000 PDC rounds for their own guns, and 20000 credits.
Every base station collapses at half its hull pool.
