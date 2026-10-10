# Open-world progression: ship ownership, refit, salvage and stations

Research only, design A, on master `467ae965b` (2026-10-10). No code,
content, test, Cargo or GPU run. Nothing here is approved. Evidence is
`path:line` on master. **Hypothetical** marks proposed code, RON or rules.
**Unverified** marks a claim not traced to a call site. **Weak** marks an
external claim without a fetched primary source. This paper is independent
of the other worker's paper and does not edit `RESEARCH.md` or `TASK.md`.

Question: what does the player build and accomplish in the open world, how
does the ship get meaningfully better, where do stations fit, and which
seams can mods author? Focus: ship ownership and refit, salvage, and station
capability. Not a full X4 economy.

## 1. Verdict

1. The spine should be **salvage and refit**: the player's ship is an owned
   design that changes section by section. Every piece it needs already
   exists in some form: catalog sections, per-family capability scores,
   an advancement gradient by distance, derelict wrecks, docking, and a
   frozen-ship record that persists surviving sections.
2. Progression is gated by **section and design ownership**, not by item
   identity. The minimum refit slice does not need Plan D, but needs a live
   section-replacement path and an explicit saved-section-state transition.
   Plan D becomes a dependency when build costs name mod materials.
3. "Stations are easy" is false for anything that produces, trades with
   traffic, or hosts more than one ship. A **depot** (a persistent,
   non-producing body with storage and a refit bay) is the smallest honest
   station. Production needs a world clock that does not exist.
4. The starting ship is the strongest base hull. Any upgrade loop needs a
   weaker start or a harder world. That is an owner decision (section 9).

## 2. Verified current state relevant to progression

### 2.1 What the player has today

| Fact | Evidence |
|---|---|
| Open world starts in the line warship with 2,000 cr, 12 plates, 6,000 PDC, 20 slugs, 12 torpedoes | `crates/nova_authoring/src/base_content/scenarios/open_world.rs:72-101` |
| Weapon keys bind by section id | `open_world.rs:84`, `:107` |
| The only outcome is death, a Defeat | `open_world.rs:8`, `:62` |
| Credits buy and sell only items; prices are global per item | `crates/nova_gameplay/src/inventory.rs:83-112`, `plan_item_trade` `:492-534` |
| Any docked partner that is not neutralized or lootable trades | `inventory.rs:482-486` |
| Repair spends `HullPlate` on one section's health | `crates/nova_interface/src/ship/sections.rs:477-500` |
| Ore has no production sink; it can be sold for its bid (3-12 cr), stored or jettisoned | `crates/nova_scenario/src/mining.rs:151-157`, `inventory.rs:100-112` |
| Hold capacity is 100 kg per hull section | `crates/nova_scenario/src/objects/ship_design.rs:393`, `:402` |

There is no credit sink except items and no ore production sink. This does
not mean player actions vanish: visited sectors already persist altered
asteroids, ships and canisters in frozen records and native saves. On Defeat the menu offers Load last save in a saved world
(`open_world.rs:8-9`), so death costs progress since the last save, not the
world. Generated wrecks are the loot a docked player may Take
(`open_world.rs:6-7`).

### 2.2 What the world already generates

- Civilizations have an advancement in `[0, 1]` that rises with centroid
  distance from the world origin, plus a seeded offset
  (`crates/nova_world_base/src/civilizations.rs:40-44`, `:101-125`). The
  curve is still a TODO pending the owner's pick
  (`crates/nova_world_base/src/sector_ships.rs:55-59`).
- `ShipPartSnapshot` scores every usable section in one of 7 families (Hull,
  Controller, Thruster, Weapon, CargoIntake, Docking, Mining) and normalizes
  the score per family to the advancement needed to use it
  (`crates/nova_world_base/src/ship_parts.rs:1-25`, `:57-73`). Scoring is
  provisional (`ship_parts.rs:25`).
- The snapshot already refuses one section id from two unrelated packs: the
  Plan D collision rule exists for sections in this path
  (`ship_parts.rs:5-10`). The runtime merge still uses last-wins
  (`crates/nova_assets/src/merge.rs:850-852`).
- Generated layouts validate after build: unique ids, connected socket
  graph, clear exit and docking lanes, thrust floor, flight-computer count,
  mass as collider volume at density 1
  (`crates/nova_world_base/src/ship_layout.rs:15-33`).
- Ship roles are Civilian, Industrial, Scavenger, Armored
  (`crates/nova_world/src/generation.rs:169-178`). Role stock mixes and
  hold-share bands scale with advancement (`sector_ships.rs:83-98`,
  `:367-421`). Trader balances are 50-200 cr at advancement 0 and
  500-2,000 cr at 1 (`sector_ships.rs:93-95`). No restock code exists (search
  `restock` finds one test comment only).
- Extinct civilizations leave derelicts: the intact design with seeded
  breaches through about a fifth of its structural cubes
  (`ship_layout.rs:473-486`). Wreck stock is half the intact share
  (`sector_ships.rs:86-89`).

### 2.3 Ship identity and persistence

- `ShipDesign { sections: Vec<SpaceshipSectionConfig>, integrity,
  presentation }` (`ship_design.rs:77-90`). A section placement is `id`,
  `position`, `rotation`, `source` (`crates/nova_scenario/src/objects/spaceship.rs:403-414`).
- `ShipDesignSource` is `Inline(ShipDesign)` or `Prototype { id,
  section_patches }` (`ship_design.rs:303-321`). A spawn patch changes
  position, rotation and config; it cannot change a section's prototype id
  or add or remove a section (`spaceship.rs:418-440`, `:330-345`).
- `FrozenShip` stores the design source and a `FrozenShipState` with
  surviving sections keyed by section id, hold, health, marks and AI
  (`spaceship.rs:549-596`). The save stores the player as `SavedPlayer {
  ship: FrozenShip, .. }` (`crates/nova_world_base/src/save/mod.rs:115-129`).
- A destroyed section becomes a detached piece with art and physics only,
  no gameplay state (`crates/nova_ship/src/sections/frozen_piece.rs:1-9`),
  and despawns after 30 s (`crates/nova_gameplay/src/integrity/explode.rs:82-86`).
- `SectionKind` is closed: 9 kinds (`crates/nova_ship/src/sections/base_section.rs:442-462`).
  `BaseSectionConfig` has id, name, description, health, sounds, collider,
  link points, damage effects, animations; no mass, price or build cost
  field (`base_section.rs:345-412`). A mod adds sections through
  `Content::Section` (`crates/nova_modding/src/lib.rs:77-124`).
- Link points carry id, position and normal; the id is "not compatibility"
  (`crates/nova_ship/src/sections/link_points.rs:31-38`). Fit is by cell
  grid and sockets (`cell_grid_fit`, imported at `ship_parts.rs:33-35`).
- The editor saves a document as `Content::Ship` designs plus one scenario:
  "export my ship" is the saved file (`crates/nova_editor/src/bundle.rs:1-7`).

### 2.4 Docking, sectors and time

- One docking connection per ship; a second `DOCK` is refused
  (`crates/nova_ship/src/sections/docking_section/connection.rs:100-108`).
- Retirement explicitly refuses a docked ship (`crates/nova_world/src/streaming.rs:925-937`),
  but active-window ownership moves a docked pair into its current cell;
  this is not proof that an ordinary player-depot dock commonly retires.
  Docked-pair save/load currently restores both ships undocked.
- A visited sector freezes into `FrozenSector::Visited` and comes back as it
  was left (`streaming.rs:917-935`; `crates/nova_world/src/frozen.rs:266-274`).
  Body types: Asteroid, Ship, PendingShip, Canister, WreckFragment, OreDrop
  (`frozen.rs:386-400`). No player-placed body type.
- Generated body ids are `"{cell}_{name}_{index}"`
  (`crates/nova_world/src/generation.rs:509-511`). A severed wreck piece
  derives `<base>/wreck/<section>` from its parent, checked on save
  (`save/mod.rs:335-341`, `:418-428`); a Loop A strip can reuse that rule.
  Only canisters have a counter-minted id with a saved watermark
  (`save/mod.rs:110-112`); a depot would need the same.
- The save has no world clock (search `clock|elapsed|world_time` in
  `save/mod.rs` finds nothing). Frozen sectors do not simulate.
- `WORLD_SAVE_FORMAT = 1`; every cell retired from the live window becomes a
  visited record, not just cells changed by the player (`streaming.rs:954-959`).
  Unvisited cells regenerate; a generator change needs a format bump
  (`save/mod.rs:53-59`).
  Any catalog digest change refuses the save (RESEARCH.md 1.4).

## 3. External references (what each teaches Nova)

| Game | Verified mechanic | Source | Lesson for Nova |
|---|---|---|---|
| X4: Foundations | Buy a plot, plan modules only from owned blueprints, supply materials to build storage, a builder ship with drones builds; NPC builders charge 50k cr; a manager trades | [Egosoft manual: Station Building And Management](https://wiki.egosoft.com/X4%20Foundations%20Wiki/Manual%20and%20Guides/X4:%20Foundations%20Manual/Station%20Building%20And%20Management/) | Knowledge (blueprint) and materials are separate gates. Stations depend on a running economy and NPC traders. |
| Space Engineers | Blocks are made of components; a grinder disassembles blocks into reusable components; components come from an assembler or a grinder | [Grinder (tool)](https://spaceengineers.wiki.gg/wiki/Grinder_(tool)), [Component](https://spaceengineers.wiki.gg/wiki/Component) (official wiki per page title) | Salvage returns materials, not finished modules. Per-block component loss on damage: not confirmed from the fetched pages (**weak**). |
| Starsector | Colony costs 1,000 crew, 200 supplies, 100 heavy machinery; size 3 allows 1 industry, up to 4 at size 6; hazard multiplies upkeep; stability below 5 cuts income 20% per point | [Colony](https://starsector.wiki.gg/wiki/Colony) (community wiki); hazard and upkeep also in the developer blog [Colony Management](https://fractalsoftworks.com/2017/12/21/colony-management/) (fetched by the research subagent) | Stations need a recurring sink and a time model. |
| Starsector | Hullmods cost ordnance points; logistics hullmods change only while docked; recovered ships get d-mods removed by costly restoration in dock; damaged ships sell for little | [Hullmods](https://starsector.wiki.gg/wiki/Hullmods), [Ship recovery](https://starsector.wiki.gg/wiki/Ship_recovery) (community wiki) | Refit at a dock. Salvaged hardware arrives worn. A budget (OP) bounds builds. |
| Factorio | Labs consume science packs to research; technologies unlock recipes or give bonuses; some unlock by trigger | [Technologies](https://wiki.factorio.com/Technologies) | Unlocks are a sink. Trigger unlocks fit "salvage teaches a design". |
| Factorio | A recipe names ingredient and result items by name, a category limits machines, `enabled: false` until a technology unlocks it | [RecipePrototype](https://lua-api.factorio.com/latest/prototypes/RecipePrototype.html) | Item, recipe and unlock are separate prototypes. Nova should keep item, section, recipe and design separate. |
| Factorio | Mods run `data.lua`, then every `data-updates.lua`, then every `data-final-fixes.lua`; later mods may change earlier mods' prototypes without a dependency; the game records which mod changed which prototype | [Data lifecycle](https://lua-api.factorio.com/latest/auxiliary/data-lifecycle.html) | Factorio accepts unrelated overlays and records them. Plan D refuses them instead; that is a deliberate difference, not an oversight. |
| Avorion | Material tiers found nearer the core; each tier stronger; subsystem sockets need processing power and a per-tier building-knowledge cap | avorion.fandom.com [Materials](https://avorion.fandom.com/wiki/Materials), [Processing Power](https://avorion.fandom.com/wiki/Processing_Power); my fetch returned HTTP 402, the research subagent fetched them (**weak**, community wiki) | Same shape as Nova's advancement-by-distance gradient; a capacity budget bounds builds. |

## 4. Why the open world is not worth playing yet

1. The player starts at the top: the line warship and full magazines
   (`open_world.rs:72-101`). There is no better hull to reach.
2. Credits have no destination. Items are the only purchase, and the
   player's hold is small (100 kg per hull section, `ship_design.rs:393`).
3. Ore and salvaged parts exist only to be sold back at a fixed spread
   (`inventory.rs:83-112`).
4. Trader balances are finite and never restock (`sector_ships.rs:93-95`);
   visited cells stay frozen. New money comes only from new cells.
5. Nothing the player does leaves a mark beyond the hold, balance, emptied
   wrecks and carved asteroids.

A loop must therefore add: a weaker start or harder outer world, a credit
and material sink tied to ship power, and a persistent result the player
chose.

## 5. Three player loops

All three share the advancement gradient: better hardware is further from
the origin. They differ in what the player owns.

### Loop A: Salvager-refitter (recommended spine)

Fantasy: "I took that railgun off a dead Armored hull at the rim, and now it
is mine."

Goals:
- Early: replace a starting part (one turret, one thruster) with a better
  one from a nearby derelict. Learn that wrecks hold hardware.
- Mid: reach advancement-0.5 space; swap drives and controllers to fly a
  heavier build; add hull cubes for hold capacity.
- Late: a full hull swap at a yard to a design learned from a wreck, then
  outfit it from salvaged sections.

Current vs hypothetical:

| Step | Today | Needed (hypothetical) |
|---|---|---|
| Find hardware | Derelicts exist with real sections (`ship_layout.rs:473-486`) | Readout that names a wreck section's family and score |
| Take hardware | Credit take from lootable ships, item transfer | **Strip** a section from a derelict while docked: learn its design, gain materials |
| Pay | Credits for items | Section build cost in items plus credits |
| Install | No path: patches cannot swap a source (`spaceship.rs:418-440`) | Player ship stored as `Inline(ShipDesign)`; refit swaps one placement's `SectionSource::Prototype` id |
| Persist | `FrozenShip.design` and section states (`spaceship.rs:562-596`) | Same struct, but refit must replace the swapped section's saved state with a valid new record; deleting it marks the section destroyed. Known designs need new saved state. |

Player actions and feedback:
1. Lock a derelict, read its sections (family, score, condition).
2. Dock, choose Strip on one section. A timed action; feedback: progress,
   then the section detaches as art (existing detached piece path) and the
   player gets the design entry plus some material items.
3. At a yard (a docked intact Industrial trader, or a depot from Loop C),
   open Refit. Pick a placement, pick a known section of the same family
   and footprint. The panel shows cost, delta score, hold delta, thrust
   floor and controller count.
4. Confirm: credits and items leave, the section respawns. Refused plans
   change nothing (same rule as `plan_item_trade`).

Resource flow:

| Resource | Source | Sink |
|---|---|---|
| Section design (knowledge) | Strip from derelicts; buy at yards (**hypothetical**) | None: permanent unlock |
| Materials (`SalvagedParts`, plates, ore-derived goods) | Strip yield, mining, trade | Section build cost |
| Credits | Sales, credit take | Yard fee per refit |
| Removed old section | Refit | Yields a share of its build cost back, always less than the cost |

Persistence identity: the player ship id stays `SavedPlayer.id`. The design
becomes an owned value in the save, not a catalog reference. Each placement
keeps its `SectionId`, so key bindings keyed by section id stay valid
(`open_world.rs:107`). The known-design set is a sorted list of section ids
in the save. The catalog digest already refuses a save whose sections
changed (RESEARCH.md 1.4).

Module compatibility (recommended first rule): swap only within one family
and one cell footprint and socket set at the same placement. This keeps the
socket graph, colliders and lanes the generator already validates. Then run
the generator's own post-build checks (thrust floor, controller count, lanes,
connectivity, `ship_layout.rs:24-33`) on the result before it applies.
Free placement (add or remove cubes anywhere) is deferred breadth.

Mod seams:
- Section (exists): behavior, health, collider, sockets. A mod section joins
  generated ships today through `ShipPartSnapshot` and would appear on
  wrecks.
- Build cost (new field or new kind, **decision**): which items and how many
  build one section. Names item ids, so mod materials need Plan D.
- Ship design (exists): `Content::Ship` is the hull a yard can sell or a
  wreck can teach.
- Item (Plan D): the material, not necessarily the section. A whole section
  is a 10 m cell and a hold holds 100 kg per hull section, but sections have
  no authored mass (`base_section.rs:345-412`). Whether sections can be cargo
  is an open product rule, not a proven mass limit.

Failure and abuse:
- Strip-and-refit money loop: refund or strip yield >= build cost. Lint must
  refuse a section whose strip yield is worth more than its cost at the
  item asks.
- Self-stripping to strand: removing the last thruster or controller. Refit
  must refuse a plan that fails the thrust floor or controller count.
- Hold overflow: removing hull cubes below the current cargo mass. Refuse;
  never drop cargo.
- Refit in combat: require docked to a yard; the dock itself already ends
  free flight.
- Wreck farming: a stripped wreck freezes stripped (`streaming.rs:917-935`);
  new wrecks need new cells, which is exploration, not abuse.
- Unknown or removed mod section in a saved design: catalog digest refusal,
  never a substitute section.
- Bindings: a new weapon placement has no key. Decision: refit only swaps,
  so ids keep their bindings; added weapons need a binding rule later.

Minimum vertical slice:
1. Start ship is a weaker base hull (owner choice).
2. Player ship persists as an owned design.
3. Refit at a docked intact trader: same-family, same-footprint swap of one
   placement, paid in credits only, from a short list of base sections.
4. Save round trip keeps the swapped section.

Deferred breadth: strip yields, build costs in items, design learning,
hull purchase, free placement, worn hardware (Starsector d-mod analog),
binding rules for added weapons.

### Loop B: Prospector-fabricator

Fantasy: "My ore becomes my ammunition and my plating, and the rim ores make
the parts the core cannot."

Goals:
- Early: mine common ore and turn it into hull plates and PDC rounds aboard
  instead of buying them.
- Mid: carry a fabricator section; make rail slugs and torpedoes; supply
  Loop A build costs.
- Late: rare ores near the rim feed high-tier section costs.

Current vs hypothetical:

| Step | Today | Needed (hypothetical) |
|---|---|---|
| Ore | Four kinds map to four items (`mining.rs:151-157`); kinds are a Rust const (RESEARCH.md 1.3) | Asteroid kinds as content before mod ores |
| Convert | Not possible | Recipe content and a fabricator section kind (new `SectionKind` variant, closed enum) |
| Use | Ammo refill and repair consume fixed items (RESEARCH.md 1.2) | Same consumers, plus build costs |

Actions and feedback: open the fabricator pane, pick a recipe, queue counts;
progress per batch; output enters the hold or the queue stops with a named
refusal (no room, missing input).

Resource flow: ore (mining, wrecks) -> recipe -> plates, ammo, components ->
repair, refill, build cost. Credits only buy missing inputs.

Persistence: fabricator queue state on the section, like `SectionReload`
on ammo sections (`crates/nova_ship/src/sections/ammo.rs:189-215`).
Frozen with the ship. No world clock needed: work happens only near the
player.

Compatibility: a recipe names a fabricator category (Factorio `categories`
model). A section declares which categories it runs.

Mod seams: item (Plan D), recipe (new kind), fabricator section (new
`SectionKind` variant: code, not content). Asteroid kind -> ore binding
needs asteroid kinds as content (Stage 3 in RESEARCH.md 3).

Failure and abuse:
- Buy-craft-sell arbitrage with global prices: lint must refuse a recipe
  whose output bid exceeds its input ask total.
- Mass not conserved: recipes may change mass, but the hold check must run
  on the output before inputs leave.
- Infinite mining: asteroids are finite per cell and carved state freezes
  (RESEARCH.md 1.4 `FrozenAsteroid`).

Minimum slice: one base recipe (ore -> hull plate) on one base fabricator
section, run aboard. Proves item consumption and production without mods.

Deferred: recipe chains, categories, mod ores, fabricator upgrades.

### Loop C: Depot-keeper (stations, reduced)

Fantasy: "My depot at the edge of the belt holds my spare parts and refits
my ship. Later it makes things while I am away."

Goals:
- Early: none. A depot should not be the first goal.
- Mid: anchor one depot from a station design; store cargo beyond the hold;
  refit there (Loop A yard).
- Late: a second depot further out; a fabricator module (Loop B) that
  produces against a world clock.

Current vs hypothetical:

| Need | Today | Gap |
|---|---|---|
| Persistent player body | Sectors persist only generated bodies and canisters (`frozen.rs:386-400`) | New body type or a `FrozenShip` with no drive, plus a minted id like canisters (`save/mod.rs:110-112`) |
| Placement | Generated ids and positions are deterministic (`generation.rs:509-511`) | Overlap check against the cell's generated plan |
| Docking several ships | One connection per ship (`connection.rs:100-108`) | Player-only docking is enough for a depot |
| Sector retire while docked | Guard refuses retirement (`streaming.rs:931`); docked pairs currently remain active | Define and prove the intended static-depot docking and resume policy |
| Production over time | No world clock in the save | Clock field, catch-up rule on thaw |
| Traffic buying from it | No economy routing; traders are spawned stock | Deferred; X4-style managers need a running economy |

Actions and feedback: buy a station design kit at a yard; fly to a cell;
place (preview shows overlap and lanes); deliver build materials to its
build storage (X4 model); depot completes. Dock to transfer cargo or refit.

Resource flow: credits and materials -> depot build; hold <-> depot
storage; later inputs -> production -> outputs over clock time.

Persistence identity: depot id is minted (`depot_<n>`, **hypothetical**)
with a saved watermark. It lives in its cell's `Visited` record. Its design
is an owned value like the player ship.

Compatibility: station modules are sections; a station design is a
`Content::Ship` with no drive (decision: new kind or a role flag).

Mod seams: station design (ship design), module sections (existing kinds;
new behavior needs new `SectionKind` code), recipes (Loop B).

Failure and abuse:
- Storage exploit: unbounded depot storage removes hold pressure. Cap by
  hull cubes, like ships.
- Placement griefing generated content: refuse overlap with the cell's
  planned bodies and lanes.
- Offline production on wall-clock time: abuse by clock change. Use
  in-world time only.
- Save size: visited cells are already recorded; a depot adds persistent
  bodies and storage to its cell, rather than uniquely keeping it frozen.
- Generator change moves bodies under a depot: needs the format bump rule
  (`save/mod.rs:53-59`).

Minimum slice: one depot from one base design, player-only dock, storage
only. No production, no traffic.

Deferred: production, a clock, traffic, multiple docks, defense, upkeep
(Starsector-style sink).

## 6. Alternative models considered

| Model | Gives the player | Cost in Nova | Verdict |
|---|---|---|---|
| Contracts and reputation (deliver, bounty) | A reason to go somewhere | Needs mission state and a giver; civilizations exist to anchor reputation | Good motivation layer over Loop A; not a progression by itself |
| Exploration and lore caches (extinct civs) | Discovery, map completion | Extinct civilizations and derelicts exist (`sector_ships.rs:11-19`) | Strong pairing with Loop A: wrecks teach designs |
| Fleet and escorts | Command fantasy | AI escort orders, docking graph refused (`connection.rs:100-108`) | Deferred |
| X4 empire (stations, managers, traffic) | Economic power | Clock, economy sim, routing, many docks | Out of scope |
| Pure credits shop (buy better hulls) | Simple upgrade | Only a price table | Cheap first slice but no salvage identity; acceptable as Loop A step 3 |

## 7. Challenge: "space stations are easy"

A station that only looks like one is easy: a ship design with no drive.
Everything players expect a station to do is not:

1. **Nothing runs while you are away.** No world clock; frozen sectors do not
   simulate (section 2.4). Production needs a clock and a catch-up rule.
2. **One dock.** One connection per ship (`connection.rs:100-108`). A
   station with visitors needs a docking graph.
3. **Docked retirement is guarded** (`streaming.rs:931`), but a docked
   pair remains in the active window under current cell ownership.
   Whether a static depot changes that ordering needs a focused proof.
4. **No id for player bodies.** Generated ids come from the cell plan
   (`generation.rs:509-511`); only canisters mint ids.
5. **No traffic economy.** Traders are spawned stock with finite balances
   and no restock (`sector_ships.rs:93-95`). Nobody will buy a station's
   goods.
6. **Generator coupling.** A depot sits in a cell whose other bodies
   regenerate from the seed; a generator change needs a format bump
   (`save/mod.rs:53-59`).

So: depot first (storage and refit), production after a clock, traffic
last.

## 8. Seam boundaries for mods

| Thing | Owns | Kind | Exists | Depends on |
|---|---|---|---|---|
| Item | Mass, price, category, label | `Content::Item` | No (Plan D) | Plan D |
| Section | Behavior, health, collider, sockets | `Content::Section` | Yes | - |
| Build cost | Items to build one section | Field on section, or `Content::Recipe` | No | Plan D for mod items |
| Strip yield | Items a stripped section gives | Same owner as build cost | No | Build cost |
| Recipe | Items in -> items out, category | `Content::Recipe` | No | Plan D, fabricator kind |
| Ship design | Placements, integrity, look | `Content::Ship` | Yes (editor writes it, `bundle.rs:1-7`) | - |
| Station design | Ship design with no drive | `Content::Ship` plus flag, or new kind | No | Depot body |
| Section kind behavior | Code | Closed `SectionKind` | Yes, closed | Code change |

Rule: an item never carries behavior; a section never carries an economy
value except its build cost; a design never carries prices.

Illustrative RON (**hypothetical**, not generated, not approved):

```ron
// A base section with a build cost. Items are Plan D ids.
Section((
    base: (id: "turret_pdc_mk2", name: "PDC mount II", health: 400.0, ..),
    kind: Turret((..)),
    build: (
        credits: 300,
        items: {"SalvagedParts": 2, "HullPlate": 4},
        strip: {"SalvagedParts": 1},
    ),
)),

// A recipe for Loop B.
Recipe((
    id: "plate_from_iron",
    category: "fabricator",
    inputs: {"IronOre": 3},
    outputs: {"HullPlate": 1},
    seconds: 20.0,
)),
```

Lint cases (**hypothetical** messages):

```
recipe 'plate_from_iron': passes (output bid 30 cr <= input ask 3 x 16 = 48 cr)
recipe 'plate_from_carbon' (inputs {"CarbonOre": 3}): refused, output bid
    30 cr exceeds input ask 3 x 8 = 24 cr
section 'a': refused, strip yield at bid is worth more than its build cost at ask
recipe 'x': unknown item 'IronOar'
section 'y': build names category 'fabricator' that no section runs
```

## 9. Owner decisions (none made)

1. **Spine**: Loop A first, B second, C reduced to a depot later?
   Recommendation: yes.
2. **Start ship**: keep the line warship, or start in a weaker base hull
   (Industrial or Civilian)? Recommendation: weaker hull; the warship is a
   late goal.
3. **Ship ownership format**: store the player ship as
   `ShipDesignSource::Inline(ShipDesign)` in the save, or add a source swap
   to the spawn patch? Recommendation: Inline; patches stay patches.
4. **Refit rule**: same family and footprint swap only, or free placement?
   Recommendation: swap only, plus the generator's post-build checks.
5. **Where refit happens**: any docked intact trader, only Industrial
   traders, or only depots? Recommendation: Industrial traders first.
6. **Salvage result**: design knowledge plus materials (X4 and SE model), or
   the section as cargo? Recommendation: knowledge plus materials; a 10 m
   section cannot ride in a 100 kg-per-cube hold.
7. **Build cost owner**: a `build` field on `SectionConfig`, or a separate
   `Content::Recipe` that names a section? Recommendation: field first; it
   is one owner and one lint.
8. **Price of a section**: authored, or derived from the family score?
   Recommendation: authored; scores are provisional (`ship_parts.rs:25`).
9. **Money-loop invariants**: refuse strip or recipe value above cost at
   lint? Recommendation: yes, at lint then load.
10. **Plan D timing**: needed only when build costs or recipes name mod
    items. Recommendation: Loop A slice ships before Plan D; Plan D before
    item build costs.
11. **Depot body**: `FrozenShip` with no drive plus minted id, or a new
    body type? Recommendation: decide after Loop A lands.
12. **Docked-retire policy** for a depot: undock, hold the sector, or
    freeze the pair? Needed before any depot.
13. **World clock**: add one now (cheap field, no consumer) or with
    production? Recommendation: with production; no field without a
    consumer.
14. **Save format**: the ownership change alters `FrozenShip.design` use and
    `SavedPlayer`; saves are unshipped (RESEARCH.md 1.1), so no migration,
    but bump `WORLD_SAVE_FORMAT` once per layout change and keep the last
    good save refused, not rewritten.

## 10. Stages, dependencies and proof

```
S0 decisions (section 9)
 -> S1 weaker start + owned player design + credit-only swap refit
      at Industrial trader                       (no Plan D)
 -> S2 derelict strip: design knowledge + SalvagedParts
 -> S3 Plan D items (RESEARCH.md) -> build costs and strip yields in items
 -> S4 fabricator section + recipes (Loop B)     (needs S3; ore kinds as
                                                  content for mod ores)
 -> S5 depot body, minted id, docked-retire policy (Loop C storage + refit)
 -> S6 world clock + depot production            (needs S4, S5)
 -> S7 traffic and station trade                 (deferred)
```

Proof per stage (for later gates; not authorized):

- S1 pure: refit plan refuses wrong family, wrong footprint, thrust floor,
  controller count, hold below cargo, short credits; a refusal changes
  nothing (same pattern as `plan_item_trade`, `inventory.rs:492-534`).
- S1 ECS: dock to an Industrial trader, swap one turret, freeze and thaw
  the player, assert the new prototype id, fresh health, matching ammo/reload
  state and old bindings on that section id. A missing frozen section means
  destroyed, so the swap must never delete its saved entry.
- S1 save: resume round trip with the swapped design
  (`examples/systems/system_world_resume.rs`); then remove the section from
  the catalog and assert `WorldRefusal::Catalog` with no file change.
- S2: strip moves a wreck section to a detached piece, adds the design id
  once, adds materials, and the frozen wreck stays stripped after
  retire and return.
- S3/S4 lint: money-loop refusal, unknown item, unknown category.
- Player flow (`nova-bench`): from a weak start, reach and complete one
  refit; judge the observation log for the section swap, not exit code.
  Never assert time to complete.
- S5: retire and return with a depot in the cell; depot storage and id
  survive save; placement overlap refusal.

## 11. Sources

- Egosoft, X4 manual, Station Building And Management:
  https://wiki.egosoft.com/X4%20Foundations%20Wiki/Manual%20and%20Guides/X4:%20Foundations%20Manual/Station%20Building%20And%20Management/
- Space Engineers wiki (official per page title), Grinder (tool):
  https://spaceengineers.wiki.gg/wiki/Grinder_(tool)
- Space Engineers wiki, Component: https://spaceengineers.wiki.gg/wiki/Component
- Starsector wiki (community), Colony: https://starsector.wiki.gg/wiki/Colony
- Starsector wiki (community), Hullmods: https://starsector.wiki.gg/wiki/Hullmods
- Starsector wiki (community), Ship recovery: https://starsector.wiki.gg/wiki/Ship_recovery
- Factorio wiki, Technologies: https://wiki.factorio.com/Technologies
- Factorio API, RecipePrototype: https://lua-api.factorio.com/latest/prototypes/RecipePrototype.html
- Factorio API, Data lifecycle: https://lua-api.factorio.com/latest/auxiliary/data-lifecycle.html
- Starsector developer blog, Colony Management: https://fractalsoftworks.com/2017/12/21/colony-management/
- Avorion wiki (community, **weak**), Materials and Processing Power:
  https://avorion.fandom.com/wiki/Materials, https://avorion.fandom.com/wiki/Processing_Power
