# Second-tier sections and items with real sinks for the shipyard and swap

- STATUS: OPEN
- PRIORITY: 70
- TAGS: v0.18.0,content

Planned 2026-11-11 from the v0.17.0 post-release review. Owner direction:
more content, items and sections, so the shipyard and family swap have
something to upgrade into.

## Agent findings
- Base pack: 20 items in 5 categories (`assets/base/items/base.content.ron`;
  categories `crates/nova_gameplay/src/inventory.rs:323-334`): Raw x4, Ammo x3,
  Repair x1, Food x1, Parts x11. Nothing consumes ore except selling it.
- 20 sections in 9 kinds (`crates/nova_ship/src/sections/base_section.rs:465-482`):
  Hull 6, Thruster 3, Turret 4, Torpedo 2; Controller, Railgun, Docking,
  CargoIntake, Mining one each.
- Tiered shipyard and same-family swap landed in tasks 20261012-100012 and
  -100014 with little to swap into.
- Item and recipe prices are marked provisional
  (`crates/nova_authoring/src/base_content/items.rs:14`,
  `sections/recipes.rs:6`); mining beam stats provisional
  (`sections/mining_beam.rs:18,107`).
- Generated hulls build from parts for 4 roles (`crates/nova_gameplay/src/market.rs:63-72`);
  new sections only reach generated ships through the layout tables in
  `crates/nova_world_base/src/ship_layout.rs`.

## Decisions (pending owner review)
- Section slice: one second-tier variant for each single-variant kind:
  heavy cargo intake (wider mouth, more mass), long-range mining beam,
  armoured controller (more health, slower turn), fast docking port (wider
  capture gates). Plus one more thruster and one more torpedo warhead.
  Each needs a .glb; reuse existing meshes with scale and material where no
  new art exists, and list the art gap as capturable.
- Item slice: three trade goods with real sinks (e.g. Coolant used by station
  restock, Alloy from Iron + Carbon ore at an industrial station, Medkits
  bought by Distress jobs) and one contraband item refused by living stations
  with a Lawful stance. Every item gets `price_lot`, ask and bid, and a trade
  profile row so generated traders carry it.
- Generated layouts must use the new variants by civilization advancement
  (`civilizations.rs`), or they never appear outside the shipyard.
- All content goes through the Rust builders and `content -- gen`. Never
  hand-edit base RON.

## Delivery
- Owners: `nova_authoring` base_content items, sections, recipes, trade
  profiles; `nova_world_base` ship_layout and civilizations;
  `nova_gameplay` inventory categories if a new category is needed.
  Dies: nothing. May break: lint ranges, the berth census test in
  `nova_authoring`, ledger webmod expectations, price tables.
- Sequence: sections -> layouts -> items and sinks -> trade profiles -> wiki
  catalogue pages and changelog.

## Verification
- Lint passes on the regenerated pack; census test counts the new sections.
- Asserted example: a shipyard at advancement 3 offers the tier-two intake and
  the swap keeps cargo; a generated Industrial hull at advancement 3 carries a
  tier-two section.
- Trade probe: a generated trader sells the new goods at a station that
  stocks them, and the contraband is refused at a Lawful station.

## Done when
- No section kind has a single variant.
- At least three items have a consumer other than the sell button.
- Generated ships and shipyards show the new content without authoring.
