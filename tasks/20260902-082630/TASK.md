# Spend the lance's wasted power on width: the rake blast

- STATUS: OPEN
- PRIORITY: 62
- TAGS: v0.13.0,ship,weapon,balance

Split out of the review of `20260824-125947` on 2026-09-02, at owner
direction. The lance shipped and reads WEAK beside PDC spam. It is not
weak. It is aimed through a budget no shipped ship is thick enough to
spend.

## The measurement

`slug_power: 1800`, and a Pierce round pays `max_health /
pierce_power_multiplier` to cross a layer. A 1500 u/s slug pins that
multiplier at its `PIERCE_POWER_CEILING` of 3.0, so a 200 hp reinforced
cell costs 66.7. The budget therefore buys **27 layers**. The lance also
carries `layers: u32::MAX`, so nothing else bounds it.

No shipped craft is more than about six cells deep along a line of fire.
A four-cell corvette line spends 267 of 1800: **85 percent of every shot
leaves through the far side.**

What that costs, per 13.5 s cycle (1.5 s charge + 12 s reload), against
four cells of reinforced hull:

| weapon | reach | sustained | intercept |
|---|---|---|---|
| PDC kinetic | 200 u | 267 dps | none possible |
| Railgun lance | 1800 u | 59 dps | none possible |
| Torpedo (Serpent) | 3200 u | 75 dps | ~370 PDC rounds each |

The gun is 4.5 times worse than the weapon it is meant to outrange, and
it costs a 1.5 s unabortable commit to fire.

## What this task wants

Convert the surplus DEPTH into WIDTH. Owner's framing: a slug that
punches a needle should leave a hole you can see.

Proposed shape, to be proved rather than assumed:

- Two new optional fields on `RailgunSectionConfig`, `rake_blast_radius`
  and `rake_blast_damage`. Both `Option`, both omitted meaning exactly
  today's behavior, so the gun is authorable per mod and the base
  catalog is the only thing that changes.
- At each layer the slug crosses, fire the shipped `apply_blast_damage`
  at the crossing point, alongside the flat 300 Pierce.
- Starting numbers to measure from: `radius 4.0, damage 60.0`. A
  neighbour 1 u off the axis takes `60 * (1 - 1/4) = 45` per crossed
  layer, so a four-cell rake puts 180 into each neighbour (heavy, not a
  kill on a 200 hp cell) and a six-cell rake puts 270 in (a kill). The
  widening SCALES with how much ship the slug actually found, which is
  the property worth having: it blooms in a hull and stays a needle in a
  fighter.

Reusing `apply_blast_damage` is the point. Its rules are already shipped
and tested - pressure falls linearly to zero at the edge, a surviving
section shields what is behind it, a destroyed one transmits 65 percent,
and one crater is cut per BODY rather than per collider.

## Explore before committing to the shape

- The blast's own shielding rule interacts with the tunnel the slug is
  cutting: layers already destroyed transmit freely, so a late blast
  reaches further back than an early one. That is physically sensible
  and may read well. It needs a probe, not an argument.
- A variant worth weighing: spend LEFTOVER power at the exit instead of
  a fixed per-layer blast, so a lance that over-penetrates a corvette
  blooms and one that barely gets through a capital hull does not. More
  elegant, more novel, more risk. Ship the fixed version first if the
  measurement does not clearly favour this.
- Whether the rake blast should carry its own `DamageType`. It is
  Explosive today by reuse; a lance's spall is not a warhead, and the
  impact table will voice it as one.
- What it does to carve on rocks. `mark_radius(200)` is already 2.29 u,
  so the lance does NOT look narrow on asteroids. Adding blasts there
  may over-cut.

## Done when

- The base lance leaves a visibly wider hole than its bore, cut as the
  slug passes through, and a probe run shows the widened bite.
- Effective dps against a four-cell line is recorded before and after,
  in the same rig, and sits in a defensible place against the PDC's 267.
- The two fields are documented in the creator reference's Railgun
  chapter, which now exists.
- If the rake lands, the reload is NOT also shortened. It roughly
  triples effective per-shot damage on its own.

## Not this task

`AI_STANDOFF_RANGE` is 100 u while the lance reaches 1800 u, so every
fight collapses to inside PDC range and the lance never gets its window.
That is the OTHER half of the balance problem and it is fight geometry,
not a weapon stat. It wants its own task.
