# Open-world purpose and progression: design B

Research and plan only. Master `467ae965b` (2026-10-10). No code, content,
test, Cargo, probe or GPU run. Nothing here is approved. It is a companion
to [RESEARCH.md](RESEARCH.md) (item identity, Plan D). It does not replace it.

Labels: **Hypothetical** marks proposed code, RON or numbers. **Unverified**
marks a claim that was not traced to a call site or a fetched page.
**Inference** marks a conclusion drawn from verified facts but not observed
in play.

Short answer: give the player a *reason to go somewhere* before you give them
a *thing to buy*. The spine I recommend is a **charter frontier** (loop 1):
authored and seeded goals that send the player outward along the existing
civilization advancement gradient. Below it sit a **restoration** sink for
the ship (loop 2) and **regional demand** for the economy (loop 4).
**Infrastructure** (loop 3) is the long-term goal and starts without
stations. A shipyard comes last, if at all.

## 1. Constraints this design accepts

| Constraint | Source |
|---|---|
| Plan D is the favored research direction, not an approved item schema or implementation. | User, this request; [RESEARCH.md](RESEARCH.md) section 3 |
| RON items are last in v0.16.0. Implementation stays deferred. | `TASK.md` User facts |
| Crafting is not a goal. Trading raw for processed goods can fill its role. | `tasks/20261007-085535/TASK.md` User facts |
| The owner deleted the ship-tuning spike, the in-game ship editor task and the reputation/diplomacy task. | Same file; deleted files `tasks/20260925-190219`, `20260930-100908`, `20261001-175224` (`git show d6b6c8271^:tasks/<id>/TASK.md`) |
| Stations are not assumed easy. | User, this request |
| Closed vocabulary. No embedded scripting. No save compatibility between versions. Save points, not saving anywhere. | `tasks/20260824-125938/RESEARCH.md` "Decisions (owner, 2026-09-06)" |
| The save format has not shipped. Bare item keys in content have shipped. | [RESEARCH.md](RESEARCH.md) 1.1, 1.4 |
| Explicit authoring, known ids, fail at lint then load, no silent defaults. | `AGENTS.md` Change policy |

## 2. Verified current state: why the world has no purpose

### 2.1 What the player can do

| Fact | Evidence |
|---|---|
| The open-world scenario has no objective. Its only outcome is a Defeat when the player dies. | `crates/nova_authoring/src/base_content/scenarios/open_world.rs:1-9` |
| Start: `block_line_warship`, 2000 cr, 12 plates, 6000 PDC rounds, 20 slugs, 12 torpedoes. | `assets/base/scenarios/open_world.content.ron:31`, `:78-85` |
| Prices are global and fixed per item. Ask is greater than bid, so Buy then Sell at one trader loses credits. | `crates/nova_gameplay/src/inventory.rs:83-112` |
| Trade only with a docked intact ship that trades. Take only from a lootable (derelict or neutralized) ship. Robbing takes all of a lootable ship's credits. | `inventory.rs:478-534`, `:400-428`; `plan_credit_take` `inventory.rs:548-571` |
| Generated ships start with finite credits: 50-200 cr at low advancement, 500-2000 cr at high. Wrecks hold a share. | `crates/nova_world_base/src/sector_ships.rs:85-95`, `:468-480` |
| Generated stock depends on role (closed lists) and advancement. | `sector_ships.rs:392-465` |
| Hold capacity is 100 kg for each hull section. | `crates/nova_scenario/src/objects/ship_design.rs:392-402` |
| A canister holds at most 200 kg. | `inventory.rs:574` |
| Plate repair refuses a destroyed or disabled section. | `plan_plate_repair` `inventory.rs:793-825`, `Destroyed` check near `:803` |
| A saved or frozen ship never spawns a destroyed section again. | `crates/nova_scenario/src/objects/spaceship.rs:578-581` |
| There is no refit, ship purchase or section replacement anywhere. | ow-runtime scout grep; deleted task `20260930-100908` |
| Docking is ship to ship only: "There is no station service here". | `crates/nova_ship/src/sections/docking_section/mod.rs:12-16` |
| Clusters are `AsteroidRich`, `RockOnly`, `PlanetHeavy`, `DerelictField`. No station, outpost or settlement. | `crates/nova_world_base/src/clusters.rs:244-254` |

What credits can buy today: ammunition and hull plates. Nothing else.
Torpedoes (400 cr) are the largest purchase. **Inference:** after the first
hour, credits pile up with no use, and the player has no reason to pick one
direction over another.

### 2.2 Hooks a progression system can reuse

| Hook | Evidence | Use |
|---|---|---|
| Civilizations on a 240 km lattice. Each has a status (Living/Extinct), an allegiance (1/3 each Enemy/Player/Neutral) and an advancement that rises with distance from the origin, saturating at 1000 km. | `crates/nova_world_base/src/civilizations.rs:25-57`, `:82-132` | A free difficulty and reward gradient: the frontier is *outward*. |
| Three noise fields per position: material density, volatiles, human activity. | `crates/nova_world_base/src/environment.rs:101-182` | Regional supply, demand and survey value. |
| Sector edge 32 km; pure, order-independent generation. | `crates/nova_world_base/src/lib.rs:139`; `crates/nova_world/src/generation.rs:813-853` | Deterministic goal placement from seed + cell. |
| Retired sectors freeze as `FrozenSector::Visited` and come back as left. | `crates/nova_world/src/frozen.rs:266-275`; `crates/nova_world/src/streaming.rs:914-1008` | Depletion is durable. "Visited" already exists for each cell. |
| Native saves write on each sector crossing and on leaving. The web build has no saved worlds. | `crates/nova_world_base/src/save/session.rs:1-10`; `crates/nova_world_base/src/lib.rs:27-36` | A progression record has a commit point. Web progression is session-only. |
| `WorldSaveState { format, generation, player, sectors, canister_ids_next, transients }`, `deny_unknown_fields`, `WORLD_SAVE_FORMAT = 1`. | `crates/nova_world_base/src/save/mod.rs:54-59`, `:96-114` | The owner of a new `progress` field. |
| Load refuses any catalog digest change. | `save/mod.rs:180-210` | Progression content is pinned with the world. |
| Event triggers `OnDocked`, `OnEnter`, `OnDestroyed`, `OnNeutralized`, `OnShipOrderComplete`. | `crates/nova_scenario/src/events.rs:81-140` | Triggers exist, but generated bodies are not scenario-addressable (`tasks/20261007-085535/TASK.md` findings on `streaming.rs:165-175`). |
| `Objective`, `ObjectiveComplete`, `ObjectiveMarkerAttach/Detach` actions; HUD stack and marker chips. | `crates/nova_scenario/src/actions/mission.rs:19`, `:212`, `:231`, `:285`; `crates/nova_gameplay/src/objectives.rs:42-45`; `crates/nova_hud/src/objective_stack.rs`, `objective_markers.rs` | Goal display exists. It is not saved: `WorldSaveState` has no objectives or variables. |
| Section patches: health; thruster `magnitude`; controller `steering_lag`, `max_torque`; turret `muzzle_speed`, `projectile_lifetime`, damage, muzzle `fire_rate`; torpedo, railgun, docking fields. | `crates/nova_ship/src/sections/patch.rs:83-112`, `:155-176`, `:296-345`, `:434`, `:521`, `:603` | A bounded, already-validated "upgrade" vocabulary. |
| `FrozenShip` saves `design: ShipDesignSource` (catalog id plus `section_patches`) and the surviving sections by id. | `spaceship.rs:562-593`; `crates/nova_scenario/src/objects/ship_design.rs:303-319` | Upgrades as patches and hull swaps as design ids already fit the save. Restoring a section does not. |
| Cheats: `item_give`, ammo refill and infinite ammo after `enable`. | `crates/nova_console/src/cheats.rs:35-139` | Progression must decide how cheats affect completion. |

## 3. External comparisons (fetched sources only)

The scout fetched each URL and I fetched Factorio myself. Claims that came
only from search snippets are excluded. Avorion is excluded because no
official page was fetched.

| Game | What the player builds or accomplishes | Ship improvement | Sink | Goal display | Lesson for Nova | Source |
|---|---|---|---|---|---|---|
| X4: Foundations | Stations on a paid plot. Builder ships deliver wares to a build storage; modules come from blueprints. | Empire scale, not the player hull. | Plot tax (1000-7000 cr/km3, waived in unowned space), delivered wares, basic station about 2-3M cr. | Plot tab, construction queue. | A legible pipeline (blueprint -> plot -> deliveries -> ordered build) is the right shape for a late Nova structure. It needs builder logistics that Nova lacks. | Egosoft wiki, "Station Building And Management" (official) |
| EVE Online Equinox | Upgrades on a sovereignty hub pick ore types and site value. | Indirect. | None (production upgrade). | Not described. | CCP's own retrospective: more nominal value with more labour per unit lowered participation; ratting paid better, so miners left. Measure time-to-canister before tuning yields. | eveonline.com "Equinox mining balance, philosophy and learnings" (official) |
| Factorio | Production chains that make the next science pack. | n/a | Infinite research with geometric cost: the cost to reach level N is about the cost of level N+1. | Tech tree with pack costs. Trigger technologies unlock by doing (mine X, craft Y), with no packs. | Finite unlocks plus one named infinite sink. Trigger unlocks are the cheapest goal type: they reward an action the player already does. | wiki.factorio.com/Technologies (official) |
| Starsector | Colonies on surveyed planets: 1000 crew, 200 supplies, 100 heavy machinery. Industries up to 4 at size 6. | Indirect (funds the fleet). | Upkeep times a hazard rating; -2 stability per extra colony past 2. | Story points buy permanent improvements. | Survey first, then claim. Hazard as a *multiplier* on upkeep scales difficulty with location. Anti-snowball friction is deliberate. | starsector.wiki.gg/wiki/Colony (community) |
| No Man's Sky | Freighter rooms; frigate missions; time-limited community Expeditions. | Frigates, not the player ship (freighter base). | Switching freighters does not refund the base. | Expeditions: a fixed shared seed, phases of milestones in any order, an exclusive permanent reward, then the save converts to normal play. | An expedition is a milestone list over a fixed seed. Nova already has seeded worlds. This is the cheapest mod-authored goal format. | nomanssky.com/expeditions-update (official); nomanssky.miraheze.org Freighter Base Building (community) |
| Elite Dangerous | Fleet carrier: a mobile base with services. Engineers: module grades from materials. | Direct module upgrade grades. | 100,000 cr per carrier jump (community guide). Weekly upkeep figures are unverified. | Unverified. | Upkeep on a mobile base is a clean sink. The claim that the engineer grind was later cut is unverified; do not cite it. | pilotstradenetwork.com fleet-carrier guide (community) |

Lessons I apply below:
1. Purpose comes from a *map of intent* (Expeditions, survey-then-claim), not from a shop.
2. The cheapest goals reward actions that already exist (Factorio triggers).
3. Sinks must scale with location and ambition (Starsector hazard, carrier jumps), not with time spent (EVE retrospective).
4. A station is a logistics project (X4). Do not make it the first step.

## 4. Loop models

Four models. Each is playable alone. They combine in the staged rollout in
section 7. All numbers are **hypothetical** and need a measured
time-to-canister and travel time from a `nova-bench` play first (the EVE
lesson). No timers, dailies or decay in any loop.

### Loop 1: Charter frontier (expedition and survey)

**Pitch.** The player holds a *charter*: a short, visible list of goals over
this world's seed. Goals send the player outward through civilizations of
rising advancement. Completing goals raises the *charter rank*. A higher rank
unlocks the next charter and the rewards of loop 2 and loop 4.

**What a goal is.** Each goal is a condition that the game checks from the
save, never from prose:

| Goal type (hypothetical) | Checked against | Example |
|---|---|---|
| `Reach` advancement band | deterministic civilization advancement derived from seed and position (`civilizations.rs:115-132`); define which civilization owns the position | "Reach space where advancement is at least 0.4." |
| `Survey` a cluster | dwell plus target lock on N bodies of a `ClusterType` (`clusters.rs:244`) | "Lock and scan 3 bodies in a derelict field." |
| `Recover` from a derelict | Take of an item from a ship of an `Extinct` civilization (`civilizations.rs:82-88`) | "Recover 4 salvaged parts from extinct hulls." |
| `Deliver` to a civilization | Sell or Give to a ship of a named living civilization | "Sell 20 water ice to an industrial ship of a far civilization." |
| `Neutralize` | `OnNeutralized` of an Enemy-allegiance ship | "Neutralize 2 armed hulls in hostile space." |
| `Mine` | new observed successful carve/yield event; `MinedOre` (`mining.rs:204-213`) is ore still owed by the rock, not a player credit | Trigger goal: "Mine your first iron." |

**Time scales.**
- First 30 minutes: charter 0 ("Line warship shakedown") holds 4-5 trigger
  goals the player meets by trying each verb: mine any ore, pick up a
  canister, dock and sell, repair one section, cross into a new cell. Each
  completion shows in the objective stack. The last goal names a direction:
  the nearest living Neutral or Player-allegiance civilization.
- Middle (2-6 h): charters 1-3 each need one goal in a band of higher
  advancement (0.2, 0.4, 0.6), one survey and one delivery. Each band brings
  armored traffic (`civilizations.rs:143-160`), richer stock
  (`sector_ships.rs:85`) and richer traders (`:95`).
- Long (10 h and more): "Deep charters" in saturated space (advancement
  near 1.0 past 1000 km); extinct-civilization recovery chains; one
  open-ended *survey ledger* (the infinite sink): each new surveyed cluster
  type in a new band adds rank at a geometric cost (Factorio). Mod charters
  add authored arcs with their own goals.

**What the player builds.** A chart: the record of surveyed clusters,
reached bands and completed charters. It is knowledge, not a structure. It
is saved, shown on the Map, and it gates rewards.

**How the ship improves.** Charter rank unlocks *offers* from loop 2
(restoration grades, authored patches) and loop 4 (licensed demand).
Loop 1 alone gives no stats.

**Budget and sinks.**
- Income: deliveries at regional prices, recovered parts (bid 90 cr,
  `inventory.rs:100-112`), contract payouts (hypothetical: a fixed payout per
  goal, scaled by band).
- Sinks: ammunition and plates on the way out (existing); the stake to open
  a charter (hypothetical, about 10-20% of its payout, so failure costs
  something). Deep charters cost torpedoes in practice, because armored
  traffic rises outward.
- Target: a charter pays for its ammunition and plates plus a margin that
  buys one loop 2 offer (hypothetical 1.5-3x running cost).

**Goal display and completion proof.**
- HUD: the existing objective stack shows the active charter's open goals;
  marker chips point at the target cell or body (`objective_markers.rs`).
- TAB: a Charter pane (new UI) lists charters, goals, rank and rewards.
- Proof: completion is a fact in the progress record; a probe asserts the
  record after a scripted flow, not HUD text.

**Exploit, softlock and grind risks.**

| Risk | Mitigation |
|---|---|
| Cross cells on autopilot to farm "visited". | No goal counts crossing. Survey needs a lock and a dwell on bodies. |
| Reroll offers by reloading. | Offers come from seed + cell + charter id, so a reload gives the same offer. |
| Hostile neighborhood: Enemy allegiance is drawn at 1/3 for each civilization (`civilizations.rs:129-131`), so the nearest band can be hostile. | Charter 0 targets the nearest non-Enemy civilization; the goal picker refuses a world with no reachable candidate inside a bound, with a named reason (decision). |
| Goal target destroyed or emptied in a frozen sector. | The goal checks the frozen body; if the source can no longer satisfy it, the goal is void with a defined result (refund stake). Never re-spawn the source. |
| Loss of a needed section (mining emitter, intake, dock port) blocks goals permanently (`spaceship.rs:578-581`). | Loop 2 restoration must land before any charter depends on that section. |
| Cheats complete goals. | Decision: flag the world as cheated, or ignore. |
| Grind: survey counts that are only bigger. | Each rank asks for a *new* band or cluster type, not a larger count. |

**Persistence and IDs.**
- Content: `CharterDesignId` (authored catalog id, AGENTS.md naming) for
  each charter; goal ids local to the charter.
- Runtime: generated targets named from cell + civilization + index, the
  `sector_id` pattern (`generation.rs:509-511`); never a content id.
- Save: a `progress` field on `WorldSaveState` (hypothetical
  `WorldProgress { charters: BTreeMap<CharterDesignId, CharterState>, surveyed:
  BTreeSet<SurveyKey>, rank: u32 }`), with a `WORLD_SAVE_FORMAT` bump. The
  digest pins charter content (`crates/nova_assets/src/merge.rs:670-783`).

**Before Plan D?** Yes, if the first charters name no items: `Reach`,
`Survey`, `Neutralize`, `Mine` (ore by asteroid kind) and goals that name a
civilization role. `Recover` and `Deliver` name items, so they wait for
Plan D, or they ship as code-only base goals with no RON item reference
(decision, section 6).

**Without stations.** Fully viable. Every goal uses ships, rocks and cells.

### Loop 2: Restoration and refit offers (ship progression without a shipyard)

**Pitch.** The ship is the player's career. It wears down, loses sections
and gets them back. Charter rank and credits buy *restoration* (bring back a
destroyed section) and *offers* (authored patches to one section of the
player's design). Late game, a *hull exchange* swaps to another catalog
design.

**Three grades, smallest first.**

| Grade | What changes | Owner today | New engine work |
|---|---|---|---|
| R1 Restoration | Respawn a destroyed design section at full health. | `FrozenShipState.sections` (`spaceship.rs:578-581`) drops it for good. | A transaction that adds the section back to the live ship and the frozen state. Needs a place rule (anywhere / docked with an industrial ship). |
| R2 Offer (patch) | Apply an authored `SectionConfigPatch` to a named section: e.g. thruster `magnitude` +10%, turret `fire_rate` +15%, section health +25%. | `section_patches` in `ShipDesignSource::Prototype` (`ship_design.rs:309-319`); one `apply` (`patch.rs:106-112`); kind check (`patch.rs:44-56`). | Change the player's saved design source at runtime and respawn or patch live components. Patch fields stay inside the existing kind-checked boundary, so flight validity and connectivity do not change. |
| R3 Hull exchange | Replace `block_line_warship` with another catalog design (e.g. one with more hull sections, so more hold). | `ShipDesignId` (`ship_design.rs:37-46`); `cargo_capacity_g` (`:396-402`). | Move hold, credits and charter state to a new ship; refuse when the hold does not fit. A scenario-level swap of the player ship; big. |

**Time scales.**
- First 30 minutes: none needed. Plate repair already covers damage.
- Middle: the first section loss in a fight is the hook. R1 at a docked
  industrial ship of a living civilization (hypothetical), paid in credits
  plus salvaged parts. First R2 offers at charter rank 1: one mobility, one
  weapon, one durability offer.
- Long: R2 offers stack to a cap for each section (decision: one offer per
  section, or tiers). R3 at a high rank as a capstone: a larger hull, so more
  hold, more mounts. Mod offers add patch sets for their own sections.

**What the player builds.** Their ship: a visible list of applied offers and
restorations on the Ship pane (`crates/nova_interface/src/ship/`).

**Budget and sinks.** This is the main credit sink. R1 cost scales with the
section's catalog value (hypothetical: parts count plus credits). R2 cost
scales with grade and is gated by charter rank, so credits alone cannot buy
the top grade. R3 is the largest single purchase.

**Goal display and proof.** Ship pane shows each section's applied patch and
"restorable" sections. Proof: after a purchase, the saved `FrozenShip` design
source carries the patch, and a resumed world spawns the patched value
(assert the component value, not the pane).

**Exploit, softlock and grind risks.**

| Risk | Mitigation |
|---|---|
| Patch stacking makes combat trivial. | Cap for each section; the patch replaces, not adds (patch fields are `Option` overrides, `patch.rs:302-345`). |
| Restore for free by dying and loading. | Load returns the last save, which already lacks the section. No exploit, but no rescue either. |
| Stranded with no thrusters and no credits. | R1 must exist with a place rule the stranded player can reach, or a "distress tow" rule (decision). Today this is a hard softlock in a saved world. |
| Offer patches outlive a content change. | The digest refuses the world (`save/mod.rs:180-210`), so a stale patch cannot load. Players lose long worlds on every balance patch (section 6, decision 9). |

**Persistence and IDs.** `UpgradeOfferDesignId` (hypothetical) names the
offer; the save stores the resulting patch in `ShipDesignSource`, not the
offer id. That keeps the ship spawnable without the offer catalog (decision:
store offer ids for display, or patches only). R1 stores nothing new: the
section reappears in `FrozenShipState.sections`.

**Before Plan D?** R1 and R2 yes: they name sections and patches, not items,
if the cost is credits only. A cost in parts names `SalvagedParts`; under the
closed enum that is a bare key in a new RON kind and breaks again at Plan D.
R3 does not depend on items.

**Without stations.** R1 and R2 can happen docked with an industrial ship
(a ship-to-ship service). This is an inference: today a docked partner only
trades or is looted (`inventory.rs:483-486`), so a "service" partner is a new
rule. No station required.

**Overlap with the deleted ship-tuning work.** R2 is not an editor. The
player picks authored offers; the patch boundary is fixed by the engine. The
owner deleted "bring a ship editor into the game" for v0.16.0. Whether
authored offers count as the same idea is a product choice (section 8, Q3).

### Loop 3: Infrastructure and logistics (build a foothold)

**Pitch.** The player turns a place into theirs: a *depot* to stage cargo
far out, then a *relay* that marks charted space, then (much later) a
*waystation* that civilian traffic visits.

**Ladder (each step is useful without the next):**

| Step | What it is | Built from today | New work | Station needed |
|---|---|---|---|---|
| I1 Depot | A named, anchored stack of player canisters that the save keeps. | Canisters freeze and persist (`crates/nova_world/src/frozen.rs:386-400`); canister ids are save-checked (`save/mod.rs:335-495`). | Anchor (no drift), a label, a map entry; a hold limit per depot. | No |
| I2 Relay beacon | A deployable marker that charts a cell: shows on the Map, counts as surveyed, and lets loop 4 demand appear there. | `Beacon` exists in scenario content (five ledger scenarios spawn one or more). | A deployable item (Plan D) and a persistent world body. | No |
| I3 Waystation | A static docking hull that trades and offers R1/R2. | Ship-to-ship docking and trading. | An immobile hull type or role. `ShipRoleType` is closed (`generation.rs:169-177`) and opening it reaches layout, parts and civilization weights (`tasks/20261007-085535/TASK.md` findings). Physics of a docked immobile body: unverified. | Yes |
| I4 Player station | X4-like: a plot, staged deliveries of goods, a finished structure. | Nothing. | A structure kind, construction state, builder logistics, UI. | Yes |

**Time scales.**
- First 30 minutes: none.
- Middle: I1 depot at the edge of the first far band, so a deep run does not
  start from the origin.
- Long: relays chart a corridor; a waystation at a charted junction (I3).
  I4 only if the owner wants a builder game.

**Budget and sinks.** I1 costs hold space (canisters stored there are not on
the ship) and a deploy fee. I2 consumes a beacon item. I3/I4 consume large
amounts of raw goods delivered in many trips: the logistics *is* the sink.

**Exploit and softlock risks.** Depot as an infinite hold: cap it. Canister
conservation across retire and save is already an invariant; a depot must
keep it. A waystation trader with regenerating credits creates an arbitrage
loop with loop 4 unless its credits are finite.

**Persistence and IDs.** Depot and relay are world bodies with ids minted
like canisters (`inventory.rs:634-659`), saved in `FrozenSectors`.
`StructureDesignId` (hypothetical) for I3/I4 designs.

**Before Plan D?** I1 yes (canisters exist). I2 needs a beacon item (Plan D,
or a closed-enum item that breaks again). I3/I4 need content kinds that name
goods: after Plan D.

**Without stations.** I1 and I2 stand alone. That is the alternative path:
foothold without stations.

### Loop 4: Regional demand (economy with geography)

**Pitch.** Prices stop being global. Each living civilization *wants* goods
that its environment lacks and *pays* in goods it has: raw in, processed out.
This is the owner's "trade raw materials for processed goods" in place of
crafting. Routes appear between unlike regions.

**Model (hypothetical).**
- Each item gets a base price (Plan D content) and a *tag* set (raw ore,
  volatile, provisions, parts).
- A civilization's buy and sell multipliers come from the three environment
  fields at its centroid (`environment.rs:164-171`) and its advancement:
  material-poor space pays more for ore; volatile-poor space pays more for
  ice; advanced space sells processed goods (plates, ammunition, mod items)
  cheaper.
- Processed goods exist only through trade: an industrial trader sells
  plates for ore at a ratio. No recipe, no crafting UI.
- Trader credits and stock stay finite and frozen with the sector
  (`spaceship.rs:562-571`). **Inference:** nearby traders run dry, so the
  player must range outward. That already happens today and is a natural
  expedition pressure.

**Time scales.**
- First 30 minutes: global prices as now; the first trader sale is a
  charter 0 goal.
- Middle: the Map shows each known civilization's wants (only for
  civilizations the player traded with or surveyed: knowledge as reward).
  The player runs ore from a material-rich band to a poor one.
- Long: high-value mod goods in far bands; supply runs to the player's
  waystation (loop 3).

**Budget and sinks.** Margin per trip is the income; ammunition, plates and
travel risk are the costs. Regional buy-side caps keep a single route from
paying forever.

**Exploit risks.**

| Risk | Mitigation |
|---|---|
| Arbitrage loop between two adjacent traders. | Finite trader credits and stock; ask/bid spread kept at each trader; multipliers derived from the civilization, not the ship, so adjacent ships of one civilization share prices. |
| Rob a trader (neutralize, then `plan_credit_take`). | Exists today. With reputation deleted, the cost is only combat risk. Decision: accept, or add a cost. |
| Market saturation hidden from the player. | Show remaining credits of a docked trader (the Inventory pane already shows partner credits: unverified). |

**Persistence and IDs.** Prices are a pure function of seed, civilization and
item content, so they need no save. Depleted credits and stock are already
saved in frozen ships. Item ids are Plan D `ItemDesignId`.

**Before Plan D?** Partly. Multipliers on the closed enum can ship as code.
Mod goods and authored demand tags need Plan D.

**Without stations.** Fully viable on ship traders.

### Loop 5 (variant of loop 1): Authored expeditions for mods

An `Expedition` is a charter with a fixed seed, as in No Man's Sky: the
mod ships a seed and a milestone list; New Game offers it; on completion the
world continues as a normal open world. This is the most mod-friendly goal
format, because the author can test every milestone against one known world.
It needs `CharterDesignId` plus a `seed` field and a New Game entry. Not
time-limited (no live service).

## 5. Comparison

| | 1 Charter frontier | 2 Restoration/offers | 3 Infrastructure | 4 Regional demand |
|---|---|---|---|---|
| Answers "why go there" | Yes, directly | No | Late | Yes, by margin |
| Answers "what to spend on" | Stakes only | Yes, main sink | Yes, late | Indirect |
| Ship improves | Through unlocks | Yes | No | No |
| Reuses existing systems | Civilizations, fields, freeze, objectives HUD | Patches, design source, save | Canisters, freeze | Trade, frozen credits |
| Biggest new work | Progress record, goal checks, Charter pane | Runtime design change, section respawn | Anchoring; later a static hull | Price function, Map knowledge |
| Viable before Plan D | Yes (item-free goals) | R1/R2 with credit costs | I1 only | Code multipliers only |
| Needs stations | No | No | I3/I4 only | No |
| Worst risk | Hostile start; void goals | Combat trivialized; strand softlock | Infinite hold; arbitrage | Arbitrage; robbing |

## 6. Why not shipyard first

A design that opens with "a shipyard where the player buys sections and
hulls" fails on evidence:

1. **It answers the wrong question first.** The Builder paper proposes
   docked-trader refit, not a station shipyard. This objection applies to
   refit-first priority, not to a station prerequisite. A shop answers "what to spend
   on". The open world has no answer to "why go anywhere"
   (`open_world.rs:1-9`). Upgrades with no goal to use them on only make the
   same sandbox easier.
2. **The owner deleted it for this release.** The ship-tuning spike and
   "Bring player ship tuning and section editing into the game" were removed
   (`tasks/20261007-085535/TASK.md`). A shipyard-first plan restarts that
   work under another name.
3. **It needs a place that does not exist.** Docking is ship to ship
   (`docking_section/mod.rs:12-16`); no station exists in generation
   (`clusters.rs:244-254`). A shipyard needs I3 first: a closed role enum to
   open and physics to verify.
4. **Free section editing is the most expensive form.** Sections have no
   generic mass, cargo or mining-rate stat; hold comes from the hull count
   (`ship_design.rs:392-402`). Editing layout touches connectivity, flight
   validity, clearance and the save: `FrozenShip` stores a design source and
   surviving section ids (`spaceship.rs:562-581`), not an edited layout. An
   `Inline` edited design per player ship is possible but large.
5. **The cheaper ship progression is not safe just because it fits the save.**
   Authored patches (R2) live in `section_patches` (`ship_design.rs:309-319`),
   but thaw reapplies saved health and ammo capacity (`frozen.rs:280-299`).
   A live change needs a section spawn/replacement seam and a matching frozen
   state update. R1 also needs live respawn; neither is a free save-only edit.
6. **Mods.** A shipyard sells section and hull ids, which are the kinds with
   silent cross-bundle last-wins today (`merge.rs:850-941`) and a precedent
   for dependency-ordered overlays (`webmods/gauntlet/gauntlet.bundle.ron:17-21`,
   historical rather than a current unrelated collision).
   Selling them makes merge order a price and balance decision before the
   collision policy is settled.

A shipyard is a valid capstone (R3 hull exchange, then I3), not a spine.

## 7. Plan D and progression content: a robust migration

### 7.1 Two facts that shape the order

- **Any new RON kind that names an item before Plan D ships a second item
  spelling that Plan D must break again.** The bare key `{PdcRound: n}`
  already shipped in base scenarios and the six ledger scenario files
  (`webmods/the-ledger/ledger_0[1-6]_*.content.ron`). `webmods/gauntlet` has
  no stock. Correction to [RESEARCH.md](RESEARCH.md) 2 Plan B: "migrate ...
  gauntlet" is not needed for stock; base scenarios other than
  `open_world` (drills, menus, `season_one_chapter_one`) also author stock,
  but they are generated from builders. The ledger id `"racer"` is defined
  only in the ledger (`ledger_ships.content.ron:6`); no base id collides.
- **The save format has not shipped.** A `progress` field with a format bump
  costs no migration (owner: no save compatibility between versions).

### 7.2 Stages

| Stage | Content | Items | Plan D relation |
|---|---|---|---|
| P0 | Measure: one `nova-bench` New Game play records time-to-first-canister, credits per hour from ore, and travel time across one civilization lattice step (240 km). | none | Independent |
| P1 | Charter 0 and charters 1-3 with item-free goals; `WorldProgress` in the save; Charter pane; objective stack rebuilt on resume. R1 restoration for credits. | Closed `ItemType` untouched. | Before Plan D. Charters are code-authored, or a `Content::Charter` whose goals name no item. |
| P2 | Plan D Stage 2 from [RESEARCH.md](RESEARCH.md): `ItemDesignId`, `Content::Item`, collision rule, quoted keys `**(breaking)**`. | Open | Plan D itself |
| P3 | Item-naming goals (`Recover`, `Deliver`); R2 offers with part costs; regional demand tags on items; depot I1. | `ItemDesignId` | After Plan D |
| P4 | Plan D Stage 3 bindings (mod ammo, ore, repair) plus relay item I2; mod expeditions (loop 5). | Open bindings | After Plan D Stage 3 |
| P5 | R3 hull exchange; I3 waystation if an immobile hull proves sound. | | Independent of items; gated on stations |

**Alternative if the owner wants no RON charter kind before Plan D:** keep
P1 goals in Rust (base-only, like the open-world bootstrap builder) and make
`Content::Charter` part of P3. Consequence: mods get no goals until P3.

### 7.3 New content kinds and their ID, merge and failure rules

Each new kind touches the same list as any kind: `Content` variant,
`kind()`/`id()` arms (`crates/nova_modding/src/lib.rs:77-157`), a
`MergeOutcome` field and a routing arm (`merge.rs:811-842`, `:897-940`), a
reference arm beside `section_errors` (`merge.rs:620-666`), a lint module
(`crates/nova_scenario/src/lint/`), a builder under
`crates/nova_authoring/src/base_content/`, docs and changelog. The
duplicate check inside one bundle and the digest are generic over
`kind()`/`id()` (`merge.rs:748-783`, `:866-884`).

| Kind (hypothetical) | Id | References | Merge |
|---|---|---|---|
| `Charter` | `CharterDesignId` | goal targets: items, civilization roles, cluster types, asteroid kinds | Plan D rule (refuse unrelated duplicates) |
| `UpgradeOffer` | `UpgradeOfferDesignId` | section id in a ship design, a `SectionConfigPatch`, costs in items | Plan D rule |
| `Structure` (I3/I4) | `StructureDesignId` | a ship design id, services, goods | Plan D rule |
| `Item` (Plan D) | `ItemDesignId` | category | Plan D rule |

Recommendation: apply the Plan D collision rule to *every new progression
kind* from its first day. It is cheap at the collision point:
`transitive_deps` answers dependency direction from the graph the merge
already built (`crates/nova_mod_format/src/deps.rs:54-80`;
`merge.rs:289-296`). Leave `Section`, `Ship`, `Style`, `Lesson`, `UiTheme`
on last-wins until the owner decides; changing them is a real behavior change
(historical dependency-ordered overlays are a separate policy choice).

Must fail loudly:
- A goal, offer or structure that names an unknown item, section, design,
  cluster type or asteroid kind: lint error; base fatal; mod quarantined
  (`merge.rs:184-268`).
- An offer patch whose kind does not match the target section:
  `SectionPatchError` at lint and at resolution (`patch.rs:41-56`).
- A charter with no goals, or a goal with no completion rule: lint error.
  No default reward and no default goal.
- A save whose progress names a charter the catalog lacks: the digest
  refuses first (`save/mod.rs:180-210`); a state parse failure lists the world
  as `Unreadable` (`save/mod.rs:299-302`).
- A goal whose target can no longer exist: void with the authored result,
  never a silent re-spawn.

### 7.4 Call graph (hypothetical)

Before:

```
New Game -> open_world scenario -> spawn player
streaming -> generate_sector(seed, cell) -> materialize -> freeze on retire
dock -> plan_item_trade | plan_item_transfer | plan_credit_take
save on crossing/leave -> WorldSaveState { player, sectors, .. }
```

After P1-P3:

```
merge_bundles -> Content::{Item, Charter, UpgradeOffer} -> collision rule
             -> GameItems, GameCharters, GameUpgradeOffers (digest covers all)
New Game -> open_world -> spawn player -> WorldProgress::new(charter 0)
resume -> WorldSaveState.progress -> rebuild GameObjectives + marker chips
gameplay events (mine, dock trade, take, neutralize, lock+dwell, cell)
   -> progress_check(&WorldProgress, &GameCharters, world facts) -> goal done
   -> rank, unlocked offers
dock industrial ship -> plan_restoration | plan_offer_purchase
   -> FrozenShip.design.section_patches / sections (saved on next crossing)
save on crossing/leave -> WorldSaveState { player, sectors, progress, .. }
```

## 8. Unresolved product choices

1. **Spine:** charter frontier (recommended), regional demand first, or
   restoration first?
2. **Goal source:** authored charters only, seeded generated contracts only,
   or authored arcs with generated filler (recommended)?
3. **Ship improvement scope:** do authored patch offers (R2) fall under the
   deleted ship-tuning decision, or are they allowed? Is restoration (R1)
   wanted, and where (anywhere, docked with an industrial ship, depot)?
4. **Stranded player:** a distress tow, a free R1 at the nearest living
   civilization, or permanent loss with Load last save as the only rescue?
5. **Charter content before Plan D:** Rust-only base goals in P1, or a RON
   `Charter` kind restricted to item-free goals?
6. **Collision rule:** Plan D rule for all new progression kinds from day one
   (recommended), or last-wins until a separate decision?
7. **Stations:** none for v0.16.x (recommended: I1 depot as the foothold), a
   static-hull waystation spike, or a player-built station goal?
8. **Robbing traders:** with reputation deleted, accept robbery as a free
   strategy, or add a non-reputation cost?
9. **Balance patches versus long worlds:** every content change refuses a
   world (digest). Progression makes worlds long. Keep strict refusal, or
   pin progression balance outside the digest (a persistence-task decision)?
10. **Web:** no saved worlds on web. Hide charters on web, keep them
    session-only, or block the long loop there?
11. **Cheats:** mark a world as cheated, block completion, or ignore?
12. **Infinite sink:** survey ledger with geometric cost (Factorio), offer
    tiers past a cap, or no infinite sink?

## 9. Proof plan (for a later gate; not authorized)

- P0: one recorded `nova-bench` New Game play; report if canister pickup,
  sale and travel actually complete, with observed elapsed times. One run
  cannot establish a reliable earnings rate. Numbers stay hypothetical.
- Pure: `progress_check` over fixed facts; offers drawn from seed + cell are
  equal across reloads and visit orders; unrelated duplicate charter ids
  refuse with both pack ids.
- ECS: a goal completes from a real dock sale or neutralize event; an offer
  purchase changes the live component value and the frozen design source;
  a restored section spawns again after retire and revisit.
- Save: `examples/systems/system_world_resume.rs` round trip with progress,
  an applied patch and a restored section; a changed charter catalog refuses
  with `WorldRefusal::Catalog`.
- Player flow: `nova-bench` play of charter 0 from New Game, judged by the
  saved progress record and frames, not by exit status.
