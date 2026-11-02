# Trade profiles for mods

A `TradeProfile` says what a visiting ship trades with a station and which
jobs it posts on the station's board. The open world draws a profile for a
generated ship by its role. Trade profiles are content like items and start
roles. Create a new id to add a profile, or reuse a base id to replace that
profile.

The trade list and the job list are independent. Either or both may be
empty. An empty profile is valid traffic that only visits. A trade never
creates cargo or credits: a visit plans each trade against the ship's hold
and balance and the station's stock, balance and price.

This page is the field-by-field trade profile reference. For general RON
spelling rules, see
[RON spelling rules](../reference/#how-ron-content-is-written).

## The TradeProfile item

```ron
[
    TradeProfile((
        id: "my_mod/ice_runner",
        generated: [(role: Industrial, weight: 2)],
        trades: [
            (item: "WaterIce", side: Sell, quantity: 10, limit_cr: 3),
            (item: "Rations", side: Buy, quantity: 4, limit_cr: 9),
        ],
        jobs: [
            Procure(item: "HullPlate", count: 3, reward_cr: 220),
        ],
    )),
]
```

Every field is required.

| field | type | meaning |
|---|---|---|
| `id` | string | the profile id. A mod namespaces it, as the base game uses `base/...` |
| `generated` | list of `(role, weight)` | the roles of generated ships the open world draws this profile for. `role` is `Civilian`, `Industrial`, `Scavenger` or `Armored`; `weight` is above 0 and sets the draw odds against every other profile that names the role. Empty: only a script gives a ship this profile |
| `trades` | list of trade intents | what the ship trades on each visit, in order |
| `jobs` | list of visit jobs | what the ship posts on the station's board on each visit, in order |

A trade intent has four fields:

| field | type | meaning |
|---|---|---|
| `item` | item id | the item traded. Must be a known [item](../objects/#inventory) |
| `side` | `Buy` or `Sell` | seen from the visiting ship: a `Buy` moves items from the station into the ship, a `Sell` from the ship into the station |
| `quantity` | number | the most units traded on one visit, from 1 to 1000 |
| `limit_cr` | number | the price limit per unit, above 0. A `Buy` pays at most this times the count. A `Sell` earns at least this times the count. The visit trades the largest count that keeps the limit. An item priced under 1 cr a unit can never keep a `Sell` limit |

A visit job is `Procure(item, count, reward_cr)`: bring `count` units of
`item` to the station, for `reward_cr` paid on the hand-in. `count` and
`reward_cr` are above 0.

A ship whose role no profile names never visits.

## Overlay and lint

A later bundle that declares an id replaces the earlier profile whole, like
an item. Two profiles with one id in one bundle are a conflict: the merge
keeps the first and reports the second.

`content lint` and the content check at load both report an error for a
profile that:

- has an empty id;
- names an item that no visible bundle defines, in a trade or in a job;
- states a `quantity` of 0 or above 1000;
- states a `limit_cr`, `count` or `reward_cr` of 0;
- states a `weight` of 0;
- names one role twice in `generated`;
- names one item on one side twice in `trades`.

## Base trade profiles

Each base limit derives from the item's reference price per unit: a `Sell`
limit is half the bid, a `Buy` limit is one and a half times the ask, each at
least 1 cr.

| id | drawn for | trades | jobs |
|---|---|---|---|
| `base/ore_hauler` | `Industrial`, weight 3 | sells iron ore 12, stone ore 12, carbon ore 8; buys rations 4 | procure 6 water ice for 180 cr; 3 salvaged parts for 150 cr |
| `base/provisioner` | `Civilian`, weight 3 | sells rations 10; buys salvaged parts 4, water ice 6 | procure 3 hull plate for 220 cr |
| `base/salvage_broker` | `Scavenger`, weight 2 | sells salvaged parts 6; buys hull plate 2, PDC round 20 | none |
| `base/garrison_supply` | `Armored`, weight 1 | none | procure 40 PDC rounds for 200 cr; 6 rations for 160 cr |
| `base/idle_traffic` | `Civilian` 1, `Industrial` 1 | none | none |

The profiles are generated into `assets/base/trade_profiles/base.content.ron`.
