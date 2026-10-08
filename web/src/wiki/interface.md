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
     `map_input`, `update_map_panel`, `on_map_goto_button`). Route and GOTO
     tag: scene.rs `project_map_route`. Footer and its live key hints:
     crates/nova_interface/src/pane.rs (`pane_footer`,
     `refresh_pane_input_hints`). Icons: crates/nova_interface/src/icons.rs.
     Keys: crates/nova_interface/src/bindings.rs. -->

The Map pane is a schematic 3D chart of local space: distance rings, a hub, and every contact as an icon - a chevron for a ship, a lump for an asteroid, a ringed disc for a planet, a diamond for an objective - tinted by what it is to you. Click one, or step through them with <kbd>[</kbd> / <kbd>]</kbd>, and the contact panel beside the chart gives its code, name and kind, then its range and bearing. Under them, the panel names your live GOTO destination over a **GOTO** button. An unselected contact has a faint frame; the selected one has a bright frame and always shows its label. The footer under the chart holds the legend of the icons on the chart now and **Reframe** with the keys the pane answers to. The key hints follow your bindings.

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
| <kbd>G</kbd>, or click **GOTO** | <kbd>A</kbd> | **GOTO**: engage the autopilot toward the selected contact and travel-lock it. |
| <kbd>T</kbd>, or click **Reframe** | left stick click | Reset the view. |

<details class="explain">
<summary>Show explanation</summary>

A typical panel reads `HOST-1`, `Raider`, `HOSTILE`, `Range 412 m`, `Bearing 214 mark +12` and `Hostile contact.` Selecting a contact re-centres the map once; <kbd>T</kbd> re-frames it. <kbd>G</kbd> or the **GOTO** button engages the flight autopilot toward the selection, sets your travel lock on it and flashes `GOTO SET: Raider` on the panel note - the burn keeps flying after you close the interface, so the map is a real navigation console, not a picture of one. With nothing or your own ship selected, the button is off and <kbd>G</kbd> does nothing.

While the GOTO flies, the chart draws the [predicted path](../hud/#flight-readouts) from your ship up to 30 seconds ahead, then a dim straight guide from its end to the target. The guide is not a forecast. Before the first forecast, and while the path hides because the target changes course or spin or a gravity well moves, the guide runs from your ship. For a moving target the guide runs from the end of the path to where the target is now, so it can run back along the target's track. The target carries a `GOTO` tag, and the panel reads `GOTO HOST-1  412 m`. With no GOTO flying it reads `No GOTO set`. They follow the autopilot, not the selection: select another contact, or close and reopen the Map, and they still point at the destination. When the GOTO is cancelled or arrives, the path, guide, tag and destination clear; the travel lock stays.

Docked, GOTO refuses while the game cannot measure the pair (`GOTO REFUSED: HELM FAULT`), refuses until you hold the [helm](../sections/docking/#what-the-clamp-holds) (`GOTO REFUSED: TAKE THE HELM`), and never flies to the ship you are docked to (`GOTO REFUSED: DOCKED PARTNER`). A refusal changes only the note: your travel lock and any GOTO already flying stay as they were.

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

The Ship pane is a schematic 3D viewer of your own hull: one block per section, a badge on each, and a section panel beside it. The footer under the view holds the section legend on the left, the pane's live key hints in the middle and **Fit** and **Reset** on the right. The section panel is where the selection is named. Sections carry short codes, stable for the whole session, and a family icon:

| Code | Section | Family |
| --- | --- | --- |
| `HULL-1` | Hull plating | Hull |
| `THR-1` | Thruster | Thruster |
| `CTL-1` | Controller | Controller |
| `PDC-1` | Turret | Weapon |
| `TRB-1` | Torpedo bay | Weapon |
| `RAIL-1` | Railgun | Weapon |
| `DOCK-1` | Docking clamp | Docking |
| `MNG-1` | Mining beam | Mining |
| `CGO-1` | Cargo intake | Cargo Intake |

The footer legend shows all seven family icons: Weapon, Thruster, Controller, Hull, Docking, Mining and Cargo Intake. The Dock and Helm keys are in **Settings > Controls > FLIGHT**; their defaults are <kbd>D</kbd> and <kbd>H</kbd>. Each mining beam has its own key, which you rebind here like a weapon's.

Select a section by clicking its badge, with <kbd>[</kbd> / <kbd>]</kbd>, or with **Prev** / **Next** in the panel. The panel shows its family icon, condition bar, status and a short description of the section's role. Fact rows show **Integrity**, **Ammunition** for a weapon and **Control** for a section with a key. A damaged section that repair can restore also shows the repair form below them; a section at full integrity, destroyed or with no integrity to restore shows no form.

| Key | Button | Does |
| --- | --- | --- |
| <kbd>P</kbd> | **Repair** | Spend the selected number of hull plates to restore the section's integrity, up to 20 HP per plate. |
| - | number field, slider, **All** | Choose how many hull plates the repair spends: type a count, drag the slider, or press **All**. The slider hides when only one plate can be spent. |
| <kbd>B</kbd> | **Rebind** | [Rebind](#rebinding-a-section) the section's key. Shown only for a section that has one. |
| <kbd>G</kbd> (gamepad <kbd>A</kbd>) | - | Overlay the structural mates: which sections hold which. |
| - | **Fit** | Frame the whole ship at its current angle. |
| <kbd>Q</kbd> / <kbd>E</kbd>, <kbd>R</kbd> / <kbd>F</kbd>, <kbd>T</kbd> | **Reset** | Turn and tilt the view; tilt or right-drag down past the hull to see its underside. Reset restores the opening angle, zoom and centre. |

<details class="explain">
<summary>Show explanation</summary>

The blocks are the shape of your ship - a fill in the section family's colour inside an outline per section, with a gap so neighbours read apart. An arrow past the foremost block points at the bow. Status lives on the badges and in the panel, not in the block colour: each badge carries a pip coloured by status (`nominal`, `degraded`, `critical`, `neutralized`), and the section you have selected spells out its code beside its badge. Only that one does - a development stress hull has two thousand sections, and a label on each is a wall of text with the ship somewhere behind it.

Repair answers on the note line with what happened, or why not, and it is instant: a success reads `repaired HULL-3: 2 hull plates, 80/100 HP`. It works anywhere, docked or not, and spends [hull plates](#the-inventory) from your ship: each plate restores up to 20 HP, and part of a plate's worth still costs a whole plate. The form shows your live plate stock, the count and the integrity the repair will reach. Selecting a section sets the count to **All**: the plates the damage needs, or your whole stock if that is fewer. A section at 55/100 with 12 plates starts at 3, which stop at 100; the unused HP of the last plate is lost. With 2 plates All is 2, which take 40/100 to 80/100. <kbd>P</kbd> and **Repair** spend exactly the selected count.

While the count field has focus, keys type into it rather than controlling the Ship pane; leave the field to use pane keys such as <kbd>P</kbd> and <kbd>B</kbd>. **Repair** is disabled while the typed count is empty, not a whole number, zero, more than you carry or more than the damage needs. A damaged section with no plates in stock keeps its form, with **Repair** disabled and the refusal shown. The count is checked again against the live section and stock when you repair. Repair spends nothing when the section has no integrity to restore, is destroyed, is at full integrity, the count is zero or you carry no plates, the count is more than you carry, or the count is more than the damage needs, checked in that order: `repair: HULL-3 has no integrity to restore`, `repair: HULL-3 is destroyed`, `repair: HULL-3 is at full integrity`, `repair: no hull plates selected or in stock`, `repair: HULL-3 request exceeds live hull plate stock`, `repair: HULL-3 request exceeds live missing integrity`. A destroyed section is one at 0 HP, or one knocked out by a structural collapse with HP left; repair does not bring it back. Spent plates are not saved: they return when the scenario loads again.

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

Your weapon, thruster and mining beam sections answer rebindable keys, and the Ship pane is where you rebind them:

1. Select a thruster, turret, torpedo bay, railgun or mining beam and press <kbd>B</kbd> (or click **Rebind**). A section with no key offers no **Rebind**.
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
     (partner through `DockedShip` -> `DockingConnection`); keys and their
     cues: `inventory_keys`; clipped description: `about_box`,
     `update_inventory_about`, `ABOUT_LINES`; filter order
     `FILTER_ORDER`. Open-world stock: crates/nova_authoring/src/base_content/
     scenarios/open_world.rs `player`; generated stock and credits:
     crates/nova_world_base/src/sector_ships.rs `ship_stock`, `ship_credits`,
     spawned lootable by crates/nova_world/src/streaming.rs
     `spawn_sector_ship`. Ship pane repair spends
     hull plates: crates/nova_interface/src/ship/sections.rs `repair_section`.
     Hold: `HULL_SECTION_CARGO_G` and `ResolvedShipDesign::cargo_capacity_g`
     in crates/nova_scenario/src/objects/ship_design.rs.
     Take and Give: crates/nova_gameplay/src/inventory.rs `plan_item_transfer`;
     Take credits: `plan_credit_take`, applied by
     `apply_credit_take_commands`; Buy and Sell: `plan_item_trade`, prices
     `ItemType::ask_cr` and `ItemType::bid_cr`, balance `ShipCredits` from
     `SpaceshipConfig::credits`; Jettison: `plan_item_jettison`; all but Take
     credits applied by crates/nova_interface/src/inventory/app.rs
     `apply_inventory_action_commands`; the action a row opens:
     `offered_action`, the Sell/Give switch: `draft_fits`. The intake's door, drop and pickup
     live on wiki/sections/cargo-intake.md. -->

The Inventory pane lists what your ship carries in the left column and, while you are docked, what the docked ship carries in the right column. Undocked, the right column is blank. A ship that carries nothing reads `Inventory empty.` A ship's hold takes 100 kg per hull section. Under your ship's name, **Cargo** shows your load against the hold and **Credits** your balance, for example Cargo `3520 kg / 13100 kg` and Credits `2,000 cr`. The docked ship's column shows its **Credits** too. In the open world your ship starts with 12 hull plates (10 kg each), 6,000 PDC rounds, 20 rail slugs, 12 torpedoes - 3,520 kg in all - and 2,000 cr. A generated ship, intact or a derelict wreck, starts with its own role-specific goods and a credit balance too (see [Generated ships](../ships/#generated-ships)): stop with one of your docking collars beside one of its docking ports to [dock](../sections/docking/), then Take what a neutralized or derelict one carries, or Take credits for its whole balance in one click. Other ships start as their scenario authors them; an AI raider or a tutorial drone carries no reserve of its own, so its magazines do not refill once spent. A [repair](#the-ship) spends hull plates, and a weapon's idle [reload](../combat-weapons/#magazines) spends matching ammunition. Nothing is saved: spent and moved items and credits return to their starting values when the scenario loads again; a retired sector regenerates a generated ship's stock and credits. With cheats armed, [`item give`](../commands/#cheats) adds items to a ship's hold.

| Filter | Shows |
| --- | --- |
| **All** | Every item. |
| **Food** | Provisions carried for trade. |
| **Ammo** | Weapon magazine supply. |
| **Repair** | Material a repair consumes, such as hull plates. |
| **Raw** | Mined or salvaged bulk material. |
| **Parts** | Scavenged objects for a story, an objective or trade. |

Click an item to inspect it, or step through the shown items with <kbd>[</kbd> / <kbd>]</kbd>, your column first; a step opens the same form a click does. The inspector shows its category, what it is, how many the selected ship carries and the weight of one item. A PDC round weighs 0.2 kg. While a Take, Give, Buy, Sell or Jettison is open, **Total weight** shows the weight of the quantity you chose: 2 hull plates read 20 kg however many you carry. The description keeps four lines, so the rows under it stay in place as you change items. A longer one is cut off with an `I Full text` cue: point at it, or press <kbd>I</kbd>, to read all of it. <kbd>I</kbd> again, or another item, closes it. You can rebind <kbd>I</kbd> as **Item Details** in **Settings > Controls > INVENTORY**.

### Take and give

Docked, a click on an item opens a transfer form in the inspector at a quantity of 1:

| Click | Opens | When |
| --- | --- | --- |
| An item in your column | **Sell** to the docked ship | When the docked ship trades. The **Sell** and **Give** buttons switch the form and keep the quantity, so you can still give cargo away. |
| An item in your column | **Give** to the docked ship | When the docked ship is neutralized or a lootable derelict, for example to deliver cargo. |
| An item in the docked ship's column | **Take** from it | Only when the docked ship is neutralized or a lootable derelict. Taking from a live ship would be stealing. |
| An item in the docked ship's column | **Buy** from it | When the docked ship trades: it is neither neutralized nor lootable. |

Set the quantity with the mouse wheel over the quantity row, the slider, the number field, or **All** for the whole stack. The slider hides when the stack holds one item. **Confirm** moves the items at once and the note line reads `Took 3 Hull plate from <ship>` or `Gave 3 Hull plate to <ship>`, where `<ship>` is the docked ship's name. A refused move changes nothing and keeps the form open so you can fix the quantity: `Refused: enter a quantity`, `Refused: quantity is zero`, `Refused: only 2 Hull plate in <ship>`, `Refused: <ship> has room for 30 kg more`, `Refused: <ship> is not neutralized or lootable`, or `Refused: not docked`. A move never splits to fit: with room for 30 kg, a move of 5 hull plates is refused whole. Take and Give have no price. Items given to a ship that the open world streams away are gone with it, and a wreck you emptied has its generated stock again when its sector streams back in.

The partner column's header also carries a **Take credits** button, shown only while the partner is neutralized or lootable and holds a credit above zero. One click moves its whole balance into your ship and leaves the partner at zero, and the note line reads `Took 500 cr from <ship>`. Repeating it against the now-empty balance refuses with `Refused: <ship> holds no credits`, an intact, non-neutralized partner refuses with `Refused: <ship> is not neutralized or lootable`, and a take that would overflow your own balance refuses with `Refused: your ship cannot hold more credits` - every refusal changes nothing. Take credits has no form and no quantity: it always moves the whole balance in one click.

### Buy and sell

Every live ship trades: any docked ship that is neither neutralized nor lootable. It sells only what its hold carries and pays only from its own credits. Nothing restocks. A living generated ship trades its own seeded goods and credits; a generated wreck is lootable, not a trader.

| Item | Buy (ask) | Sell (bid) |
| --- | --- | --- |
| Hull plate | 40 cr | 30 cr |
| PDC round | 4 cr | 3 cr |
| Rail slug | 40 cr | 30 cr |
| Torpedo | 400 cr | 300 cr |
| Stone ore | 4 cr | 3 cr |
| Iron ore | 16 cr | 12 cr |
| Water ice | 12 cr | 9 cr |
| Carbon ore | 8 cr | 6 cr |
| Rations | 8 cr | 6 cr |
| Salvaged parts | 120 cr | 90 cr |

Prices are fixed per item and the same on every ship. A Buy pays the ask and a Sell earns the bid, so selling back what you bought loses credits. While the form is open it shows the price and your balance after, for example `Price 20 cr, you after: 1,980 cr`. **Confirm** moves the items and the credits at once, and the note line reads `Bought 5 Stone ore from Frame Tender for 20 cr` or `Sold 10 Stone ore to Frame Tender for 30 cr`. A refused trade changes nothing and keeps the form open: `Refused: enter a quantity`, `Refused: quantity is zero`, `Refused: only 2 Stone ore in Frame Tender`, `Refused: Line Warship has room for 30 kg more`, `Refused: Frame Tender has only 20 cr`, `Refused: Frame Tender cannot hold more credits`, `Refused: Frame Tender does not trade`, or `Refused: not docked`. A trade never splits to fit the hold or the credits.

### Jettison and pickup

Undocked, with a working [cargo intake](../sections/cargo-intake/), a click on an item in your column opens a **Jettison** form. **Confirm** takes the whole quantity out of your hold at once and packs it into canisters queued on the intake, and the note line reads `Jettisoned 4 Hull plate: 1 canister queued` with the number of canisters now waiting. A canister holds at most 200 kg across all its stacks, so a larger quantity fills several: one 150 kg torpedo, 10 rail slugs of 20 kg, 20 hull plates of 10 kg or 1000 PDC rounds of 0.2 kg each. A confirmed stack first tops up the last waiting canister where whole items still fit, even in the same frame. A refused jettison changes nothing: `Refused: undock to jettison`, `Refused: no working cargo intake`, `Refused: enter a quantity`, `Refused: quantity is zero`, or `Refused: only 2 Hull plate in Line Warship`.

Once you close the interface, the intake's door folds open and the canisters leave through it one at a time. To take a canister back in, or one a [mining beam](../sections/mining-beam/) cut loose, fly the intake's door onto it. [Cargo intake](../sections/cargo-intake/) has the door, the pickup sight, the drop and the canister rules.

## The command shell

Press <kbd>:</kbd> for `NOVA COMMANDS`: a text prompt on the ship's CRT monitor, over the interface, flight, the menus and the editor. It reads the run, changes your settings and prints the flight log. The [Commands](../commands/) page has the whole catalog, and the monitor's own knobs and sounds.
