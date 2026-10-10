# RON-authored items and open-world identity: research

Research only, on master `467ae965b` (2026-10-10). No code, content, test,
Cargo, probe or GPU run. The owner favors Plan D as the research direction;
no implementation, ID spelling, overlay, migration, or progression policy is approved. Evidence is `path:line`
on master. **Hypothetical** marks proposed code or RON that does not exist.
**Unverified** marks a claim that was not traced to a call site.

## 1. Verified current state

### 1.1 The item type is a closed Rust enum

- `ItemType` is a `Copy` enum with 10 variants: `HullPlate`, `PdcRound`,
  `RailSlug`, `Torpedo`, `StoneOre`, `IronOre`, `WaterIce`, `CarbonOre`,
  `Rations`, `SalvagedParts` (`crates/nova_gameplay/src/inventory.rs:39-64`).
  Serde is derived with no rename, so RON writes the bare variant name.
- Every attribute is a `match` in code: `mass_g` (`inventory.rs:69-78`),
  `ask_cr` (`:83-95`), `bid_cr` (`:100-112`), `category` (`:123-131`),
  `label` (`:137-149`). Prices are global per item, not per trader.
- `ItemCategoryType` is closed: `Raw`, `Repair`, `Ammo`, `Food`, `Parts`
  (`inventory.rs:168-179`). Icons and colors key on category only
  (`crates/nova_interface/src/icons.rs:343-358`, `:366-`;
  `crates/nova_interface/src/inventory/app.rs:236-254`).
- The spelling is a shipped authored format. v0.15.0 ships the same enum
  (`git show v0.15.0:crates/nova_gameplay/src/inventory.rs`), and live
  webmods author it: `inventory: {PdcRound: 2000}`
  (`webmods/the-ledger/ledger_01_drift_run.content.ron:36`, and five other
  ledger files). Base: `assets/base/scenarios/open_world.content.ron:78-83`.
- The save format is not shipped. v0.15.0 has no
  `crates/nova_world_base/src/save/mod.rs`; saves landed in `b210fee0c` under
  `[Unreleased]`.

### 1.2 Consumers (compile worklist if the enum becomes an open ID)

| Consumer | Evidence | Item binding |
|---|---|---|
| Authored stock | `ShipInventoryStock` `inventory.rs:191-193`, custom serde `:831-860` | map key |
| Runtime hold | `ShipInventory` `inventory.rs:246-251` (derived `Deserialize`, no capacity check; `new` checks at `:255-261`) | map key |
| Repair | `plan_plate_repair` `inventory.rs:793-825`; UI `crates/nova_interface/src/ship/sections.rs:494-497` | hard `HullPlate` |
| Transfer | `plan_item_transfer` `inventory.rs:400-428` | generic |
| Trade | `plan_item_trade` `inventory.rs:478-534`; any docked partner that is not neutralized or lootable trades (`:483-486`) | generic, global prices |
| Jettison | `plan_item_jettison` `inventory.rs:715-753`; queue `crates/nova_ship/src/sections/cargo_intake_section.rs:145`, drain `:503-617` | generic |
| Canister | `CargoCanister` `inventory.rs:578-628`, 200 kg cap `:574` | map key |
| Pickup | `run_cargo_intakes` `cargo_intake_section.rs:482` | generic |
| Mining | `ore_for_asteroid_kind` `crates/nova_scenario/src/mining.rs:151-157`; spawn `:891` | hard kind -> item |
| Ammo refill | `SectionReload { item: ItemType, .. }` is `Copy` and serialized (`crates/nova_ship/src/sections/ammo.rs:189-215`) | hard per weapon: turret `turret_section/setup.rs:175`, rail `railgun_section/firing.rs:34`, torpedo `torpedo_section/bay.rs:171` |
| Generated stock | role mixes `crates/nova_world_base/src/sector_ships.rs:392-421`, draw `:453-465` | hard lists |
| UI text | `item_about` `crates/nova_interface/src/inventory/app.rs:220-233` | exhaustive match |
| Command shell | `ITEM_WORDS` `crates/nova_command/src/commands.rs:147-161` (no gameplay dependency) | parallel list |
| Cheats | parse `crates/nova_console/src/cheats.rs:199-211`; test pin `:366-380` | parallel list |
| Bench | `crates/nova_bench/src/observation.rs:1003` writes `"item": "HullPlate"` | spelling |
| Base builder | `crates/nova_authoring/src/base_content/scenarios/open_world.rs:92-97` | variant literals |
| Editor | new ships get `ShipInventoryStock::new([])` (`crates/nova_editor/src/scenario.rs:712`, `:819`, `:911`, `:943`); no stock editing found | empty stock |
| Docs | `web/src/create/objects.md`, `web/src/create/sections.md`, `web/src/wiki/commands.md`, `docs/sections.md` name item spellings | spelling |

Exhaustive `match` sites on `ItemType`: 7 (5 methods above, `item_about`,
the cheats test). Full-list sites: `ITEM_WORDS`, the cheats parser. Hard
item choices in runtime code: mining (4 kinds), 3 weapon bindings, repair.
No destruction drop spawns canisters: the only non-test `CargoCanister::new`
calls are jettison (`inventory.rs:749`) and mining (`mining.rs:891`).

### 1.3 Content registries and mod merge

- `Content` has 7 kinds: `Section`, `Scenario`, `Campaign`, `Style`, `Ship`,
  `Lesson`, `UiTheme` (`crates/nova_modding/src/lib.rs:77-124`). No item kind.
- `ShipDesignId(String)` is transparent serde, a flat id
  (`crates/nova_scenario/src/objects/ship_design.rs:39-46`). It resolves
  against RON content (`Content::Ship`) through
  `GameShipDesigns::get_design(&self, &ShipDesignId)`, a linear `Vec` find
  (`ship_design.rs:352-359`). This is the working pattern.
- `AsteroidKindId(String)` is open in type only. Its table is a Rust const,
  `ASTEROID_KINDS: [&str; 5]`
  (`crates/nova_scenario/src/objects/asteroid_kind.rs:270`). A mod cannot add
  a kind today. Do not copy this as "RON-authored".
- Order: base, then shipped catalog order, then downloaded order, then a
  dependency topological sort with that order as tie-break
  (`crates/nova_assets/src/merge.rs:111-151`, `:279-304`).
- Policy: the same id twice in one bundle keeps the first and logs a
  conflict (`merge.rs:873-882`). The same id in another bundle replaces the
  earlier one silently (`merge.rs:850-852`, `:897-941`). Two unrelated mods
  that define one id: the later in merge order wins, with no diagnostic.
- Ids are one flat space per kind. No content id carries a mod namespace;
  `self://` and `dep://` apply only to resource refs (`crates/nova_assets/src/mod_refs.rs:94`, `:263`).
- A missing required field is a RON decode error at asset load
  (`crates/nova_modding/src/lib.rs:234-241`). Base section or ship errors are
  fatal; a mod's errors quarantine the mod (`merge.rs:184-268`).
- `ContentCatalogDigest` is FNV-64 over sorted mod metadata and every
  effective item with the mod that won it (`merge.rs:670-783`). It does not
  depend on merge order, but it does change when a different mod wins an id.
- Generator: `cargo run content gen` writes base RON from Rust builders;
  `cargo run content lint` checks ids, refs and duplicates
  (`crates/nova_authoring/src/cli.rs:57-79`).

### 1.4 Persistence

- Header `WorldSaveHeader { format, name, seed, catalog: u64, mods, .. }`
  (`crates/nova_world_base/src/save/mod.rs:71-94`). `WORLD_SAVE_FORMAT = 1`
  (`:59`). RON files `world.ron` and `state.<gen>.ron` (`:1-21`). Native only.
- Load refuses any digest change: `WorldRefusal::Catalog` with text "content
  changed; saved with <mods>" (`mod.rs:180-210`, `:904-922`). A missing mod
  changes the digest, so it refuses before state parse. The text lists the
  saved mods; it does not name which one is missing.
- Items in the state: player `FrozenShip.credits` and
  `FrozenShipState.inventory: ShipInventory`
  (`crates/nova_scenario/src/objects/spaceship.rs:559-591`); queued jettison
  `FrozenSection.ejection_queue` (`crates/nova_ship/src/sections/frozen.rs:86`, `:227`);
  canisters `FrozenBodyType::Canister` (`crates/nova_world/src/frozen.rs:386-400`);
  mined ore and mined queues on `FrozenAsteroid`
  (`crates/nova_scenario/src/objects/asteroid.rs:439-456`); reload item in
  `SectionReload` (`ammo.rs:189-201`). All keys are bare variant names.
- Today, an unknown item name in a state file is an unknown enum variant, so
  `ron::from_str` fails and the world lists `Unreadable` (`mod.rs:299-302`).
  With open string-backed IDs this check disappears. Add a post-parse
  saved-item lookup over stock, queued cargo, reloads, canisters and mined
  records, at Load and before writing; fail visibly and preserve the last
  good save. A matching catalog digest is not a substitute for validation of
  a malformed or manually edited state file.
- Ids: sector bodies `"{cell}_{name}_{index}"` (`crates/nova_world/src/generation.rs:509-511`);
  canisters by a process counter whose watermark the save stores
  (`inventory.rs:634-659`, `WorldSaveState.canister_ids_next` `mod.rs:110-112`).
  None of these depend on item ids.
- Save ids are checked on open and on save: duplicate body ids, bad wreck
  ids, duplicate or unminted canister ids (`save/mod.rs:335-495`).
- Gap today: item mass and price are code, not content, so the digest does
  not cover them. A build that changes `mass_g` loads an old save, and the
  derived `ShipInventory` deserialize does not recheck capacity
  (**unverified** whether a later system catches an overfull hold).

## 2. Plans compared

All plans keep: typed id in the owning crate, RON authored by Rust builders
for base, explicit fields, no default item, unknown id fails at lint then
load, digest-pinned saves.

### Plan A: closed ids, RON attributes

`ItemType` stays the closed id. A new `Content::Item` carries mass, prices,
category, name and about text for each variant. Base must define all 10;
a mod may overlay attributes but cannot add an item.

- Scope: inventory attribute methods, `item_about`, merge, lint, builders.
  Consumers keep `Copy` and `match`.
- Complexity: low. No consumer signature change except attribute lookups.
- Failure modes: a missing base definition must be fatal; overlay is the
  existing silent last-wins.
- Migration: none for content or webmods. Save format unchanged.
- Gain: balance values move into content and into the digest (closes the
  1.4 gap). Mod-added items: none. Fails the owner's mod-item goal.

### Plan B: open authored catalog, flat ids (mirror `ShipDesignId`)

`ItemDesignId(String)` (name per AGENTS.md "authored catalog ID"; owner
decision) replaces `ItemType`. `Content::Item` adds or overlays an item by
id with the existing last-wins rule. Engine roles (repair plate, three
ammo, four ores) stay code constants that name base ids, like `KIND_ROCK`;
lint and load require those ids to exist with the right category.

- Scope: every row in 1.2. `ItemType` loses `Copy`; `SectionReload` loses
  `Copy`; pure `plan_*` functions take an item table argument.
- Complexity: medium-high. 46 files reference `ItemType` (crates and
  examples, tests included).
- Failure modes: two unrelated mods defining one id resolve by merge order,
  so stock values depend on load order. This breaks the task's invariance
  check unless lint refuses it.
- Migration: ids can keep the shipped spellings (`"HullPlate"`), but map keys
  change from bare `{PdcRound: 2000}` to quoted `{"PdcRound": 2000}`. The
  workspace `ron` is 0.12.2 (`Cargo.lock:6233-6234`); its `Parser::string`
  accepts only `"` or `r` and otherwise returns `ExpectedString`
  (`ron-0.12.2/src/parse.rs:1163-1169`). A `String`-keyed map rejects bare
  keys, so this is **(breaking)**: migrate base builders, the example mod,
  the six ledger files with nonempty stock. Gauntlet's stock maps are empty
  and do not need a key migration. Only a custom key deserializer could keep
  bare keys; that is an adapter for a shipped spelling, not recommended.
- Gain: mod items exist and flow through every generic consumer. Mod items
  cannot be ammo, repair or ore until a later authored binding.

### Plan C: open catalog, owner-namespaced ids, refused collisions

As B, plus: a mod item id must be `<mod_id>/<name>` and only its pack may
define it; base ids stay bare. Another pack may overlay an id only if it
depends on the owner (direct or transitive). Any other collision is an error
at lint, and a quarantine at load.

- Scope: B plus a namespace rule in lint and in `merge_bundles` for items.
- Complexity: high. Item merge policy differs from the other six kinds
  unless the rule extends to all (larger blast radius).
- Failure modes: an author who forgets the prefix is refused at lint;
  a valid overlay without a dependency is refused.
- Migration: same as B; base ids unchanged.
- Gain: ids and effective stock are order-independent by construction; a
  missing-mod refusal can name the owner from the id prefix.

### Plan D: staged B with C's collision rule (owner-favored research direction)

Ship B's open catalog, but for items only refuse a same-id definition from
two packs when neither depends on the other. Do not require a prefix; record
the prefix as an author convention in docs. This gives order invariance at
C's guarantee with B's naming freedom, and keeps the other kinds unchanged.
Current cross-pack last-wins has no diagnostic; unlike an intra-pack duplicate,
unrelated item conflicts must be rejected before merge, with a named pack/error.
`nova_mod_format::deps::transitive_deps` (`deps.rs:54-80`) can test overlay
ownership. Its topological sort preserves input order among ready packs, not
lexicographic order (`deps.rs:94-101,131-154`). Dependency cycles and
same-pack duplicate definitions need an explicit fail-closed rule. Which
pack is quarantined on an unrelated clash must not change with load order.

### Decision matrix

| | A closed+attrs | B flat open | C namespaced | D staged B+rule |
|---|---|---|---|---|
| Mod adds item | no | yes | yes | yes |
| Load-order invariant stock | yes | no | yes | yes |
| Content format break | none | yes (quoted keys) | yes | yes |
| Save format change | none | key type only | key type only | key type only |
| Consumer churn | low | high | high | high |
| New merge policy | none | none | items + prefix | items only |
| Mod ammo/ore/repair | no | later | later | later |
| Closes mass/price digest gap | yes | yes | yes | yes |

## 3. Recommendation and staging

Player experience first: the base loop (inventory -> canister -> market ->
save) works today with the closed enum. Keep this task last, as the owner
asked.

1. Stage 0 (no code): owner answers section 6.
2. Stage 1: Plan A data move only if the owner wants balance in content
   before v0.16.0. It is a strict subset of D and its `Content::Item` shape
   survives.
3. Stage 2: Plan D. Open id, generic consumers, engine-role constants,
   collision rule, docs, changelog `**(breaking)**` if keys change. Validate
   every saved item ID after parse and before save, even with digest pins.
4. Stage 3 (separate gate): authored bindings so mod items can be ammo
   (`ammo_item` on weapon sections), ore (per asteroid kind; needs kinds as
   content first), repair, and generated trader stock.

## 4. Illustrative RON (hypothetical, not production or generated)

Base, generated from a builder (hypothetical `Content::Item`):

```ron
Item((
    id: "HullPlate",
    name: "Hull plate",
    about: "Bolted over a damaged section. One plate restores 20 health.",
    category: Repair,
    mass_g: 10000,
    ask_cr: 40,
    bid_cr: 30,
)),
```

Mod adds an item and stocks it (hypothetical):

```ron
// mods/deep-salvage/deep-salvage.content.ron
Item((
    id: "deep-salvage/reactor_core",
    name: "Reactor core",
    about: "A sealed core pulled from a derelict. Traders pay well.",
    category: Parts,
    mass_g: 80000,
    ask_cr: 900,
    bid_cr: 650,
)),
// in a spawned ship:
inventory: {"deep-salvage/reactor_core": 1, "PdcRound": 500},
```

Invalid cases and the expected result (hypothetical messages):

```ron
// Missing required field: RON decode error at asset load; lint fails first.
Item((id: "deep-salvage/coolant", name: "Coolant", category: Raw, mass_g: 5000)),
// -> missing field `about` (and `ask_cr`, `bid_cr`)

// Unknown id in stock: lint error, then scenario load refusal.
inventory: {"deep-salvage/reactr_core": 1},
// -> ship 'drifter_1': unknown item 'deep-salvage/reactr_core'

// Zero count or over-mass: existing checks (inventory.rs:191-233, lint overstock).
inventory: {"PdcRound": 0},

// Conflict (Plan C/D): two unrelated packs define one id.
// pack "deep-salvage": Item((id: "reactor_core", ..))
// pack "ore-rush":     Item((id: "reactor_core", ..))
// -> item 'reactor_core' is defined by 'deep-salvage' and 'ore-rush', and
//    neither depends on the other

// Engine role missing or wrong category (B/C/D): base content is fatal.
// pack "base" overlays nothing named "PdcRound"
// -> turret ammunition item 'PdcRound' is not defined
```

Save header after a mod item is stocked (existing fields, real shape):

```ron
(format: 1, name: "Rust Belt", seed: 42, catalog: 1234567890,
 mods: ["base", "deep-salvage"], ..)
```

Load without `deep-salvage`: digest differs, `WorldRefusal::Catalog`,
"content changed; saved with base, deep-salvage" (`mod.rs:209-210`).
The world is never opened with the item removed or replaced.

## 5. Proposed interfaces and call graph (hypothetical, for review)

Owning crate: `nova_gameplay` (lowest crate that `nova_ship`,
`nova_scenario`, `nova_interface`, `nova_world*` share).

```rust
// nova_gameplay::inventory (hypothetical)
pub struct ItemDesignId(Arc<str>);          // Clone, Ord, Hash, transparent serde
pub struct ItemDesign { id, name, about, category: ItemCategoryType,
                        mass_g: u32, ask_cr: u32, bid_cr: u32 }
#[derive(Resource)] pub struct GameItems { by_id: BTreeMap<ItemDesignId, ItemDesign> }
impl GameItems { pub fn get(&self, id: &ItemDesignId) -> Option<&ItemDesign>; }
pub fn plan_item_trade(items: &GameItems, trade, partner_trades,
                       item: &ItemDesignId, quantity, own, own_cr,
                       partner, partner_cr) -> Result<ItemTrade, ItemTradeRefusalType>;
// + ItemTradeRefusalType::UnknownItem? (decision: unknown id should be
//   impossible after load; a refusal variant may hide a bug)
```

`Arc<str>` keeps clones cheap without a numeric index. A numeric index
would depend on merge order and must never be saved.

Before:

```
RON {PdcRound: n} -> ShipInventoryStock -> ShipInventory
plan_* -> ItemType::mass_g/ask_cr/bid_cr (code match)
mining -> ore_for_asteroid_kind -> ItemType::StoneOre..
turret setup -> SectionReload::from_config(_, ItemType::PdcRound)
save -> FrozenShip/CargoCanister/SectionReload -> "PdcRound"
```

After (Plan D):

```
bundles -> merge_bundles -> Content::Item -> collision rule -> GameItems
       \-> content_catalog_digest (covers items)
RON {"PdcRound": n} -> ShipInventoryStock(ItemDesignId) --lint/load: GameItems.get
plan_*(&GameItems, ..) -> ItemDesign.mass_g/ask_cr/bid_cr
mining -> ore_for_asteroid_kind -> ItemDesignId const (checked at load)
turret setup -> SectionReload::from_config(_, ItemDesignId const)
save -> same structs, keys are ids; Load -> digest gate (unchanged)
UI -> ItemDesign.name/about; ITEM_WORDS -> runtime list from GameItems
```

Docs to ship with Stage 2: `web/src/create/mod-files.md`,
`web/src/create/base-content.md`, `web/src/create/objects.md`,
`web/src/create/sections.md`, `web/src/wiki/commands.md`,
`docs/sections.md`, example mod README; changelog under
`### Modding & Mod Portal` (`CHANGELOG.md:34`).

## 6. Owner decisions (none made)

1. Plan: owner favors D for research. This is not approval to implement D or
   to ship A first; ask whether an A-like preliminary attributes move helps.
2. Id name and shape: `ItemDesignId` (AGENTS.md catalog-id rule) or
   `ItemKindId`; keep `"HullPlate"` spellings or move to snake case. Keys
   become quoted in any open-id plan, so the format breaks either way.
3. Collision policy: silent last-wins (today), refuse unrelated duplicates
   (D), or mandatory prefix (C).
4. Can a mod overlay a base item's mass or price? Overlay changes hold
   arithmetic for every ship that carries it.
5. Engine roles: may a mod delete or recategorize `HullPlate`, `PdcRound`,
   `RailSlug`, `Torpedo` or the four ores? Recommendation: no; fatal.
6. Do mod items enter generated traders, loot and mining in Stage 2, or only
   through authored stock until Stage 3?
7. Category: stays a closed `ItemCategoryType` (icons need it), or opens.
8. Missing-mod text: keep "content changed; saved with ...", or name the
   missing and extra mods (needs the save to record mod versions).
9. Balance patches: digest pinning refuses every save after any content
   change, items included. This is a persistence-task decision, but items
   in content make it more frequent.

## 7. Proof plan (for a later gate; not authorized)

- Pure: catalog build from two packs in both orders gives equal
  `GameItems` and equal digest; unrelated duplicate refuses with both pack
  ids; missing field and unknown stock id fail lint; engine-role item absent
  is fatal for base.
- ECS: a mod item moves player hold -> jettison queue -> canister ->
  pickup -> trader Sell -> Buy back, with grams and count conserved at each
  step (extend the existing `nova_world/src/tests/frozen.rs:783` and
  `inventory.rs` patterns rather than adding a parallel harness).
- Save: `examples/systems/system_world_resume.rs` round trip with the
  example mod enabled and one mod item in hold, canister and ejection queue;
  then load with the mod disabled and assert `WorldRefusal::Catalog` and
  that no save file changed.
- Content: regenerate base RON, lint base and every webmod, assert the
  example mod's README example parses.
