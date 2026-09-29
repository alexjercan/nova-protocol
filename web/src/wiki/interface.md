# Interface

Every ship carries a second seat of control: the **interface**. Press <kbd>Tab</kbd> in flight and a full-screen panel opens over the frozen world with three panes: **Map**, **Ship** and **Inventory**. Flight resumes the moment you close it.

<figure class="figure">
    <!-- Capture: assets/wiki-interface-map.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Screenshot</span
        >
        <span class="figure__placeholder-name"
            >assets/wiki-interface-map.png</span
        >
        <span class="figure__placeholder-note"
            >The Map pane over a frozen skirmish: range rings,
            contact icons, the contact panel on a selected
            hostile, and the footer legend.</span
        >
    </div>
</figure>

## Opening and closing

<!-- Open + close: crates/nova_interface/src/pane.rs `toggle_interface`
     (needs a player ship; Unpaused -> Interface, Interface -> Unpaused; Tab is
     left alone under the command modal). Pane switch: `next_interface_pane`.
     Defaults: crates/nova_interface/src/bindings.rs `interface_bindings`
     (interface_toggle Tab / Right Thumb, interface_next_tab M / Y).
     Back-out owner (Escape, Start): crates/nova_interface/src/terminal/input.rs
     `close_surface_from_menu_keys`. First open shows Map, later opens the last
     pane: `InterfacePaneType` in pane.rs. -->

| You do | What happens |
| --- | --- |
| <kbd>Tab</kbd> (or click the right stick) | The interface opens on the Map pane, or on the pane you last used. It needs a live ship, and it does not open over the pause menu. |
| <kbd>Tab</kbd> again, <kbd>Esc</kbd>, or gamepad Start | Close it and return to flight. |
| <kbd>M</kbd> (or gamepad <kbd>Y</kbd>) | Step to the next pane: Map, Ship, Inventory, then Map again. While a text field has the caret, such as the Inventory quantity, <kbd>M</kbd> types instead. |
| Click **Map**, **Ship** or **Inventory** | Switch to that pane. |
| <kbd>:</kbd> | Open the [command shell](../commands/) over the pane. Close it and you are back on the same pane, with its camera and selection as you left them. |

<details class="explain">
<summary>Show explanation</summary>

<!-- Freeze: crates/nova_menu/src/lib.rs OnEnter/OnExit(Interface) (clocks held,
     cursor freed); crates/nova_gameplay/src/freeze.rs `FreezeOwner::Interface`.
     HUD hides: crates/nova_hud/src/lib.rs. Chips marked read:
     crates/nova_hud/src/objective_stack.rs. -->

While the interface is open the game is frozen: the clocks stop, so combat, physics, AI and every projectile hold mid-frame, and a held trigger cannot fire into the frozen world. The mouse cursor is freed, so you can point, read and click at your own pace. It is a pause without a pause menu: <kbd>Esc</kbd> closes the interface instead of opening the menu.

Opening the interface hides the flight instruments behind it and marks every posted objective chip as read. The standing list is the `objectives` command, and `log` prints the comms and objective history.

The interface and the command shell share one freeze. Opening `:` over a pane, and closing it again, never runs a frame of the world and never lets a key reach flight or the pane on the way.

</details>

## The map

<!-- Contact kinds + codes: crates/nova_interface/src/map/contacts.rs
     (`MapContactKind`, `code_prefix`). Contact panel: map/app.rs
     `spawn_map_panel`. Blips, legend, selection, GOTO and its refusals:
     crates/nova_interface/src/map/scene.rs (`spawn_blip`, `refresh_map_legend`,
     `map_input`, `update_map_panel`). Footer and its live key hints:
     crates/nova_interface/src/pane.rs (`pane_footer`,
     `refresh_pane_input_hints`). Icons: crates/nova_interface/src/icons.rs.
     Keys: crates/nova_interface/src/bindings.rs. -->

The Map pane is a schematic 3D chart of local space: distance rings, a hub, and every contact as an icon - a chevron for a ship, a lump for an asteroid, a ringed disc for a planet, a diamond for an objective - tinted by what it is to you. Click one, or step through them with <kbd>[</kbd> / <kbd>]</kbd>, and the contact panel beside the chart gives its code, name, kind, range and bearing. The footer under the chart holds the legend of the icons on the chart now, **Reframe** with the keys the pane answers to, and a summary of the selection. The key hints follow your bindings.

| Label | Contact |
| --- | --- |
| `SELF` | Your own ship. |
| `ALLY-1` | A friendly ship. |
| `HOST-1` | A hostile ship, in the danger colour. |
| `OBJ-1` | A mission objective. |
| `NEU-1` | A ship on no side. |
| `PLN-1` | A planet. |
| `AST-1` | Terrain: an asteroid mass. Its code shows only while it is selected. |

| Key | Gamepad | Does |
| --- | --- | --- |
| <kbd>W</kbd> <kbd>A</kbd> <kbd>S</kbd> <kbd>D</kbd> | triggers, <kbd>X</kbd> and <kbd>B</kbd> | Move the view. |
| <kbd>Q</kbd> / <kbd>E</kbd>, <kbd>R</kbd> / <kbd>F</kbd> | D-pad | Turn and tilt. Right-drag looks; the wheel zooms. |
| <kbd>[</kbd> / <kbd>]</kbd> | bumpers | Select the previous or next contact. |
| <kbd>G</kbd> | <kbd>A</kbd> | **GOTO**: engage the autopilot toward the selected contact. |
| <kbd>T</kbd>, or click **Reframe** | left stick click | Reset the view. |

<details class="explain">
<summary>Show explanation</summary>

A typical panel reads `HOST-1`, `Raider`, `HOSTILE`, `Range 412 m`, `Bearing 214 mark +12` and `Hostile contact.` Selecting a contact re-centres the map once; <kbd>T</kbd> re-frames it. <kbd>G</kbd> engages the flight autopilot toward the selection and flashes `GOTO SET: Raider` on the panel note - the burn keeps flying after you close the interface, so the map is a real navigation console, not a picture of one. Your own ship refuses GOTO.

Docked, GOTO refuses while the game cannot measure the pair (`GOTO REFUSED: HELM FAULT`), refuses until you hold the [helm](../sections/docking/#what-the-clamp-holds) (`GOTO REFUSED: TAKE THE HELM`), and never flies to the ship you are docked to (`GOTO REFUSED: DOCKED PARTNER`).

</details>

## The ship

<figure class="figure">
    <!-- Capture: assets/wiki-interface-ship.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Screenshot</span
        >
        <span class="figure__placeholder-name"
            >assets/wiki-interface-ship.png</span
        >
        <span class="figure__placeholder-note"
            >The Ship pane: family-tinted schematic blocks with
            the bow arrow, section badges, and the section panel
            on a damaged section.</span
        >
    </div>
</figure>

<!-- Codes: crates/nova_interface/src/ship/sections.rs `code_prefix`. Family
     icons and tints: crates/nova_interface/src/icons.rs `SectionIconType`.
     Blocks do not encode status; badges and the panel do:
     crates/nova_interface/src/ship/scene.rs. Status words + thresholds:
     `ShipSectionView::status`. Panel: ship/app.rs, sections.rs. Repair action:
     `repair_section`; plate rule: `plan_plate_repair` in
     crates/nova_gameplay/src/inventory.rs. Weapon magazines refill on their
     own idle timer instead: crates/nova_ship/src/sections/ammo.rs
     `SectionReload::advance`. -->

The Ship pane is a schematic 3D viewer of your own hull: one block per section, a badge on each, and a section panel beside it. The footer under the view holds the section legend, **Fit** and **Reset** with the pane's live key hints, and a summary of the selected section. Sections carry short codes, stable for the whole session, and a family icon:

| Code | Section | Family |
| --- | --- | --- |
| `HULL-1` | Hull plating | Hull |
| `THR-1` | Thruster | Thruster |
| `CTL-1` | Controller | Controller |
| `PDC-1` | Turret | Weapon |
| `TRB-1` | Torpedo bay | Weapon |
| `RAIL-1` | Railgun | Weapon |
| `DOCK-1` | Docking clamp | Docking |

Select a section by clicking its badge, with <kbd>[</kbd> / <kbd>]</kbd>, or with **Prev** / **Next** in the panel. The panel shows its family icon, a condition bar, its status, what it does, its HP, ammunition and current bindings.

| Key | Button | Does |
| --- | --- | --- |
| <kbd>P</kbd> | **Repair** | Spend hull plates to restore the section's integrity, up to 20 HP per plate. |
| <kbd>B</kbd> | **Rebind** | [Rebind](#rebinding-a-section) the section's trigger. |
| <kbd>G</kbd> (gamepad <kbd>A</kbd>) | - | Overlay the structural mates: which sections hold which. |
| - | **Fit** | Frame the whole ship at its current angle. |
| <kbd>Q</kbd> / <kbd>E</kbd>, <kbd>R</kbd> / <kbd>F</kbd>, <kbd>T</kbd> | **Reset** | Turn and tilt the view; reset restores the opening angle, zoom and centre. |

<details class="explain">
<summary>Show explanation</summary>

The blocks are the shape of your ship - a fill in the section family's colour inside an outline per section, with a gap so neighbours read apart. An arrow past the foremost block points at the bow. Status lives on the badges and in the panel, not in the block colour: each badge carries a pip coloured by status (`nominal`, `degraded`, `critical`, `neutralized`), and the section you have selected spells out its code beside its badge. Only that one does - a development stress hull has two thousand sections, and a label on each is a wall of text with the ship somewhere behind it.

Repair is the panel's only action key. It answers on the note line with what happened, or why not, and it is instant: a success reads `repaired HULL-3: 2 hull plates, 80/100 HP`. It works anywhere, docked or not, and spends [hull plates](#the-inventory) from your ship: each plate restores up to 20 HP, and part of a plate's worth still costs a whole plate. A section at 55/100 takes 3 plates and stops at 100; the unused HP of the last plate is lost. With too few plates the repair uses what you have: 2 plates take 40/100 to 80/100.

Repair is off, and spends nothing, when the section has no integrity to restore, is destroyed, is at full integrity, or you carry no plates, checked in that order: `repair: HULL-3 has no integrity to restore`, `repair: HULL-3 is destroyed`, `repair: HULL-3 is at full integrity`, `repair: no hull plates`. A destroyed section is one at 0 HP, or one knocked out by a structural collapse with HP left; repair does not bring it back. Spent plates are not saved: they return when the scenario loads again.

There is no manual reload. A weapon's magazine refills on its own from your inventory - see [Magazines](../combat-weapons/#magazines) - and with no matching ammunition left in stock, an empty magazine stays empty until you restock it.

</details>

## Rebinding a section

<!-- Arm: crates/nova_interface/src/ship/scene.rs (`ship_rebind`, bindable
     sections only; prompt text in `update_ship_panel`). Capture one key or
     button, Esc cancels: ship/rebind.rs `apply_ship_rebind`. Refusal policy:
     `RebindSurface::ShipPanel` in crates/nova_input/src/registry.rs
     (`rebind_verdict`), against the LIVE table; the captured key replaces the
     desk half and keeps the pad half: `captured_binds`. The command shell
     will not open while a capture is armed: `open_command_shell` in
     crates/nova_menu/src/pause.rs; nor will TAB or M act:
     `toggle_interface` and `next_interface_pane` in
     crates/nova_interface/src/pane.rs. -->

Your weapon and thruster sections fire on rebindable inputs, and the Ship pane is where you rebind them:

1. Select a thruster, turret, torpedo bay or railgun and press <kbd>B</kbd> (or click **Rebind**).
2. The panel arms: `PRESS A KEY OR MOUSE BUTTON - ESC CANCELS`.
3. The next key or mouse button you press takes over that section's keyboard-and-mouse trigger: `Bound engine_port to K`. A controller trigger on the same section is left alone, so rebinding at the desk never costs you the pad.

A reserved flight control is refused on the spot - `Space is already bound to flight control: Main Drive` - and the capture stays armed for another try. <kbd>LMB</kbd> is refused too, with `Left Mouse stays the pointer`: it is the button you click the blips with. Several sections may share one input (one key can fire every tube). <kbd>Esc</kbd> backs out with `Rebind cancelled` and leaves you on the pane. While a capture is armed, <kbd>:</kbd> does not open the command shell, <kbd>Tab</kbd> is refused as the interface toggle and does not close the interface, and <kbd>M</kbd> binds like any other key instead of switching the pane.

## The inventory

<figure class="figure">
    <!-- Capture: assets/wiki-interface-inventory.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Screenshot</span
        >
        <span class="figure__placeholder-name"
            >assets/wiki-interface-inventory.png</span
        >
        <span class="figure__placeholder-note"
            >The Inventory pane while undocked: category filters,
            your ship's empty store and the item inspector. The
            partner column appears only while docked.</span
        >
    </div>
</figure>

<!-- Items and categories: crates/nova_gameplay/src/inventory.rs (`ItemType`,
     `ItemCategoryType`, `ShipInventory`, required by `SpaceshipRootMarker`).
     Pane: crates/nova_interface/src/inventory/app.rs `update_inventory_panel`
     (partner through `DockedShip` -> `DockingConnection`); filter order
     `FILTER_ORDER`. Open-world stock: crates/nova_authoring/src/base_content/
     scenarios/open_world.rs `player` and `derelict`. Ship pane repair spends
     hull plates: crates/nova_interface/src/ship/sections.rs `repair_section`.
     Hold: `HULL_SECTION_CARGO_G` and `ResolvedShipDesign::cargo_capacity_g`
     in crates/nova_scenario/src/objects/ship_design.rs.
     Take and Give: crates/nova_gameplay/src/inventory.rs `plan_item_transfer`;
     Jettison: `plan_item_jettison`; both applied by
     crates/nova_interface/src/inventory/app.rs
     `apply_inventory_action_commands`. Intake: crates/nova_ship/src/sections/
     cargo_intake_section.rs `run_cargo_intakes`; numbers from
     crates/nova_authoring/src/base_content/sections/cargo_intake.rs. Tag:
     crates/nova_hud/src/cargo_canister_chips.rs. -->

The Inventory pane lists what your ship carries in the left column and, while you are docked, what the docked ship carries in the right column. Undocked, the right column is blank. A ship that carries nothing reads `Inventory empty.` A ship's hold takes 100 kg per hull section, and your column's title shows your load against it, for example `Line Warship 3520 kg / 13200 kg`. In the open world your ship starts with 12 hull plates (10 kg each), 6,000 PDC rounds, 20 rail slugs and 12 torpedoes - 3,520 kg in all - with a Derelict Tender carrying 8 more hull plates moored 140 m ahead of your port collar. Fly forward and stop beside it to [dock](../sections/docking/). Other ships start as their scenario authors them; an AI raider or a tutorial drone carries no reserve of its own, so its magazines do not refill once spent. A [repair](#the-ship) spends hull plates, and a weapon's idle [reload](../combat-weapons/#magazines) spends matching ammunition. Nothing is saved: spent and moved items return to their starting counts when the scenario loads again. With cheats armed, [`item give`](../commands/#cheats) adds items to a ship's hold.

| Filter | Shows |
| --- | --- |
| **All** | Every item. |
| **Food** | Provisions carried for trade. |
| **Ammo** | Weapon magazine supply. |
| **Repair** | Material a repair consumes, such as hull plates. |
| **Raw** | Mined or salvaged bulk material. |
| **Parts** | Scavenged objects for a story, an objective or trade. |

Click an item to inspect it: the inspector shows its category, what it is, how many the selected ship carries and the weight of one item. A PDC round weighs 0.2 kg. While a Take, Give or Jettison is open, **Total weight** shows the weight of the quantity you chose: 2 hull plates read 20 kg however many you carry.

### Take and give

Docked, a click on an item opens a transfer form in the inspector at a quantity of 1:

| Click | Opens | When |
| --- | --- | --- |
| An item in your column | **Give** to the docked ship | Always while docked, for example to deliver cargo. |
| An item in the docked ship's column | **Take** from it | Only when the docked ship is neutralized or a lootable derelict. Taking from a live ship would be stealing. |

Set the quantity with the mouse wheel over the quantity row, the slider, the number field, or **All** for the whole stack. The slider hides when the stack holds one item. **Confirm** moves the items at once and the note line reads `Took 3 Hull plate from Derelict Tender` or `Gave 3 Hull plate to Derelict Tender`. A refused move changes nothing and keeps the form open so you can fix the quantity: `Refused: enter a quantity`, `Refused: quantity is zero`, `Refused: only 2 Hull plate in Derelict Tender`, `Refused: Derelict Tender has room for 30 kg more`, `Refused: Derelict Tender is not neutralized or lootable`, or `Refused: not docked`. A move never splits to fit: with room for 30 kg, a move of 5 hull plates is refused whole. There is no price. Items given to a ship that the open world streams away are gone with it.

### Jettison and pickup

A **cargo intake** is a 30 x 20 m hold mouth behind an accordion door. The open world's line warship carries one on its top deck, aft of the dorsal guns, door up.

Undocked, with a working intake, a click on an item in your column opens a **Jettison** form. **Confirm** takes the items out of your hold at once and packs them into the intake's pending canister, and the note line reads `Jettisoned 4 Hull plate`. More confirmed stacks merge into that pending canister, even in the same frame. Each hull plate weighs 10 kg; a canister holds at most 200 kg across all its stacks. The door folds open, and the canister leaves through it at 3 m/s relative to your ship. A refused jettison changes nothing: `Refused: undock to jettison`, `Refused: no working cargo intake`, `Refused: canister exceeds 200 kg`, `Refused: enter a quantity`, `Refused: quantity is zero`, or `Refused: only 2 Hull plate in Line Warship`. An intake holds one canister back until its door is fully open and no canister sits within 12.5 m of where it leaves.

To take a canister in, bring the intake's door to it:

| Canister | What the intake does |
| --- | --- |
| In front of the door and within 40 m of it | Opens the door. |
| Within 1 m of the door face, clear of the 22.2 x 15.3 m opening's edges by 0.5 m, and moving under 5 m/s relative to it, not away from it | Takes the whole canister into your hold once the door is fully open, before it touches the door. |
| Faster than 5 m/s relative to the door | Is not taken, and hits the door. |
| Touching any part of your ship | Is not taken. Once it drifts off, it is taken when it comes back under 5 m/s. |
| Heavier than your hold has room for | Stays out, whole. |
| Just jettisoned | Is taken back the moment it closes on the open door again. |

The door, a drop and a take each make a sound at the intake. With a canister within 200 m of your ship, the pickup sight draws a cross on your intake's face and a line to the canister, with no lock needed. A travel-locked canister in range takes the line first; otherwise the canister nearest an intake does. Fly until the line stands perpendicular to the intake cross, then close the gap slowly. The sight stays cyan while closing. The take sound and the canister going through the door confirm a take; the sight goes with the canister. An unavailable intake, or no canister within 200 m, draws no sight.

Within 100 m of your ship, each canister carries an amber tag that reads what it holds, such as `4 Hull plate`, or `Mixed cargo` for multiple item types, with its total mass in kg. The tag hides while the canister is off-screen. A canister has 20 HP; at zero it and its contents are destroyed, not picked up. Canisters can be designated with a travel lock but cannot enter the combat target slot. Canisters are not saved: they go when the scenario ends, and a jettisoned stack returns to your hold when the scenario loads again.

## The command shell

Press <kbd>:</kbd> for `NOVA COMMANDS`: a text prompt on the ship's CRT monitor, over the interface, flight, the menus and the editor. It reads the run, changes your settings and prints the flight log. The [Commands](../commands/) page has the whole catalog, and the monitor's own knobs and sounds.
