# Crafting rework: station and on-ship recipes, more items, consumables

- STATUS: CLOSED
- PRIORITY: 85
- TAGS: v0.19.0, economy, crafting, content

Planned 2026-11-16 from the owner review of the game-feel research doc
(claude.ai artifact Hq6GmMBXm2zAfVFwGnQg45).

## User facts
- Wants more items, more crafting and refine methods, and consumables for the
  ship.
- "the market+inventory+crafting system can be re-worked a bit, how we add
  sections to a ship, how we lose them and get them back".
- Some recipes need a station; others can be made on the ship, "just like
  minecraft has 2x2 vs 3x3 crafting".
- Owner call 2026-11-16: this reverses "No crafting tree"
  (`tasks/20261012-095526/DESIGN.html:219`), "crafting is not a goal"
  (`tasks/20261012-095526/RESEARCH-CODEBASE.md:267`) and the no-consumables
  scope (`tasks/20260926-174806/TASK.md:10,16`).
- Still on record, not reversed: no blueprints or known-design set
  (`DESIGN.html:499`).

## Decisions
- Owner 2026-11-16: more recipes mean the Inventory pane's Refine verb is
  not enough. Crafting gets its own UI, probably modelled on the Ship pane
  section catalogue (`crates/nova_interface/src/ship/catalogue.rs`, gallery
  in `crates/nova_ship/src/gallery/`). Share any catalogue UX fix with task
  20261116-133436.

- First slice (owner 2026-11-16: "go with recommended options"; agent
  calls from `DESIGN.html`, stated 2026-11-17):
  - Content: a `Recipe` kind (id, inputs, output, duration_secs, site)
    replaces the item `refine` field. `CraftSiteType::{Ship, Station {
    fee_cr }}`. Station service `Refine` becomes `Craft` (advancement 0.25).
    Lint keeps the no-loss price rule and adds output mass <= input mass.
    **(breaking)**; the example mod migrates. No save alias for `Refine`:
    a save of another content catalog is refused at the header digest
    (`crates/nova_assets/src/merge.rs` `content_catalog_digest`), and the
    base content changed this release, so no v0.18.0 save reaches those
    fields (agent call, 2026-11-17).
  - On-ship crafting is built into every hull. No fabricator section yet.
  - Recipes: PDC rounds, hull plate, section panel on the ship; alloy
    plate, rail slug, torpedo at a station. Amounts and times per
    `DESIGN.html`. No new items.
  - One timed craft job per ship on the gameplay clock, saved and resumed.
    Inputs and fee are taken at the end of each batch. Station recipes stop
    on undock and hold while no Craft dock is present (the Load gap);
    Cancel keeps finished batches. A hull buy or board cancels the job.
  - UI: crafting lives in the Inventory pane (owner 2026-11-17, replaces
    the first-slice Craft tab): a Craft button in the Inventory pane opens
    the craft view over the pane, the way the Ship pane's Catalogue button
    opens the catalogue; Esc or Back returns to the Inventory. No Craft tab
    in the top tab row. The Inventory Refine form and its examples die.
  - Deferred: consumables, sections as items, market and inventory rework,
    catalogue UX, fabricator section, craft queue, `OnCrafted` event.

- Second slice (owner 2026-11-17: "do some research and figure it out";
  agent calls from `DESIGN-2.html`, code map `MAP-2.md`, stated 2026-11-17):
  - Consumables: a new `Consumable` content kind keyed by item id, with a
    closed effect set `Repair { health, duration_secs }`,
    `ThrustBoost { factor, duration_secs }`, `Flare { near_m, far_m, lure_m }`.
    Lint refuses an unknown item, an engine-used item, bad numbers, a lure
    point inside the largest torpedo blast, and two consumables of one
    effect. **(breaking)** for mods only where noted below.
  - Items and Ship recipes: RepairKit (heals 80 HP over 10 s, lowest-share
    live section each tick, never a lost section; 2 HullPlate + 1
    SectionPanel, 12 s), BoostCell (thrust x1.5 for 8 s, manual flight only;
    1 WaterIce + 1 CarbonOre, 8 s), DecoyFlare (torpedoes homing on the
    ship from 400 to 3000 m lose lock and fly to a point 800 m aside;
    1 IronOre + 1 CarbonOre, 6 s). Prices and masses per `DESIGN-2.html`.
  - The flare is the cut line: first check that some AI ship fires
    torpedoes at the player in normal play; if none does, the flare moves
    to a later slice.
  - Use: rebindable flight actions on keys 1, 2, 3 (no pad default), and
    a `Use` chip on the player's own rows in the Inventory pane. A running
    effect of the same kind refuses a second use; a refusal keeps the item
    and plays the deny cue, with no notice: the dim dock chip shows it
    (owner UI rule 2026-11-17, no restating text). A keybind dock chip
    shows while the hold carries the item or its effect runs. A mod ship that binds a section to 1, 2 or 3 fails
    lint **(breaking)**.
  - Running effects are saved on the frozen ship and resume on Load.
  - Supply: craft, station stock and start kits, per `DESIGN-2.html`.
  - Content-only part recipes for the dock chain: `base/docking_collar`
    and `base/actuator_pack`, so a lost docking port rebuilds in the field.
  - Later: fabricator section, craft queue, Inventory and Market rework
    (after task 20261116-133436), decoy against AI targeting, AI use of
    consumables. Never: sections as items (field rebuild already takes
    hold parts), ammo packs, shield cells or heat sinks.

- Owner revision after play (2026-11-17), replaces parts of the second
  slice:
  - Keep only the repair kit. Boost cell and decoy flare are removed
    (items, recipes, effects, stock, kits, lint). Owner: "flares are bad,
    I don't like them, at least right now"; boost "kind of pointless".
  - No flight keys for consumables: "I don't really want to be able to
    use them from the world ... meh to add that many keybinds". The
    Inventory Use button is the only way to use one; keys 1, 2, 3, their
    dock chips and the section-key lint on 1-3 are removed.
  - Owner ideas, not scheduled: a distress beacon item ("sounds good"),
    and a tractor that pulls canisters in, as a section rather than a
    consumable.

## Agent findings
- 23 items in six categories (`assets/base/items/base.content.ron:3-236`,
  `crates/nova_gameplay/src/inventory.rs:391-406`); builders in
  `crates/nova_authoring/src/base_content/items.rs`.
- One refine recipe: AlloyPlate = 2 IronOre + 1 CarbonOre + 30 cr
  (`items/base.content.ron:213-233`). 26 section build recipes
  (`inventory.rs:1829-1838`). Lint refuses a refine that loses money
  (`crates/nova_assets/src/items.rs:179-214`).
- Refine is station-only, from advancement 0.25
  (`inventory.rs:1193-1195`, `crates/nova_world_base/src/station_layout.rs:143,568`).
- No consumables: thrust uses no stock (`inventory.rs:387-388`); Medkit and
  Rations are job and trade goods only.
- Inventory pane: one form for Give, Sell, Store, Buy, Take, Withdraw and
  Refine, about 8.3k lines (`crates/nova_interface/src/inventory/mod.rs`).
- Market rules are keyed by category, never by item
  (`crates/nova_gameplay/src/market.rs:23-25`).
- Item, recipe and beam prices are provisional
  (`tasks/20261111-122537/TASK.md:20-23`).

## Open decisions (owner)
- On-ship tier: built into every hull, or needs a fabricator section.
- Recipe list for each tier, and which items are new.
- Consumable kinds (agent ideas, not approved: field repair kit, ammo packs,
  thrust boost, sensor flare, decoy).
- Whether sections become items, or stay station-built only.
- Scope of the Inventory and Market pane rework.

## Delivery
1. Design page in this task folder (`DESIGN.html`): item list, recipe tiers,
   consumables, UI flow, how it links to field repair and ammo. Owner
   approves before code.
2. Content format and lint first (explicit fields, known IDs), then
   builders, `content -- gen`, then runtime and UI.
3. Changelog marks format breaks **(breaking)**; migrate the example mod.

## Verification
- Unit tests for recipe validation and lint refusals.
- ECS asserted example: an on-ship recipe crafts in flight; a station recipe
  refuses undocked.
- A playable example for the crafting loop.

## Done when
- The approved design is implemented: on-ship and station recipes work,
  consumables have an effect in flight, and the wiki explains the loop.

## Delivery record (2026-11-17, first slice; task stays OPEN)
- Landed: `Recipe` content kind and builders (`assets/base/recipes/`), recipe
  rules (`crates/nova_gameplay/src/crafting.rs`), timed craft job
  (`crates/nova_ship/src/sections/craft_job.rs`), Craft pane
  (`crates/nova_interface/src/craft.rs`), HUD chip and notice, saved job,
  Refine removed (service, credit cause, Inventory form, two examples),
  example mod migrated, `craft_sandbox` and `system_ship_craft` examples.
- Review (independent, read-only): 4 defects, 3 low; all fixed. Mod recipe
  faults refuse the owning pack at load again (price rule against the merged
  catalog); a docked Station job survives Load; hull buy cancels the job;
  save job fields are strict (no v0.18.0 exception); batch cap checked on
  load; website widgets moved to Craft.
- Verified: targeted unit and save tests (the docked-Station round trip
  fails with the old stop rule, passes now); content lint base and example;
  `cargo check --workspace --all-targets --features debug`; live under Xvfb
  and lavapipe: `system_ship_craft` PASS, `craft_sandbox` PASS, frames of the
  idle and running pane inspected.
- Not verified: live save and Load mid job, live Cancel, editor recipe
  editing, hull-buy cancel (no test). Workspace tests and clippy not run.
- Open: item names not pluralised. Fixed 2026-11-18: a docked station that
  turns derelict stops a Station job at once, as an undock does (a dock not
  rebuilt yet after Load still holds); the batch count survives a Craft view
  reopen (the amount row no longer reads its spawn values as a move).
- Left for later slices: consumables, sections as items, market and
  inventory rework, fabricator section, craft queue, `OnCrafted`.

## Delivery record (2026-11-17, second slice; task CLOSED)
- Landed: `Consumable` content kind and lint (`crates/nova_gameplay/src/consumable.rs`),
  RepairKit, BoostCell and DecoyFlare items and Ship recipes, part recipes
  `base/docking_collar` and `base/actuator_pack`, station stock and start
  kits, effects (`crates/nova_ship/src/sections/consumables.rs`), keys 1, 2,
  3 and the Inventory Use button, dock chips that wrap on narrow screens,
  saved running effects, wiki page `web/src/wiki/crafting.md`,
  `system_ship_consumables` and `consumables_sandbox` examples.
- Flare gate: AI ships fire torpedoes at the player, so the flare stayed.
- Review (independent, read-only): 2 medium, 3 low; fixed: flare lure must
  exceed 1.5x the largest blast (lured torpedoes fuze up to half a radius
  short), one flare per ship per pass, largest blast reads inline and
  scenario bays, changelog notes a custom binding on 1-3 resets. A refused
  use posts no notice by the owner UI rule (deny cue and dim chip).
- Same commit: owner UI text pass (no restating prose in the Craft view,
  Ship pane repair/SOS/lost sections, Map, Jobs, HUD notices).
- Verified: targeted unit, ECS and save tests; content lint; workspace
  check; live under Xvfb and lavapipe: `system_ship_consumables` PASS,
  `consumables_sandbox` PASS, `craft_sandbox`, `sos_rescue_sandbox`,
  `field_repair_sandbox` PASS; frames read (chips, healed section, boost,
  flare notice, Use button, dock wrap at 1280x720, craft card, SOS pane).
- Not verified: hand play, gamepad, a live Load mid effect, audio cues,
  `mdbook build`, workspace tests and clippy.
- Later (not in this task): fabricator section, craft queue, Inventory
  and Market rework (after task 20261116-133436), decoy against AI
  targeting, AI use of consumables, per-item use sounds, lure puff VFX,
  Inventory pane text pass.
- Owner revision landed (2026-11-17): boost cell, decoy flare, keys 1-3,
  their dock chips and the section-key lint removed; the repair kit stays,
  used from the Inventory Use button only (`ConsumableType::Repair`, one
  variant). Verified: targeted tests, content lint, workspace check, live
  `consumables_sandbox` and `system_ship_consumables` PASS, frames read.
