# Crafting and consumables

Ore and salvage become parts, ammunition and three consumables. You craft on the ship or at a station, use a consumable in flight with one key, and fix the hull with plates, kits and parts. Nothing here needs a station except the Station recipes.

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
            >In flight with a damaged hull: the KIT, BOOST and
            FLARE chips on the keybind dock, and the status chip
            over the ship reading KIT 6 S / BOOST 5 S.</span
        >
    </div>
</figure>

## The loop

| Step | Where | How |
| --- | --- | --- |
| Get material | anywhere | Cut ore with a [mining beam](../sections/mining-beam/), [buy](../interface/#buy-and-sell) it, or [take](../interface/#take-and-give) it from a wreck. |
| Craft | Ship recipe: anywhere. Station recipe: docked at a station that offers Craft | The [Craft view](../interface/#crafting) of the Inventory pane. One job at a time, on the game clock. |
| Use | in flight, or in the Inventory pane | <kbd>1</kbd> repair kit, <kbd>2</kbd> boost cell, <kbd>3</kbd> decoy flare. Or **Use** in the item's inspector. |
| Repair and rebuild | in flight, or docked | Hull plates, repair kits and [field rebuilds](../interface/#the-ship) from hold parts. |

## Consumables

<!-- Numbers: assets/base/consumables/base.content.ron (Repair health 80.0
     duration_secs 10.0; ThrustBoost factor 1.5 duration_secs 8.0; Flare near_m
     400.0 far_m 3000.0 lure_m 800.0) and assets/base/items/base.content.ron
     (RepairKit 20 kg Repair 120/90, BoostCell 10 kg Parts 26/20, DecoyFlare
     5 kg Ammo 24/18). Keys: crates/nova_ship/src/input/bindings.rs
     `flight_bindings` (use_repair_kit Digit1, use_boost Digit2, use_flare
     Digit3, no gamepad). Refusals in check order:
     crates/nova_gameplay/src/consumable.rs `plan_consumable_use`. -->

| Item | Key | Effect | Refused when | Mass | Ask / bid |
| --- | --- | --- | --- | --- | --- |
| Repair kit | <kbd>1</kbd> | Heals 80 HP over 10 s, always on the most damaged live section. | none held; a kit runs; no live section is damaged | 20 kg | 120 / 90 cr |
| Boost cell | <kbd>2</kbd> | Thrust x1.5 for 8 s, in manual flight only. | none held; a boost runs; docked; no live thruster | 10 kg | 26 / 20 cr |
| Decoy flare | <kbd>3</kbd> | At once: each torpedo homing on you from 400 to 3,000 m drops its lock and flies to a point 800 m to one side. | none held; no torpedo in that band homes on you | 5 kg | 24 / 18 cr |

| Rule | What happens |
| --- | --- |
| One use | takes one item at once. There is no cancel. |
| Same type twice | refused while that effect runs, so a double press never wastes an item. Different types run together: a kit and a boost at once. |
| A refusal | keeps the item and plays the cockpit deny cue. The dock chip already showed it dark. |
| Interface open | the game clock stops, so a kit or boost started with **Use** runs when you close the interface. |
| Save | a running kit or boost is saved with your ship and resumes on Load. Buy another hull and it ends with the old one. |

<details class="explain">
<summary>Show explanation</summary>

<!-- Repair: crates/nova_ship/src/sections/consumables.rs
     `advance_consumable_effects` with crates/nova_gameplay/src/consumable.rs
     `plan_repair_split`. Boost: `ShipConsumableEffects::thrust_factor`, read by
     crates/nova_ship/src/sections/thruster_section.rs. Flare:
     `use_consumables` and `flare_lure_point`; the lured torpedo fuzes on a bare
     aim point at half its blast radius,
     crates/nova_ship/src/sections/torpedo_section/projectile.rs. -->

**Repair kit.** The kit gives its health evenly over its 10 s. Each moment's share goes to the live, enabled section with the lowest share of its health, fills it, and passes the rest to the next. It never brings back a lost section. When nothing is left to heal the kit ends early, and the rest is lost.

**Boost cell.** The boost scales every thruster's push. While GOTO, ORBIT, STOP or MATCH flies the ship, the boost does not apply, but its 8 s still count down: the autopilot plans with your normal thrust.

**Decoy flare.** Each torpedo pulled off flies to its own point 800 m from your ship, at right angles to its approach, and bursts there as a torpedo whose target died does. A base warhead's blast is 300 m, so the burst misses you. A torpedo inside 400 m keeps its lock: your [point defense](../combat-weapons/#point-defense) is the last layer.

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
| Boost cell | 1 water ice, 1 carbon ore | 1 boost cell | 8 s | Ship |
| Decoy flare | 1 iron ore, 1 carbon ore | 1 decoy flare | 6 s | Ship |
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
| Repair kit | 1 kit for 80 HP, spread to the most damaged sections | 10 s | key <kbd>1</kbd> or **Use**, anywhere |
| Field rebuild | the lost section's recipe parts from the hold | 5 s a part, 20 s at least | Ship pane, undocked |
| Station rebuild | the recipe's credits and parts | at once | Ship pane, docked at a station that rebuilds |
| SOS | a station's bill | 60 s | Ship pane, when stranded |

A lost docking port rebuilds in the field from 1 docking collar, 1 actuator pack and 2 section panels, in 20 s. The collar needs an alloy plate, and an alloy plate needs a Craft station: carry one.

## Where consumables come from

<!-- Start kits: crates/nova_authoring/src/base_content/start_roles.rs.
     Station stock: CONSUMABLE_GOODS_WEIGHT in
     crates/nova_world_base/src/station_layout.rs. -->

| Source | Repair kits | Boost cells | Decoy flares |
| --- | --- | --- | --- |
| Start kit: industrial, transport | 3 | 3 | - |
| Start kit: scavenger, bounty hunter, military, pirate | 3 | - | 4 |
| Generated station | stocked | stocked | stocked |
| Craft | Ship recipe | Ship recipe | Ship recipe |

Every generated station stocks all three as trade goods, and a bought-out station restocks while its sector is unloaded. A dead station keeps its stock as loot. A mod consumable is stocked the same way.
