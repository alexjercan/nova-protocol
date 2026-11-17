# Crafting and consumables

Ore and salvage become parts, ammunition and repair kits. You craft on the ship or at a station, use a repair kit from the Inventory pane, and fix the hull with plates, kits and parts. Nothing here needs a station except the Station recipes.

<figure class="figure">
    <!-- Capture: assets/wiki-crafting-loop.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Screenshot</span
        >
        <span class="figure__placeholder-name"
            >assets/wiki-crafting-loop.png</span
        >
        <span class="figure__placeholder-note"
            >In flight with a damaged hull: the status chip
            under the RCS pips reading KIT 6 S.</span
        >
    </div>
</figure>

## The loop

| Step | Where | How |
| --- | --- | --- |
| Get material | anywhere | Cut ore with a [mining beam](../sections/mining-beam/), [buy](../interface/#buy-and-sell) it, or [take](../interface/#take-and-give) it from a wreck. |
| Craft | Ship recipe: anywhere. Station recipe: docked at a station that offers Craft | The [Craft view](../interface/#crafting) of the Inventory pane. One job at a time, on the game clock. |
| Use | the Inventory pane, docked or not | Select your repair kit row and press **Use**. |
| Repair and rebuild | in flight, or docked | Hull plates, repair kits and [field rebuilds](../interface/#the-ship) from hold parts. |

## Consumables

<!-- Numbers: assets/base/consumables/base.content.ron (Repair health 80.0
     duration_secs 10.0) and assets/base/items/base.content.ron (RepairKit
     20 kg Repair 120/90). Refusals: crates/nova_gameplay/src/consumable.rs
     `plan_consumable_use`. Use: crates/nova_ship/src/sections/consumables.rs
     `use_consumables`. -->

| Item | Effect | **Use** greyed when | Mass | Ask / bid |
| --- | --- | --- | --- | --- |
| Repair kit | Heals 80 HP over 10 s, always on the most damaged live section. | none held; a kit runs; no live section is damaged | 20 kg | 120 / 90 cr |

| Rule | What happens |
| --- | --- |
| How to use | Inventory pane, your own kit row, **Use**. No flight key. |
| One use | takes one kit at once. There is no cancel. |
| None held | kits held back by a running repair job do not count. |
| A refusal | keeps the kit and plays the cockpit deny cue. |
| Clock | the kit heals on the game clock: Tab and pause hold it, so a kit started with **Use** runs when you close the interface. |
| Save | a running kit is saved with your ship and resumes on Load. Buy another hull and it ends with the old one. |

<details class="explain">
<summary>Show explanation</summary>

<!-- crates/nova_ship/src/sections/consumables.rs `advance_consumable_effects`
     with crates/nova_gameplay/src/consumable.rs `plan_repair_split`. -->

The kit gives its health evenly over its 10 s. Each moment's share goes to the live, enabled section with the lowest share of its health, fills it, and passes the rest to the next. It never brings back a lost section. When nothing is left to heal the kit ends early, and the rest is lost.

</details>

## Recipes

<!-- assets/base/recipes/base.content.ron, from
     crates/nova_authoring/src/base_content/recipes.rs `recipe_catalog`. -->

| Recipe | One batch takes | Makes | Time | Where |
| --- | --- | --- | --- | --- |
| PDC round | 1 iron ore, 1 stone ore | 100 PDC rounds | 8 s | Ship |
| Hull plate | 2 iron ore | 1 hull plate | 8 s | Ship |
| Section panel | 2 carbon ore | 1 section panel | 6 s | Ship |
| Repair kit | 2 hull plates, 1 section panel | 1 repair kit | 12 s | Ship |
| Docking collar | 1 alloy plate, 5 iron ore | 1 docking collar | 20 s | Ship |
| Actuator pack | 4 iron ore | 1 actuator pack | 10 s | Ship |
| Alloy plate | 2 iron ore, 1 carbon ore, 30 cr | 1 alloy plate | 10 s | Station |
| Rail slug | 2 iron ore, 10 cr | 1 rail slug | 10 s | Station |
| Torpedo | 2 alloy plates, 1 actuator pack, 9 carbon ore, 40 cr | 1 torpedo | 30 s | Station |

## Fixing the ship

<!-- Plates: HULL_PLATE_HEALTH 20, PLATE_REPAIR_SECS 4; field rebuild:
     FIELD_REBUILD_SECS_PER_PART 5, FIELD_REBUILD_MIN_SECS 20, all in
     crates/nova_gameplay/src/inventory.rs. Docking port build parts:
     assets/base/sections/base.content.ron docking_port_section (1
     ActuatorPack, 1 DockingCollar, 2 SectionPanel). -->

| Fix | Takes | Speed | Where |
| --- | --- | --- | --- |
| Plate repair | 1 hull plate for up to 20 HP on the section you pick | 1 plate every 4 s | [Ship pane](../interface/#the-ship), anywhere |
| Repair kit | 1 kit for 80 HP, spread to the most damaged sections | 10 s | **Use** in the Inventory pane, anywhere |
| Field rebuild | the lost section's recipe parts from the hold | 5 s a part, 20 s at least | Ship pane, undocked |
| Station rebuild | the recipe's credits and parts | at once | Ship pane, docked at a station that rebuilds |
| SOS | a station's bill | 60 s | Ship pane, when stranded |

A lost docking port rebuilds in the field from 1 docking collar, 1 actuator pack and 2 section panels, in 20 s. The collar needs an alloy plate, and an alloy plate needs a Craft station: carry one.

## Where repair kits come from

<!-- Start kits: crates/nova_authoring/src/base_content/start_roles.rs.
     Station stock: CONSUMABLE_GOODS_WEIGHT in
     crates/nova_world_base/src/station_layout.rs. -->

| Source | Repair kits |
| --- | --- |
| Start kit: every base role | 3 |
| Generated station | stocked where its market deals in them |
| Craft | Ship recipe |

A bought-out station restocks while its sector is unloaded. A dead station keeps its stock as loot. A mod consumable is stocked the same way.
