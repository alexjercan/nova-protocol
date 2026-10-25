# Price tables for mods

A `PriceTable` holds the factors a generated trader prices every item by: the
scarcity curve, the demand of each cluster type and civilization role, and
the clamp on the product. There is one table, not a registry. The base game
states it, and a mod that states one replaces the whole table.

This page is the field-by-field price table reference. For general RON
spelling rules, see
[RON spelling rules](../reference/#how-ron-content-is-written).

## How a trader prices

A living generated station and an intact generated ship price each item at
the moment you trade, from its market:

```text
ask = reference ask_cr x scarcity.ask x cluster.ask x civilization.ask
bid = reference bid_cr x scarcity.bid x cluster.bid x civilization.bid
```

Each side's product is clamped to `clamp`, then the price rounds to the
nearest credit.

| factor | from |
|---|---|
| reference | the item's `ask_cr` and `bid_cr`, see [Content files](../mod-files/#content-files) |
| scarcity | the trader's stock of the item against its target, the stock it spawned with. At no stock the factor is `starved`, at the target 1, at `glut_share` times the target and above `glut`, and linear between. An item the trader spawned without uses `unkept` |
| cluster | the `clusters` rule for the cluster type the trader stands in and the item's category, or 1 when no rule names the pair |
| civilization | the `roles` rules for the item's category, each weighed by the civilization's preference for that role, a missing rule counting as 1 |

An authored [station](../stations/) or ship has no market. It deals at the
reference price. A Buy at a station the player owns pays the bid.

## How a trader restocks

A generated trader's stock stands still while its cell stands frozen. On the
thaw it drifts toward its target stock by the play time it stood frozen: a
starved trader buys back up, a glutted one sells back down, each by
`target * elapsed / recovery_seconds` whole units, never past the target.
An item the trader's target does not name is untouched - what you sold it
stays sold. A reload alone restocks nothing: the clock stops while the game
is paused, so no play time passes.

Neutralizing a civilization's generated hull or station is an offence
against it, not a price rule: see
[Civilization stances](../stations/#civilization-stances).

## How a job offer expires

A living station's job board has three slots, each on its own posting. An
offer the player does not hold expires after `jobs.offer_seconds` of play,
or `jobs.escort_offer_seconds` for an escort, measured from its posting.
The slot then posts a new offer under a new id. A held job
is pinned to its slot and never expires, however long you hold it; handing
one in, or losing it (its hauler lost), renews the slot at once. The clock
is play time, so the pause menu, the interface and a Load all freeze every
offer's countdown along with everything else. See [Jobs](../stations/#jobs)
for the full posting and renewal rule.

## The PriceTable item

```ron
[
    PriceTable((
        clamp: (min: 0.25, max: 3.0),
        scarcity: (
            starved: (ask: 1.5, bid: 1.25),
            glut: (ask: 0.9, bid: 0.6),
            glut_share: 3.0,
            unkept: (ask: 1.0, bid: 0.9),
        ),
        restock: (recovery_seconds: 1800.0),
        jobs: (offer_seconds: 1200.0, escort_offer_seconds: 3600.0),
        clusters: [
            (cluster: AsteroidRich, category: Raw, factor: (ask: 0.6, bid: 0.6)),
        ],
        roles: [
            (role: Civilian, category: Food, factor: (ask: 1.25, bid: 1.25)),
        ],
    )),
]
```

Every field is required.

| field | type | meaning |
|---|---|---|
| `clamp` | `(min, max)` | the lowest and highest total factor on each side |
| `scarcity` | scarcity rule | the stock curve, see below |
| `restock` | `(recovery_seconds)` | the play seconds a thawed trader's stock takes to return to its target from none, see [How a trader restocks](#how-a-trader-restocks) |
| `jobs` | `(offer_seconds, escort_offer_seconds)` | play seconds an unheld non-escort or escort offer stands before its slot posts a new one, see [How a job offer expires](#how-a-job-offer-expires) |
| `clusters` | list of cluster rules | demand by cluster type: `cluster` is `AsteroidRich`, `RockOnly`, `PlanetHeavy` or `DerelictField`; `category` is an item category; `factor` is `(ask, bid)` |
| `roles` | list of role rules | demand by civilization role: `role` is `Civilian`, `Industrial`, `Scavenger` or `Armored`; `category` and `factor` as above |

A scarcity rule has four fields:

| field | type | meaning |
|---|---|---|
| `starved` | `(ask, bid)` | the factor at no stock. Each side at least 1 |
| `glut` | `(ask, bid)` | the factor at `glut_share` times the target and above. Each side at most 1 |
| `glut_share` | number | the stock, as a multiple of the target, where `glut` applies in full. Above 1 |
| `unkept` | `(ask, bid)` | the factor for an item the trader spawned without |

## Overlay and lint

The last bundle in load order that states a table wins, whole. The base
bundle must state one.

`content lint` and the content check at load both report an error for a
pack that:

- states two tables;
- is the base game and states none;
- states a clamp whose `min` is 0 or less, or that does not hold 1;
- states a factor that is not a finite number, or that lies outside the clamp;
- states a factor whose bid is above its ask, which could push a bid above its
  ask;
- states a `starved` side below 1, a `glut` side above 1, or a `glut_share`
  of 1 or less;
- states a `restock.recovery_seconds`, `jobs.offer_seconds` or
  `jobs.escort_offer_seconds` that is not a finite number above 0;
- names one cluster type or one role with one category twice.

With every factor's bid at most its ask, a trader's bid is never above its
ask.

## Base price table

| rule | factor |
|---|---|
| clamp | 0.25 to 3.0 |
| starved | ask 1.5, bid 1.25 |
| glut, at 3 times the target | ask 0.9, bid 0.6 |
| unkept | ask 1.0, bid 0.9 |
| restock recovery | 1800 s (30 min of play) |
| non-escort offer window | 1200 s (20 min of play) |
| escort offer window | 3600 s (60 min of play) |
| `AsteroidRich` `Raw` | 0.6 |
| `RockOnly` `Raw` | 0.8 |
| `PlanetHeavy` `Raw` | 1.5 |
| `DerelictField` `Parts` | 0.7 |
| `DerelictField` `Repair` | 0.85 |
| `Industrial` `Raw` | 0.8 |
| `Civilian` `Food` | 1.25 |
| `Scavenger` `Parts` | 0.8 |
| `Armored` `Ammo` | 1.25 |
| `Armored` `Repair` | 1.15 |

The table is generated into `assets/base/market/base.content.ron`.

The proof is `market_sandbox`: it sells iron ore at a living asteroid-rich
station and at a living planet-heavy one through the Inventory pane, checks
each Sell pays the computed bid, and checks the asteroid-rich bid is the lower.
The restock proof is `system_station_restock`.
