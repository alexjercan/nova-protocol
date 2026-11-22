# Interface

Every ship carries a second seat of control: the **interface**. Press <kbd>Tab</kbd> in flight and a full-screen panel opens over the frozen world with four panes: **Map**, **Ship**, **Inventory** and **Jobs**. Docked at a living station, a fifth pane, **Station**, sells ships, escorts and the station itself. Flight resumes the moment you close it.

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
     left alone under the command modal). In the Ship pane at a refit dock
     (`ShipRuntime.refits`) Tab toggles the catalogue instead
     (`InterfaceToggleType`, `toggle_ship_catalogue` in ship/catalogue.rs). Pane switch: `next_interface_pane`.
     Defaults: crates/nova_interface/src/bindings.rs `interface_bindings`
     (interface_toggle Tab / Right Thumb, interface_next_tab M / Y).
     Back-out owner (Escape, Start): crates/nova_interface/src/terminal/input.rs
     `close_surface_from_menu_keys`. First open shows Map, later opens the last
     pane: `InterfacePaneType` in pane.rs. Station tab gate: `show_station_tab`
     and `leave_station_pane` in pane.rs. -->

| You do | What happens |
| --- | --- |
| <kbd>Tab</kbd> (or click the right stick) | The interface opens on the Map pane, or on the pane you last used. It needs a live ship, and it does not open over the pause menu. |
| <kbd>Tab</kbd> again, or click **Close** in the card's head | Close it and return to flight. The **Close** button and the **Next** hint beside it show the keys or pad buttons you would press now. In the Ship pane at a refit dock, <kbd>Tab</kbd> opens and closes the catalogue instead, and **Close** shows <kbd>Esc</kbd>. |
| <kbd>Esc</kbd> or gamepad Start | Back out one level: leave the catalogue filter, close a part's card, close the catalogue, put away a held part, close the hull **Info**, cancel a Jobs confirm, clear a trade draft or cancel an armed station buy. With none of those open, close the interface. In a text field, <kbd>Esc</kbd> only leaves the field. |
| <kbd>M</kbd> (or gamepad <kbd>Y</kbd>) | Step to the next pane: Map, Ship, Inventory, Jobs, Station while you are docked at a living station, then Map again. While a text field has the caret, such as the Inventory quantity, <kbd>M</kbd> types instead. |
| Click **Map**, **Ship**, **Inventory**, **Jobs** or **Station** | Switch to that pane. The **Station** tab shows only while you are docked at a living station; undock and the interface opens on Ship instead. |
| <kbd>;</kbd> | Open the [command shell](../commands/) over the pane. Close it and you are back on the same pane, with its camera and selection as you left them. |

<details class="explain">
<summary>Show explanation</summary>

<!-- Freeze: crates/nova_menu/src/lib.rs OnEnter/OnExit(Interface) (clocks held,
     cursor freed); crates/nova_gameplay/src/freeze.rs `FreezeOwner::Interface`.
     HUD hides: crates/nova_hud/src/lib.rs. Chips marked read:
     crates/nova_hud/src/objective_stack.rs. -->

While the interface is open the game is frozen: the clocks stop, so combat, physics, AI and every projectile hold mid-frame, and a held trigger cannot fire into the frozen world. The mouse cursor is freed, so you can point, read and click at your own pace. It is a pause without a pause menu: <kbd>Esc</kbd> closes the interface instead of opening the menu.

Opening the interface hides the flight instruments behind it and marks every posted objective chip as read, except the **Take a job** chip of a new world, which stays until you accept your first job from a station board. The standing list is the `objectives` command, and `log` prints the comms and objective history.

The interface and the command shell share one freeze. Opening `;` over a pane, and closing it again, never runs a frame of the world and never lets a key reach flight or the pane on the way.

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
     Keys: crates/nova_interface/src/bindings.rs. Chart view:
     crates/nova_interface/src/map/chart.rs; what feeds the chart:
     crates/nova_world_base/src/chart.rs (`Chart` in nova_gameplay). -->

The Map pane is a schematic 3D chart of local space: distance rings, a hub, and every contact as an icon - a chevron for a ship, a lump for an asteroid, a ringed disc for a planet, a diamond for an objective - tinted by what it is to you. Click one, or step through them with <kbd>[</kbd> / <kbd>]</kbd>, and the contact panel beside the chart gives its code, name and kind, then its range and bearing. The name is the one the [target inset](../hud/#target-viewfinder) shows: a generated asteroid reads its kind, as `Ice asteroid`, and a planet its own name. A ship, station, rock or planet that wears a gold objective chip is one gold objective contact that keeps its own icon and name; its note shows the chip's text, then each job you hold that sends you there, such as `JOB-1 Bring 4 Iron ore: hand in.` A scenario objective shows its chip's text alone. Under them, the panel names your live GOTO destination over a **GOTO** button. An unselected contact has a faint frame; the selected one has a bright frame and always shows its label. The footer under the chart holds the legend of the icons on the chart now and **Reframe** with the keys the pane answers to. The key hints follow your bindings.

| Label | Contact |
| --- | --- |
| `SELF` | Your own ship. |
| `ALLY-1` | A friendly ship. |
| `HOST-1` | A hostile ship, in the danger colour. |
| `OBJ-1` | An objective, in gold: a marked ship, rock or planet, or a nav point. A marked station keeps its `STN-1` code. |
| `NEU-1` | A ship on no side. |
| `PLN-1` | A planet. |
| `AST-1` | Terrain: an asteroid mass. Its code shows only while it is selected. |

| Key | Gamepad | Does |
| --- | --- | --- |
| <kbd>W</kbd> <kbd>A</kbd> <kbd>S</kbd> <kbd>D</kbd> | triggers, <kbd>X</kbd> and <kbd>B</kbd> | Move the view. |
| <kbd>Q</kbd> / <kbd>E</kbd>, <kbd>R</kbd> / <kbd>F</kbd> | D-pad | Turn and tilt. Right-drag looks; the wheel zooms, and over the side panel scrolls it. |
| <kbd>[</kbd> / <kbd>]</kbd> | bumpers | Select the previous or next contact. |
| <kbd>G</kbd>, or click **GOTO** | <kbd>A</kbd> | **GOTO**: engage the autopilot toward the selected contact and travel-lock it. |
| <kbd>T</kbd>, or click **Reframe** | left stick click | Reset the view. |
| <kbd>C</kbd>, or click **Chart** | - | Switch between the local view and the chart view. |
| <kbd>Z</kbd> / <kbd>X</kbd> | - | In the chart only, step the focused sector layer up or down one Y cell. |

In a generated world, the chart view draws your chart at sector scale: a grid of sectors on the focused Y layer. <kbd>Z</kbd> / <kbd>X</kbd> step up or down one layer and clear a selection; the grid and unvisited outlines follow the focus. Clicking a find focuses its layer. **Reframe** returns to your ship's layer. A sector you entered is a textured tile, one layer per cluster type it holds (an asteroid speckle, a rock scatter, a ringed planet, cracked derelict slivers) or a faint grid when it is empty; a sector you never entered is an outline. Crossing into a sector posts its coordinate on the [HUD notice](../hud/#flight-readouts), with `- FIRST VISIT` while it is still an outline. The stations and wrecks your sensors found stand on their tiles. A mark's label shows a code: `STN-1` for a station, `WRK-1` for a wreck, numbered in the order you found them. A dead station fades, a station that refuses you docking because it is Enemy or hostile to you wears a red ring whatever its ports hold, and a station with open jobs adds their count to its code, as `STN-1 (3)`; a station with none stays `STN-1`. Click a sector's coordinate or any mark to select it; the view centres on its sector, keeping your zoom and angle, the selected sector gets an accent outline and the panel opens a **SECTOR** block: its coordinate and the route from your cell, its contents as icon rows, when you entered it, every find there with its state (`living`, `dead`, `yours`, `refusing` for the same hostility as the ring), the services, trade stock and stored items you saw at your last dock at each station there, and the jobs you hold that point there. A station you bought shows in the ally colour as `OWNED STATION`. Every job you hold is marked `JOB-1`, `JOB-2` and so on, in accept order, in the sector of the body it sends you to (the wreck, the hauler, the caller or the hull), or of the posting station for a procurement; the mark stays there once the job is done, and a handed-in job has no mark; click one to read what it asks, its reward and where to hand it in. Click the selected label or row again to clear the selection and keep the view, or press **Reframe** to clear it and frame the whole grid. While nothing is selected, the panel shows the **OBJECTIVES** block: what you have to do now. Its heading carries the focused layer on its right, as `Layer 6`. It lists each job you hold as a row with its kind icon, its `JOB-1` code, title, `have / need` (or `done`) and the code of the station it hands in at, then your start role's goal title over its next unmet milestone with `have / need`. `Empty` shows only when no held job, open milestone or reached goal remains. A met milestone leaves the list. Once you meet the last one, the block shows the play time it happened and `done`, which the save keeps. Reaching it ends nothing: the world plays on. The [Jobs](#jobs) pane shows the same milestone over your held jobs. Click a job's row to select its `JOB` mark. Under it, the **discovery log** lists your last twelve finds newest first as icon rows: the find's icon, its code, its sector and how long ago it happened (`just now`, `4 min ago`, `2 h ago`). Click a row to select that find. Under the log, the **STANCES** block lists each civilization you have moved or whose station you charted: its name, a stance badge and an `ADV 15%` tag for its seeded advancement, then compact status labels: `HOSTILE 0:12:40` while provoked, `CLEAR 1:42:10` for an Enemy due to thaw, `SEEDED` for an Enemy from the start, `FINE 1,090 cr @ STN-1` for the fine and a charted station that takes it, or `FINE NONE` when no charted station takes it, and `OFFENCES 2/3` and `MERITS 1/3` for the next stance step. Select a station and the block lists each Enemy neighbour of it as a row: its name, `ENEMY`, its fine and **Pay fine**. The button shows only while you are docked at that station. While your balance is short of the fine, the button is disabled and the fine reads red. While you are not docked there, one line under the rows says `DOCK STN-1` or `STN-1 REFUSED`. Paying moves the fine to that station and returns the civilization to Neutral. Each result plays one sound: the interface click for a result, the deny buzz for a refusal. The button is silent on press. The legend is an icon row naming the four cluster types, the station, wreck and job marks, and you. The chart view's footer drops the **Select** hint: a click picks a mark. A scene with no generated world has no chart: the **Chart** button is hidden and <kbd>C</kbd> plays the deny buzz.

<details class="explain">
<summary>Show explanation</summary>

A typical panel reads `HOST-1`, `Raider`, `HOSTILE`, then a **Range** row `412 m` and a **Bearing** row `214 mark +12`; your own ship has no Bearing row, and with nothing selected the panel shows none of this. In the chart view the same rows read a mark's **Cell**, when you **Entered** a sector, and a job's **Reward**. Selecting a contact re-centres the map once; <kbd>T</kbd> re-frames it. <kbd>G</kbd> or the **GOTO** button engages the flight autopilot toward the selection and sets your travel lock on it - the burn keeps flying after you close the interface, so the map is a real navigation console, not a picture of one. With nothing or your own ship selected, the button is off and <kbd>G</kbd> does nothing. <kbd>G</kbd> or **GOTO** on the contact you already fly to cancels the trip; on another contact it flies there instead. The destination line and the **GOTO** and **Cancel** buttons stay at the foot of the panel; the rest of the panel scrolls. **Cancel** ends the GOTO in the local view, even after its target has left the map, where the panel reads `GOTO LOST`; with no GOTO flying it is off. A Map cancel clears your travel lock only while the lock is on the GOTO target, so a lock you hold on anything else stays. A target beyond your sensors loses the travel lock at once and takes it back by itself the moment it is in [sight](../targeting-radar/#line-of-sight).

While the GOTO flies, the chart draws the [predicted path](../hud/#flight-readouts) from your ship to where the leg arrives. There is no line until the first forecast is ready. For a moving target the path ends where your ship meets it, not at its blip now. The target carries a `GOTO` tag, and the panel reads `GOTO HOST-1  412 m`. With no GOTO flying that line is blank. They follow the autopilot, not the selection: select another contact, or close and reopen the Map, and they still point at the destination. When the GOTO is cancelled or arrives, the path, tag and destination clear. On arrival the travel lock stays, ready for DOCK. A save keeps the GOTO and the travel lock, and a chart GOTO with its target.

In the chart view, select a station or a sector's coordinate and press <kbd>G</kbd> or **GOTO** to fly there, also to a station far outside your sensors. A station is flown to where it was when it left your window; a sector is flown to its centre. The panel reads `GOTO STN-4  128.40 km`, or the sector's coordinate and range, the mark's label wears the `GOTO` tag, and the HUD shows the [waypoint chip](../hud/#flight-readouts). When the station comes into your window, the trip becomes a GOTO of the station with your travel lock on it. A wreck or a job mark has no GOTO: with one selected, both buttons are hidden. The station you are docked to has no GOTO either. <kbd>G</kbd> on the flown mark, or **Cancel**, ends the trip. The chart draws no path: the local view and the HUD draw the predicted path once the forecast is ready. A sector's centre can be inside a rock or a planetoid; the trip then stops outside it. The autopilot does not fly round a station, so a sector whose centre is inside a station ends the trip when that station comes into your window. A GOTO the game cannot fly, such as a station with no known place, plays the deny buzz and changes nothing.

Docked while you do not fly the pair - the other ship or a living station holds the [helm](../sections/docking/#what-the-clamp-holds), or the game cannot measure the pair - GOTO is set and waits: the trip starts when you undock with <kbd>D</kbd>. Until then <kbd>G</kbd>, <kbd>Z</kbd> or **Cancel** drops it. Holding the helm, the GOTO flies the pair at once. GOTO never flies to the ship or station you are docked to: with it selected the **GOTO** button is hidden, and <kbd>G</kbd> plays the deny buzz and changes nothing: your travel lock and any GOTO already flying stay as they were.

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
     `ShipSectionView::status`. Panel: ship/app.rs, sections.rs. Repair start:
     `start_plate_repair`; plate rules: `plan_plate_repair`, `plan_plate_step`,
     `PLATE_REPAIR_SECS` in crates/nova_gameplay/src/inventory.rs. Jobs and
     their tick: crates/nova_ship/src/sections/repair_jobs.rs; job rows:
     crates/nova_interface/src/ship/repair_jobs.rs; field rebuild:
     crates/nova_interface/src/ship/rebuild.rs. SOS: stranded rule and bill in
     crates/nova_gameplay/src/rescue.rs; the call in
     crates/nova_ship/src/sections/rescue_call.rs; the answer in
     crates/nova_world_base/src/rescue.rs; the block in
     crates/nova_interface/src/ship/rescue.rs. Weapon magazines refill on their
     own idle timer instead: crates/nova_ship/src/sections/ammo.rs
     `SectionReload::advance`. -->

The Ship pane is a schematic 3D viewer of your own hull: one block per section, a badge on each, and a section panel beside it. The footer under the view holds the section legend on the left, one line of the keys that apply now in the middle and, on the right, **Catalogue** at a refit dock, **Fit** and **Reset**. The section panel is where the selection is named. Sections carry short codes, stable for the whole session, and a family icon:

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

The footer legend shows all seven family icons: Weapon, Thruster, Controller, Hull, Docking, Mining and Cargo Intake. The Dock and Helm keys are in **Settings > Controls > FLIGHT**; their defaults are <kbd>D</kbd> and <kbd>H</kbd>. Each mining beam has its own key, which you rebind here like a weapon's. A thruster fires on the Main Drive until you give it its own key here.

Select a section by clicking its badge or with <kbd>[</kbd> / <kbd>]</kbd>. The panel shows its family icon, code and name and condition bar over blocks: **Control** (its key and **Rebind**, for a section with a key), **Repair**, **Repairs**, **Distress**, **Lost** and **Salvage** (with **Delete**). Each block has a heading, its rows and its buttons at the bottom right. A block whose button cannot work here is hidden. A button short of plates, parts or credits is disabled, and the short number is red. A button blocked by another cause is disabled behind one tag word, such as `PORT`. A damaged section that repair can restore shows the **Repair** block; a section at full integrity, destroyed or with no integrity to restore does not.

| Key | Button | Does |
| --- | --- | --- |
| <kbd>P</kbd> | **Repair** | Start a repair job that fits the selected number of hull plates, one every 4 s of game time, up to 20 HP per plate. |
| - | number field, slider, **Max** | Choose how many hull plates the repair fits: type a count, drag the slider, or press **Max**. |
| - | **Cancel** | Stop a running job in the **Repairs** block. Plates already fitted stay fitted. |
| - | **Rebuild** | Rebuild a lost section: at once at a station that rebuilds, or as a field rebuild from the parts in your hold while undocked. |
| - | **Call SOS** | Call a station to rescue a stranded ship, in the **Distress** block. Shown only undocked in the open world while the ship cannot fly or dock. |
| <kbd>B</kbd> | **Rebind** | [Rebind](#rebinding-a-section) the section's key. Shown only for a section that has one. |
| <kbd>Delete</kbd> | **Delete** | [Delete](#deleting-a-section) the section, its parts back in your hold. It sits under **Salvage**, which lists what you get back. Shown only docked at a station that refits. |
| <kbd>G</kbd> (gamepad <kbd>A</kbd>) | - | Overlay the structural mates: which sections hold which. |
| - | **Fit** | Frame the whole ship at its current angle. |
| right-drag (gamepad D-pad), <kbd>T</kbd> | **Reset** | Turn and tilt the view, as the editor's camera looks; drag down past the hull to see its underside. The wheel zooms. Reset restores the opening angle, zoom and centre. |
| <kbd>Q</kbd> | - | Pick: take the part the section under the pointer was built from in hand, as the editor's pipette does. Over empty space, put the part in hand away. Docked at a Refit station only. |

<details class="explain">
<summary>Show explanation</summary>

The blocks are the shape of your ship - a fill in the section family's colour inside an outline per section, with a gap so neighbours read apart. An arrow past the foremost block points at the bow. Status lives on the badges and in the panel, not in the block colour: each badge carries a pip coloured by status (`nominal`, `degraded`, `critical`, `neutralized`), and the section you have selected spells out its code beside its badge. Only that one does - a development stress hull has two thousand sections, and a label on each is a wall of text with the ship somewhere behind it.

Repair is a job, not an instant fix. **Repair** queues the selected plates on the section and spends nothing yet; the job shows as a row in the **Repairs** block. Before you click, the form shows the integrity the plates restore and the seconds they take, such as `100/100 HP, 12 s`. The job then fits one plate every 4 s of game time: the plate leaves your hold and the section gains up to 20 HP. The open interface and the pause menu stop the game clock, so a job only moves while you fly. The **Repairs** block lists each running job with its code, a thin progress bar, the seconds left and **Cancel**. In flight, a chip under the RCS chip shows the first job, such as `REPAIR HULL-3 40%`, with `+1` for each other job, and a [HUD notice](../hud/#flight-readouts) says how each one ended: `HULL-3 REPAIRED: 2 PLATES`, `HULL-3 REPAIR STOPPED: NO PLATES` or `HULL-3 REPAIR STOPPED: SECTION LOST`. Jobs on different sections run side by side; a section runs one job at a time, and its form hides while it does. Repair works anywhere, docked or not, and part of a plate's worth still costs a whole plate. The count's `/ 3` is the most the repair can use: the plates the damage needs, or your free plates if that is fewer. The plates a running job has still to fit are not free. Selecting a section sets the count to that most. A section at 55/100 with 12 plates starts at 3, which stop at 100; the unused HP of the last plate is lost.

While the count field has focus, keys type into it rather than controlling the Ship pane; leave the field to use pane keys such as <kbd>P</kbd> and <kbd>B</kbd>. A damaged section with no free plates keeps its block: the count reads `/ 0` in red and **Repair** is disabled. A <kbd>P</kbd> press that starts no job buzzes and changes nothing. A destroyed section is one at 0 HP, or one knocked out by a structural collapse with HP left; repair does not bring it back. A saved Open World keeps running jobs, on the desktop build: a Load resumes each job where it stopped, fitted plates stay spent and the plates a job had not fitted are still in your hold. Any other scenario saves nothing: spent plates return when it loads again.

A section you lost is listed in the **Lost** block by its old code, such as `DOCK-1`, with its price docked or its time in the field, and **Rebuild**. Docked at a station that rebuilds, **Rebuild** is instant and priced as a [station rebuild](../../create/stations/#rebuild); a price your balance is short of is red and **Rebuild** is disabled. A lost part the station does not build has no **Rebuild**. Undocked, **Rebuild** starts a field rebuild: no station and no credits, only the section's recipe parts from your hold, which the job holds back and takes when its time is up, 5 s per part and at least 20 s of game time. A lost docking port is no longer the end of docking: carry its collar, actuator pack and panels, or [craft](../crafting/#fixing-the-ship) them, and rebuild it in the field. A part short in the hold shows as its count, such as `Docking collar 0 / 1`, in red, and **Rebuild** is disabled; with no live section left to hold the lost one, **Rebuild** is tagged `NO MOUNT`. A field rebuild checks both again at its end. While a field rebuild runs, the section shows in the **Repairs** block, not here. Docked at a hull that does not rebuild, the block is hidden: undock to rebuild in the field. The notice reads `DOCK-1 REBUILT` when the section is back.

Undocked in the open world, a ship with no working drive, flight computer or free docking port is stranded. A ship that lost only its guns still flies and docks, so it is not. The **Distress** block then tags why, such as `NO DRIVE` and `NO PORT`, and shows the **Fee** a rescue charges now, the **Rebuilds** it makes and its **Time**. The fee is 200 cr plus the rebuild price of each section it restores, with parts taken from your hold first and the rest at reference ask. A balance short of the bill pays all it has and is still rescued. A rescue rebuilds only lost sections. When your only drive, computer or port is still on the hull but does not work, **Call SOS** is tagged `NO FIX` and does nothing. **Call SOS** asks the nearest living station that is not hostile to you; when none answers, it is tagged `NO STATION`. The call runs 20 s of game time, so it moves only while you fly: the block shows the seconds left, its bar and **Cancel**, which is free, and in flight the chip under the RCS chip reads `SOS 14 S`. Dock anywhere first and the call ends for free (`SOS ENDED: DOCKED`). When the call is due, a station that would no longer let you dock refuses it (`SOS FAILED: NO ANSWER`): it died, its civilization turned hostile, or you shot at it. Nothing is paid, and the Distress block offers SOS again. Otherwise the station answers and takes the bill then (`SOS ANSWERED: PAID 1,400 CR`): it rebuilds the shortest chain of lost sections to a docking port, a drive and a flight computer, moves your ship beside it at rest, puts your escorts that are not docked in clear slots round you, and docks you. Weapons are not rebuilt. A station with no free berth for your ship leaves you beside it (`NO FREE BERTH: SHIP FIXED`): you keep what you paid for, so fly in and dock by yourself. A saved Open World keeps a running call: a Load resumes it.

There is no manual reload. A weapon's magazine refills on its own from your inventory - see [Magazines](../combat-weapons/#magazines) - and with no matching ammunition left in stock, an empty magazine stays empty until you restock it.

</details>

## Rebinding a section

<!-- Arm: crates/nova_interface/src/ship/scene.rs (`ship_rebind`, bindable
     sections only; prompt in `update_ship_rebind_prompt`). Capture one key or
     button, Esc cancels: ship/rebind.rs `apply_ship_rebind`. Refusal policy:
     `RebindSurface::ShipPanel` in crates/nova_input/src/registry.rs
     (`rebind_verdict`), against the LIVE table; the captured key replaces the
     half of its own device and keeps the other: `captured_binds`. The command shell
     will not open while a capture is armed: `open_command_shell` in
     crates/nova_menu/src/pause.rs; nor will TAB or M act:
     `toggle_interface` and `next_interface_pane` in
     crates/nova_interface/src/pane.rs. -->

Your weapon, thruster and mining beam sections answer rebindable keys, and the Ship pane is where you rebind them:

1. Select a thruster, turret, torpedo bay, railgun or mining beam and press <kbd>B</kbd> (or click **Rebind**). A gun, bay, railgun or beam with no key reads `UNBOUND` and **Rebind** gives it one. A thruster with no key of its own reads `Main Drive: W / Space` (your Main Drive keys) and fires with the Main Drive.
2. The **Rebind** button lights and a banner over the view reads `Bind PDC-1` beside the <kbd>Esc</kbd> key, and for a thruster the <kbd>Backspace</kbd> key.
3. The next key, mouse button or controller button you press takes over that device's half of the section's trigger. The other half is left alone, so a key never costs you the pad and a pad button never costs you the key. With both set, the **Control** row shows both, for example `J / Right Trigger 2`.

A key a flight control holds, such as <kbd>Space</kbd> for the Main Drive, is refused on the spot: the banner tags it `IN USE`, or `RESERVED` for a key no section may take. The capture stays armed for another try. <kbd>LMB</kbd> binds like any other button: the click that armed Rebind is not taken, and the next fresh click binds and reaches nothing under the pointer. Several sections may share one input (one key can fire every tube). <kbd>Esc</kbd> backs out and leaves you on the pane.

A thruster with its own key or button fires on that only: the Main Drive no longer fires it, from the keyboard or the controller, until you put it back. To put it back, press <kbd>B</kbd> and then <kbd>Backspace</kbd>, which clears both halves. The Main Drive inputs stay reserved, the controller's Right Trigger included, so the thruster's own input is any other free one. Every other common controller button already flies the ship or opens the interface, so a thruster's controller button is Right Trigger 2, which it then shares with your guns. While a capture is armed, <kbd>;</kbd> does not open the command shell, <kbd>Tab</kbd> is refused as the interface toggle and does not close the interface, and <kbd>M</kbd> binds like any other key instead of switching the pane. The key you bind does nothing else: binding <kbd>T</kbd> does not also reset the view.

## Swapping a section

<!-- Catalogue: crates/nova_interface/src/ship/catalogue.rs. Action:
     crates/nova_interface/src/ship/refit.rs. Family rule and build checks:
     crates/nova_ship/src/sections/design_sections.rs
     (`section_swap_family`, `swap_build_fault`). -->

Docked at a living station that offers refit, the Ship pane footer shows a **Catalogue** button beside **Fit** and **Reset**, with its <kbd>Tab</kbd> key chip. It or <kbd>Tab</kbd> opens the parts gallery over the pane, the same gallery the editor picks parts from: every part the game knows, priced from your hold and its stock. The category chips narrow the grid to one kind, and the **Available** chip beside the filter box narrows it to the parts you can place right now; click it again to see every part. A row of key chips under the grid names the keys. The wheel over the grid turns its pages. <kbd>Tab</kbd> or <kbd>Esc</kbd> closes the catalogue, as do switching tab and closing the interface; it opens again on the category, filter, **Available** chip and page you left. A part this station does not build, or that neither hold has the parts for, is dimmed and its picture greyed; **Place** is off for it, and pointing at the tile or **Place** shows one tag word, such as `NOT BUILT`. The price is the part's build recipe, with parts taken from your hold first and the rest bought from the station; it is red when your balance is short. A part's card lists each recipe item as an icon with how many of it you already have over how many it needs, such as `1/2`, red when you are short; point at the row to see where each one comes from, and when any were bought, the row's right side shows what they cost together. Items of a part the station does not build are dimmed, with no price and no hover. Select a section first, and the card of each part of the same kind and size gets a **Swap** button with the swap price, such as `Swap PDC-2, 100 cr`. Click it to fit the part. The new part stands where the old one stood, at full health, with the same code and key. The old part is gone; nothing is refunded. A swap that would split the ship, block a gun or drive, or leave it short of thrust or flight computers is refused, and the card shows why in one tag word, such as `Swap PDC-1  SPLITS`. A part of another kind has no **Swap**. In a saved Open World the new part stays across a Load.

## Adding a section

<!-- Ghost and click: crates/nova_interface/src/ship/ghost.rs. Free points,
     pose, fit and binding: crates/nova_ship/src/sections/section_add.rs. -->

Click a part in the catalogue, then **Place** on its card, to take that part in hand. Move the pointer over the ship: a ghost of the part snaps to the open point nearest the pointer on the section under it. Only a free point on the hull's surface, with room for the part, takes a ghost; a section inside the hull shows none. A green outline and its price beside the pointer mean it fits; a red outline and one tag word mean it does not, such as `BLOCKED` for a part that would block a gun or a drive. The section the ghost aims at turns solid, its badge hides, and the face the part mounts on is lit. <kbd>Q</kbd> over a section of your ship takes that section's part in hand instead, and <kbd>Q</kbd> over empty space puts the part away. <kbd>Esc</kbd> also puts it away and does nothing else: the interface stays open. While a part is in hand, the section panel shows it under **Placing** in place of the selection: its icon and name, its price, the parts from your hold and the parts bought, and the socket and roll. The footer shows Q/Esc Put away. When you put it away, the panel shows the selection again. With a part in hand <kbd>R</kbd> rolls it a quarter turn and <kbd>F</kbd> mates it by its next socket; the ghost, the fit and the price follow. The **Placing** block counts the parts your hold gives and the parts the click buys from the station at its ask. The docked station and the ships at its other docks show as dim outlines, and a part that would hit them is refused. Click to fit it. The new part spawns at full health under the next code of its kind, and the view eases onto it at the same zoom. A gun takes the key of your other guns of its kind, or none until you **Rebind** it; a drive is flown by the computer. The part stays in hand for the next one. <kbd>Tab</kbd> puts it away and opens the catalogue as you left it. With nothing in hand, a click on a section selects it. In a saved Open World the new part stays across a Load. The panel scrolls with the wheel while the pointer is over it.

## Deleting a section

<!-- Rule and handler: crates/nova_interface/src/ship/delete.rs. Build
     checks: crates/nova_ship/src/sections/design_sections.rs
     (`remove_build_fault`). Design and binding:
     crates/nova_scenario/src/objects/spaceship.rs (`remove_design_section`). -->

Docked at a living station that offers refit, select a section and press <kbd>Delete</kbd> (or click **Delete**) to take it off for good. Its parts go back in your hold as a strip of a wreck gives them: the recipe's strip share, less for a damaged section. **Salvage** lists them before you press it, over the **Delete** button, one tile per part such as **+1 Frame spar** and **+3 Section panel**, or `None`. After the delete the parts are in your hold and the plating closes over the hole. A rebuild does not list a deleted section, and in a saved Open World it stays gone across a Load. Away from a dock that refits, the **Salvage** block is hidden. **Delete** is disabled when the delete would split the ship: the sections that would come off the dock paint red, fill and outline, while that section is selected. It is disabled when the parts would overfill the hold: the block shows **Hold** free / need in red. It is disabled behind one tag word when it would take off your last working flight computer (`LAST CTL`), your last weapon (`LAST WPN`) or the port the dock holds you by (`PORT`), or leave the drives too weak (`THRUST`). The <kbd>Delete</kbd> key on a refused delete buzzes and changes nothing. A second flight computer is yours to take off: the ship flies on one. With a part in hand, **Delete** hides and the key does nothing; put the part away first.

## The station

<!-- Pane: crates/nova_interface/src/station/mod.rs. -->

Docked at a living station, the **Station** pane shows what it sells: **This station**, **Ships for sale** and **Escorts**, each a block of cards in a grid that fills the pane's width. A card shows the offer, its price and its buttons, with **Info** to look the hull over in the Ship pane's view. A button that cannot work here is hidden. A button your balance is short for is disabled and the price reads red, and one blocked by an unseen cause is disabled behind one tag word, such as `ALLIES ONLY`. **Buy station** asks twice: the first press reads `Buy station?` and puts **Cancel** beside a red **Confirm**. **Cancel**, <kbd>Esc</kbd>, any other button or a pane switch cancels. Above the ships for sale, **Your ships** lists the ships you own parked at that station, each with **Board**; it shows only while you have a ship parked there. The station's job board is on the [Jobs](#jobs) pane. A station that offers none of these and holds none of your ships reads `Empty`. Each result or refusal plays one sound: the interface click for a result, the deny buzz for a refusal. The buttons themselves are silent. Refit (rebuild, swap and add) stays in the Ship pane, beside the ship it changes. The wreck strip is in the Inventory pane, over the wreck's column. Goods, ammo and hull plates are not sold here: trade them in the [Inventory](#the-inventory) pane against a docked partner that trades (see [Buy and sell](#buy-and-sell)). The Station and [Jobs](#jobs) buttons have no key or gamepad button of their own, so click them.

### Buying a ship

<!-- List and action: crates/nova_interface/src/station/shipyard.rs. Price
     and parking: crates/nova_ship/src/sections/hull_buy.rs. 3D view:
     crates/nova_interface/src/station/hull_preview.rs. Swap and
     bindings: crates/nova_scenario/src/objects/station_hulls.rs. -->

Every ship a station sells is parked at one of its docking ports, and one port is always left free for you. Docked at a living station in an Open World that offers refit, open the Station pane: **Ships for sale** lists each parked ship. Each card shows the ship itself, turning slowly, then its name and length, and its price beside **Info** and **Buy**. Click **Buy** to take it: you pay the full price and take the controls of the parked ship. Your old ship stays yours. It stays parked at its port with its hold, and its hold shows in the Inventory pane as storage while you are docked at that station. It lists under **Your ships**, above the ships for sale, and its card carries **Board** and no price: Board swaps you back into it for free, and the ship you leave parks in the same way. The new ship has full health and an empty hold, and its keys are the default ones for its weapons, the same as a new game's. A price your credits cannot cover is refused. In a saved Open World your new ship and the parked ships stay across a Load.

**Info** on any card, a ship for sale or an escort for hire, opens that ship in the Ship pane to look at before you pay: turn it with the right mouse button, zoom with the wheel, and click a section to see its name and condition. Nothing can be changed there: no catalogue, no delete, no rebind, no repair. **Close** takes you back to the Station pane, and the Ship tab shows your own ship again.

### Hiring an escort

<!-- List and action: crates/nova_interface/src/station/escorts.rs. Price and
     fleet slots: crates/nova_ship/src/sections/escort_hire.rs. -->

Under **Ships for sale**, the same station lists its **Escorts** for hire: armed hulls with a crew, waiting outside the station. You can see them coast beside it, ahead of it and behind it. Each card shows the escort turning slowly, then its name, length and price, half again what the hull would cost. Click **Hire** and the escort joins your side where it waited, then comes to you. It flies on your starboard side, the second on your port side; a third hire is refused. The list's header shows both fleet slots, **STBD** and **PORT**, each with the name of the escort that holds it or `--` while it is free. An escort flies on its own drive: while you travel it takes its place 200 m off your side and 100 m behind, and matches your speed; while you stop or turn in place it holds where it is. It follows you across sectors and fights what attacks you. In a saved Open World your escorts stay across a Load. An escort that loses its last flight computer is out of your fleet and stays behind as a wreck.

### Buying a station

<!-- Block and action: crates/nova_interface/src/station/station_buy.rs. Price:
     crates/nova_ship/src/sections/station_buy.rs. -->

Docked at a living station in an Open World, the Station pane shows **This station**: its name and its price, twice what its sections, its hold and its credits are worth at ask. Click **Buy station** to make it yours; the credits go to its civilization. The station leaves its civilization: it joins your side, so its guns fire on your foes, and it is named **Owned station**. An owned station wears an `OWNED` tag. At a station sold to allies only, **Buy station** is greyed behind an `ALLIES ONLY` tag. An owned station lets you dock whatever side you are flagged, and its hold sells to you at the price it would pay. It earns nothing yet. In a saved Open World it stays yours across a Load. A price your credits cannot cover is refused.

## Jobs

<!-- Pane: crates/nova_interface/src/jobs.rs. Board and gate: crates/nova_gameplay/src/jobs.rs
     (`plan_job_hull_fit`, `station_job_offers`); starter board: crates/nova_world_base/src/jobs.rs
     (`post_station_boards`, `StarterBoardMarker`). -->

Every living generated station posts a board of three jobs. The board is fixed by the world seed, the station and your start: a board slot's offer stands 20 minutes of play, or 60 minutes for an escort offer (the pause menu, the interface and a Load all stop that clock), then the slot posts a new offer. An offer does not change while it stands, and a Load shows the same offers. Two boards never offer the same wreck or hull at once. A job you hold never expires, and a job you hand in is replaced at once. Which kinds a board leans to follows your start role: a miner's and a hauler's boards post procurements and supply runs, a scavenger's recoveries, a bounty hunter's bounties, a patrol's escorts and bounties, a pirate's bounties and recoveries. A trader that docks for a visit may post up to three extra procurements below the board's three jobs. An extra you do not take goes when one of the station's offers runs out, or after 20 minutes of play; one you take stays until you hand it in or lose it.

| job | what to do |
|---|---|
| `Bring 8 Iron ore` | bring that many units to the station that posted it. A station asks for what its sector lacks: raw material in an asteroid or rock sector, food and raw material near a planet, parts and plates in a derelict field. A board asks for raw material by the lot of four: 8 to 32 units |
| `Recover 2 Hull plate from <wreck>` | dock with the named wreck, take the stack aboard and undock with it. With no wreck near the station, the job puts a derelict of its own 6 to 16 km out when you accept |
| `Escort a hauler carrying 1,200 cr` | a hauler waits 1 km off the station and leaves for a mark 10 to 16 km out once you are undocked and within 1 km of it, cruising at 100 m/s; stay with it and see it there. Raiders wait three quarters of the way along: near your start one small raider about your own ship's size with one turret, up to three larger raiders where the station's civilization is more advanced. The hauler carries a spare pair of drives beside its flight computer, so one cut through its hull does not leave it adrift |
| `Answer a distress call` | a neutral drifting caller 6 to 16 km out: dock with the caller |
| `Neutralize <hull>` | neutralize the named hostile hull. With no hostile near the station, the job puts one raider in wait 8 to 16 km out when you accept |
| `Rescue a stranded crew with 3 Medkit` | a neutral drifting caller 6 to 16 km out asks for goods: dock with the caller with the whole stack in your hold and the dock takes it. With the hold short the dock takes nothing, the HUD reads `JOB-1: SHORT 2 MEDKIT` and the job stays open: dock again with the rest. The goods name the job: medkits are a rescue, food a resupply, ammunition a rearm, hull plates a repair, cheap ore or parts a supply run. Generated stations sell medkits, rations, plates and ammunition |

A reward is the sum of up to four parts, and a longer or more dangerous job pays more:

| part | what it pays |
|---|---|
| **Base** | the kind's own pay: nothing for a procurement, 100 cr for a recovery, 200 cr for an escort, a distress call or a supply run, 300 cr for a bounty. A more advanced civilization pays more, up to three times at the top. A posting fee of 50 to 250 cr is added |
| **Goods** | a procurement or a supply run pays its goods at ask plus half. A recovery pays half its stack at ask, at most 2,000 cr, and you keep the stack. An escort pays a quarter of the hauler's cargo, shown as **Cargo** |
| **Range** | 30 cr per km to the job's mark, and up to three times that at an advanced station: the leg of an escort, the way to a caller, or the distance from the station to the wreck or the hull when the job was posted, or to where the job puts its own |
| **Threat** | 300 cr for each weapon the job puts against you: for an escort the number of raiders times the turrets planned for one raider at the station's advancement (one to four; a raider you meet can mount a different number), or the live turrets, torpedo bays and lances of a bounty's hull, or the turrets planned for the one raider a bounty puts in wait. An unarmed hull pays none |

The **Jobs** pane shows two lists and an inspector. Each list scrolls. From a 1024 px wide window every row is one line: a title too long for its cell is cut and ends in `...`. In a narrower window a row moves its meter and buttons to a second line. The two lists always show, docked or not: **Available** over **Your jobs**. **Available** lists the jobs you can take now, under its column heads; with none to take it lists no row. Undocked, that is the job of a hull that hails you (see [A hail job](#a-hail-job)). Docked at a living station, it lists the station's board, less the jobs you hold, completed or lost and less any offer whose window has passed: each row is the kind's icon, the title, the **Range** in km, the **Threat** as a count of weapons, the reward in credits, a meter of the time left with its minutes beside it and **Accept**. At the issuing station, a row also carries an `X`: it asks `Delete?` beside the title and the 100 cr fee, red while your balance is short, and swaps the row's buttons for **Back** and **Delete**; **Delete** pays the station and posts a fresh offer in the slot at once in its place. A short balance or a posting that renewed since you opened the row refuses it and changes nothing. **Your jobs** lists every job you hold, from any station, docked or not: the icon, the title, a `have / need` meter (the units in your hold for a procurement or a recovery, done or not for the rest) and **Hand in**, plus an `X` to drop the job. `X` asks `Drop?` beside the title and replaces the row's actions with **Back** and **Drop**; **Back** keeps the job. **Drop** is always free, at the job's posting station or away: it gives the job up and despawns any bodies it spawned, with no payout, and the posting station's slot waits out its normal offer window before it posts a fresh one there, same as any offer you never held that expired. Docked at the hand-in station the row reads **HAND IN HERE**; select the row to see where any other job hands in. Under the list's column heads, one row shows your start role's milestone: the goal's title, its next unmet line and a `have / need` meter; the role's own count comes first, then `Stations owned`. Once you meet the goal, the row reads the goal's title and the play time.

Every job asks something of your hull: a procurement needs hold room for the stack, a distress call a docking section, a recovery or a supply job hold room for its stack and a docking section, an escort or a bounty a weapon. Your **starting station's** board posts only jobs your hull can do when a slot posts, so a fresh transport is offered no escort there until it carries a gun, and all three rows show. Every board disables **Accept** on a job your hull cannot do behind one tag for the part it lacks: `HOLD`, `NO DOCK` or `NO WEAPON`. A recovery or a bounty whose target is already gone shows `GONE`. **Hand in** shows only at the job's station; it is disabled while the job is not done or the hold is short, with the short count red, and tagged `NO FUNDS` while the station cannot pay. A living station that paid out regains credits while its sector is unloaded, up to the balance it started with: from nothing to full in 30 minutes of play. A station you own regains none, and neither does a station a scenario placed.

Click a row and the inspector reads the job: what to do, its target and where that is, the hand-in station, the reward with each part that pays under it (**Base**, **Goods** or **Cargo**, **Range** with the km, **Threat** with the weapon count; a reward of one part lists none), the time left or your progress, and **Show on Map**, which opens the Map's chart view on the job's mark. A done job shows **DONE** beside its title. A hand-in takes a procurement's goods out of your hold, pays the reward from the station's balance (`OnCreditsChanged` with cause `JobReward` on both sides) and moves the job to completed. It plays the payout sound in place of the click, and the balance in the title row counts up to the new figure in under a second with the amount it rises by beside it, such as `+1,460`, which goes about two seconds later. A bonus paid with the reward is part of that amount. A [HUD notice](../hud/#flight-readouts) reads `JOB-1 PAID 1,460 CR` once you close the interface. Each other result plays one sound, the interface click for a result and the deny buzz for a refusal; the balance and the lists show what changed. **Accept** and **Hand in** make no sound of their own.

Held jobs stay on the pane undocked, and the Map's chart view marks each in the sector of its mark, the wreck, the hauler, the caller or the hull, or the posting station for a procurement; its note says what is left to do (see [the map](#the-map)). In flight, the body a held job sends you to wears a gold objective chip with the job's code, what to do and the distance, such as `JOB-1 Answer  4.2 km`, or the supply job's verb, such as `JOB-2 Rescue`; off screen the chip pins to the edge and points at it. The moment a job's work is done, a [HUD notice](../hud/#flight-readouts) reads `JOB-1 done: hand in at STN-1`; the reward is paid at the hand-in. The chip moves to the station that pays it (`JOB-1 Hand in`), and jobs that hand in at one station share its chip (`JOB-1, JOB-2 Hand in`). A procurement has no chip. A body in a sector that is not streamed in has no chip; its chart mark shows where it is. A saved Open World keeps the jobs you hold, have done and have completed; a Load brings them back. A held job never expires. You lose a job if its hauler is disabled, neutralized or destroyed, if its wreck or caller is destroyed before the job is done, or if its station dies. A lost job is dropped for free: its slot waits out its normal offer window, same as an offer that expired unheld, before the station posts a fresh one, and a Load keeps the lost job off it.

### A hail job

<!-- Row, gate and inspector: crates/nova_interface/src/jobs.rs
     (`update_jobs_pane`, `JOB_HAIL_DECLINE`, `hail_gate`). Offer, accept,
     hand-in and pay rules: crates/nova_gameplay/src/encounter.rs
     (`plan_hail_offer`, HAIL_OFFER_SECONDS 300) and
     crates/nova_gameplay/src/jobs.rs (`plan_hail_accept`,
     `plan_hail_hand_in`, `plan_hail_pay`). Notices:
     crates/nova_hud/src/hud_notice.rs `post_hail_notice`. -->

A stranded hull that [hails](../ships/#encounters) you offers one job. **Available** lists it as its first row, in space and docked, over any station's board: the icon, the title, the **Range** to the caller when it hailed, the reward, a meter of the 5 minutes of play the offer stands and **Accept**, plus an `X` that declines it. The inspector names the caller as the target and as the hand-in. **Accept** works with no dock, and is disabled behind the same hull tags as a board job (`HOLD`, `NO DOCK`). A dock with the caller while the offer stands accepts the job too. The offer is never under **Your jobs**: the job moves there when you accept it.

A hail job is a distress call or a supply run and pays by the same parts as a board job, with the caller as the poster. Once accepted it lists under **Your jobs** with an `X` to drop it. The dock with the caller does the job and pays it at once, from no station's balance. The payout sound plays and a [HUD notice](../hud/#flight-readouts) reads `JOB-1 PAID 589 CR`. A supply dock with the hold short takes nothing and the job stays open, as for a board job. While you stay docked with the caller the row reads **HAND IN HERE** and carries **Hand in**, disabled with the count red while the hold is short: get the goods aboard and press it, with no second dock. You lose the job if its caller is destroyed. The Map's chart view marks the caller's sector for the job.

## Stripping a wreck

<!-- Block, timer and action: crates/nova_interface/src/inventory/strip/mod.rs.
     Rule: crates/nova_gameplay/src/inventory.rs `plan_hull_strip`,
     `plan_section_yield`. Order and removal:
     crates/nova_ship/src/sections/section_strip.rs. -->

Docked with a derelict wreck or a neutralized ship, the Inventory pane shows a strip block over the wreck's column: a **Strip** button and four meters. **Total** shows the whole strip and its time, **Step** the section coming off now, **Sections** the sections taken of all, and **Hold** your hold's mass of its capacity, with the mass of the parts still to come. Under them, one tile per part shows what the strip has recovered so far of what it plans to give, such as `2/23 Section panel`. The plan counts the sections still on the wreck at their current health, so it drops if they take damage. **Strip** first takes everything in the wreck's hold, everything you stored at a dead station, and all its credits into your ship, as **Take all** does (see [The inventory](#the-inventory)). If your hold has no room for all of it, **Strip** is disabled, the **Hold** meter turns red and nothing moves. Make room in your hold, then strip. A strip takes the wreck apart one section at a time, the section farthest from your port first, and the port you are docked through last. Each section takes 1 s per part of its build recipe, at least 3 s, and runs only while the game runs: close the interface and fly on docked, and the HUD chip over your ship reads `STRIP 40%`. As each section comes off, its parts go into your hold: each part of its recipe times its strip yield times its health, rounded down. When the port comes off, the wreck is gone and your ship floats free. Anything put into its hold during the strip comes off before the port, as much as fits, and the strip holds there until the rest fits. A hold without room holds the strip at that section until you make room: the HUD chip reads `STRIP HELD: HOLD FULL`, the **Total** meter reads `Held` and the **Hold** meter shows the mass the section needs after the `+`, such as `+70 kg`. Undock and the strip stops; what it took stays taken, when its sector streams away and back and across a Load. A strip is refused at a living station, while one is already running and at a hull that is not a wreck you may loot. At your own dead station, or while a ship you own is parked at a dead station, **Strip** is disabled behind a tag, `OWN STATION` or `OWN SHIP PARKED`.

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

<!-- Items and categories: crates/nova_gameplay/src/inventory.rs (`ItemDesign`,
     `ItemCategoryType`, `GameItems`, `ShipInventory`, required by
     `SpaceshipRootMarker`).
     Pane: crates/nova_interface/src/inventory/app.rs `update_inventory_panel`
     (partner through `DockedShip` -> `DockingConnection`); keys and their
     cues: `inventory_keys`; clipped description: `about_box`,
     `update_inventory_about`, `ABOUT_LINES`; filter order
     `FILTER_ORDER`. Open-world stock: crates/nova_authoring/src/base_content/
     scenarios/open_world.rs `player`; generated stock and credits:
     crates/nova_world_base/src/sector_ships.rs `ship_stock`, `ship_credits`,
     spawned lootable by crates/nova_world/src/streaming.rs
     `spawn_sector_ship`. Ship pane repair jobs fit
     hull plates: crates/nova_ship/src/sections/repair_jobs.rs
     `advance_ship_repair_jobs`.
     Hold: `HULL_SECTION_CARGO_G` and `ResolvedShipDesign::cargo_capacity_g`
     in crates/nova_scenario/src/objects/ship_design.rs.
     Take and Give: crates/nova_gameplay/src/inventory.rs `plan_item_transfer`;
     Take credits: `plan_credit_take`, applied by
     `apply_credit_take_commands`; Take all: `apply_take_all_commands`; Buy and Sell: `plan_item_trade` (a lawful market: `market_deals_in`), prices
     `ItemPrice::buy_cr` and `ItemPrice::sell_cr` of `ItemDesign::reference_price`, balance `ShipCredits` from
     `SpaceshipConfig::credits`; Jettison: `plan_item_jettison`; all but Take
     credits applied by crates/nova_interface/src/inventory/app.rs
     `apply_inventory_action_commands`; the action a row opens:
     `offered_action`, the Sell/Give switch: `draft_fits`. The intake's door, drop and pickup
     live on wiki/sections/cargo-intake.md. -->

The Inventory pane lists what your ship carries in the left column and, while you are docked, what the docked ship carries in the right column. Undocked, the right column is blank. A ship that carries nothing reads `Empty`. A ship's hold takes 100 kg per hull section. Under your ship's name, **Cargo** shows your load against the hold, for example `3520 kg / 13100 kg`. Your balance shows in the interface title row on every pane, beside the play clock, as the credit mark (a ring cut by a vertical bar) and the number, for example `2,000`. A job reward counts it up, with the amount it rises by beside it for a moment (see [Jobs](#jobs)); every other change shows at once. The docked ship's column shows its **Credits** the same way, and so do row prices and job rewards. In the open world your ship starts with its [start role](../getting-started/)'s kit of credits and cargo; a hull with point-defense mounts carries 3,000 PDC rounds in its hold. A generated ship, intact or a derelict wreck, starts with its own role-specific goods and a credit balance too (see [Generated ships](../ships/#generated-ships)): stop with one of your docking collars beside one of its docking ports to [dock](../sections/docking/), then Take what a neutralized or derelict one carries, or Take credits for its whole balance in one click. Other ships start as their scenario authors them; an AI raider or a tutorial drone carries no reserve of its own, so its magazines do not refill once spent. A [repair](#the-ship) fits hull plates over time, a field rebuild takes recipe parts, and a weapon's idle [reload](../combat-weapons/#magazines) spends matching ammunition. In a saved Open World, spent and moved items and credits persist for your ship and for every ship in the world: a Load brings them back from the last save, on the desktop build (see [Saved worlds](../getting-started/#saved-worlds)); within a session, a retired sector already keeps a generated ship's stock and credits as you left them. Any other scenario saves nothing: items and credits return to their starting values when it loads again. With cheats armed, [`item give`](../commands/#cheats) adds items to a ship's hold, and [`credits give` and `credits set`](../commands/#cheats) change its credits.

| Filter | Shows |
| --- | --- |
| **All** | Every item. |
| **Food** | Provisions carried for trade. |
| **Ammo** | Weapon magazine supply. |
| **Repair** | Material a repair or a rescue consumes: hull plates, repair kits and medkits. |
| **Raw** | Mined or salvaged bulk material. |
| **Parts** | Section build parts and scavenged objects for a story, an objective or trade. |
| **Contraband** | Goods a lawful station's market does not buy or sell. See [Buy and sell](#buy-and-sell). |

Click an item to inspect it, or step through the shown items with <kbd>[</kbd> / <kbd>]</kbd>, your column first; a step opens the same form a click does. The inspector shows its category, what it is, how many the selected ship carries and the weight of one item. A PDC round weighs 0.2 kg. While a Take, Give, Buy, Sell or Jettison is open, **Total weight** shows the weight of the quantity you chose: 2 hull plates read 20 kg however many you carry. The description keeps four lines, so the rows under it stay in place as you change items. A longer one is cut off with an `I Full text` cue: point at it, or press <kbd>I</kbd>, to read all of it. <kbd>I</kbd> again, or another item, closes it. You can rebind <kbd>I</kbd> as **Item Details** in **Settings > Controls > INVENTORY**.

### Take and give

Docked, a click on an item opens a transfer form in the inspector at a quantity of 1:

| Click | Opens | When |
| --- | --- | --- |
| An item in your column | **Sell** to the docked ship | When the docked ship trades. The **Sell** and **Give** buttons switch the form and keep the quantity, so you can still give cargo away. |
| An item in your column | **Give** to the docked ship | When the docked ship is neutralized or a lootable derelict, for example to deliver cargo. |
| An item in the docked ship's column | **Take** from it | Only when the docked ship is neutralized or a lootable derelict. Taking from a live ship would be stealing. |
| An item in the docked ship's column | **Buy** from it | When the docked ship trades: it is neither neutralized nor lootable. |

The form is a block titled by its action. Set the quantity on its amount row: type it in the field, drag the slider, turn the mouse wheel over the row, or press **Max** for the whole stack. The row runs from 1 to the stack, shown as `/ 8`. **Confirm**, at the bottom right of the form, moves the items at once; the rows and the column loads show the move. The partner is named in the column beside the form. **Confirm** is greyed while the move would be refused, and the number that blocks it turns red: the `/ 2` of the row when the source holds fewer, or the **Total weight** as the room left over the weight, such as `30 kg / 50 kg`. A move never splits to fit: with room for 30 kg, a move of 5 hull plates is refused whole. Take and Give have no price. Items you give a ship stay with it even if the open world streams it away, and a wreck you emptied stays empty when its sector streams back in.

The partner column's **Credits** row carries a **Take all** button while the partner is neutralized or lootable and holds a stack, a stored item or a credit. One click takes as much of every stack as your hold has room for, then as much of what you stored at a dead station, then the whole credit balance, and what stayed stays listed in the partner column. With no room for any of it, the button is disabled.

The **Credits** row also carries a **Take credits** button, shown only while the partner is neutralized or lootable and holds a credit above zero. One click moves its whole balance into your ship and leaves the partner at zero. The button is disabled while your own balance cannot hold it. Take credits has no form and no quantity: it always moves the whole balance in one click.

### Buy and sell

Every live ship trades: any docked ship that is neither neutralized nor lootable. It sells only what its hold carries and pays only from its own credits. A generated trader restocks toward the goods it spawned with by the play time its sector stood unloaded; an authored scenario's ship never restocks. A generated station that sells PDC rounds spawns with at least 4,000, so a bought-out one comes back to at least that. A generated wreck is lootable, not a trader.

The table lists each item's reference price. Every item prices by one
except the PDC round, which prices by its lot of 100: a trade of any other
count pays its share, rounded up for a Buy and down for a Sell.

| Item | Buy (ask) | Sell (bid) |
| --- | --- | --- |
| Hull plate | 40 cr | 30 cr |
| PDC round | 10 cr / 100 | 7 cr / 100 |
| Rail slug | 40 cr | 30 cr |
| Torpedo | 400 cr | 300 cr |
| Stone ore | 4 cr | 3 cr |
| Iron ore | 16 cr | 12 cr |
| Water ice | 12 cr | 9 cr |
| Carbon ore | 8 cr | 6 cr |
| Rations | 8 cr | 6 cr |
| Salvaged parts | 120 cr | 90 cr |
| Medkit | 60 cr | 45 cr |
| Unmarked cores | 300 cr | 240 cr |

A living generated station or ship computes its own prices from the reference when you trade: its stock against the stock it started with (scarce goods cost and pay more, a glut less), the type of the cluster it stands in (ore is cheap where asteroids are many and dear among planets), and what its civilization's ships do. While the partner trades, each row shows the **Price** of one item, what the partner pays for yours and asks for its own, with a **Deal** of the percent off the reference: green when it favours you (a bid over the reference, an ask under it), amber when not. The PDC round's row shows the price of its lot of 100, as `10 cr / 100`. Select an item and the inspector shows the **Reference**, **Pays** and **Asks** with the same percent, each as `4 cr / 3 cr` for a lot of one or `10 cr / 100 | 7 cr / 100` for the PDC round's lot of 100, and the **Scarcity**, **Cluster** and **Civilization** factors that made it, with their **Total**, marked `clamped` when the price hit its limit. Under the columns, a generated trader's **MARKET** strip names the three items it pays most over the reference for and the three it sells cheapest among what it holds, its restock gap (the item furthest under its target, a meter of stock against target, and the play minutes until it is full) and, while its civilization refuses you, a red **REFUSED** badge with the minutes left or none when only a settlement ends it. The interface title row shows the play clock while a world runs. A station or ship placed by a scenario deals at the reference price and shows no strip. A Buy pays the ask and a Sell earns the bid, each as its share of the item's lot, rounded up for a Buy and down for a Sell; a Sell that would earn 0 cr is refused, and otherwise selling back what you bought loses credits. At a station you [own](#buying-a-station), a Buy pays the bid instead, rounded up the same way. While the form is open it shows the **Price** of the quantity, for example `20 cr`. **Confirm** moves the items and the credits at once, and **Buy all** or **Sell all** before it confirms the whole stack in one click, disabled while that trade would be refused. A refused trade changes nothing and keeps the form open with **Confirm** greyed: the price turns red when the buyer is short of credits or the sale pays 0 cr, and the stock or the total weight turns red as for a Take. A trade never splits to fit the hold or the credits.

About half the civilizations of a generated world are **lawful**. The market of a living lawful station does not deal in contraband: it buys none and sells none. A Sell of the item there shows the price as a red `--` with **Confirm** disabled, the item's row and inspector show no price, and the **MARKET** strip does not name it. The station still lets you dock with the item in your hold and [stores](#store-and-withdraw) it. Sell it at a station that is not lawful. A station you own, a dead station and a station placed by a scenario are not lawful.

### Store and withdraw

Docked at a living station that offers storage, a **Storage** column lists what the station keeps for you, with **Stored** showing its load against the station's cap. From 1200 px wide your column, the station's and Storage share the width equally; in a narrower window Storage sits under the station's column, which keeps two thirds of the height. A column whose rows run past its height scrolls with the mouse wheel. Click an item in your column to open **Store**, or an item in the storage column to open **Withdraw**. At a station that also trades, your item opens **Sell**: click the **Store** button to switch the form. **Confirm** moves the items at once. No credits move. A move that does not fit greys **Confirm** and turns the total weight red as the room left over the weight, such as `70 kg / 80 kg`.

Your own ships are storage too. Each of your ships docked at the same station, or docked with you directly, shows as its own section of the storage column, headed by its name and its load against its own hold, for example `Escort  300 kg / 400 kg`. Click its item to **Withdraw** it into your hold. To store into it, click an item in your column, then the **To Escort** button. A move that does not fit is refused whole, with the total weight red as the room left over the weight. Docked directly with one of your ships, it does not trade and has no column of its own: its hold is the storage section.

### Quick moves

Double-click an item to move its whole stack at once: the double click confirms the form a click opens, with the stack as the quantity. Hold <kbd>Shift</kbd> while you double-click to move one item instead.

| Double-click | Does |
| --- | --- |
| An item in your column | The action the form shows: **Sell** at a trader, **Store** at a station that keeps items, **Give** to a lootable ship. |
| An item in the storage column, or in one of your ships' sections | **Withdraw** it into your hold. |
| An item in the docked ship's or station's column | **Buy** it, or **Take** it from a lootable ship. |

The Give, Sell, Store and To buttons stay as you set them from item to item, so after one click on **Store**, each double click stores. A quick move is refused exactly as **Confirm** would refuse it, with a deny sound, and nothing changes: double-clicking a stack the storage cannot take leaves the form open with its total weight red, such as `100 kg / 120 kg`. Jettison has no quick move. Two clicks count as a double click when they land on the same item within half a second.

### Using a repair kit

<!-- Button: crates/nova_interface/src/inventory/use_item.rs
     (`update_inventory_use_button`, `on_inventory_use_button`); the use
     itself: crates/nova_ship/src/sections/consumables.rs `use_consumables`. -->

Select a repair kit in your own column and the inspector shows **Use** at the right, docked or not; for any other item it is not there. It is the only way to use one: there is no flight key. One click takes one kit. **Use** is greyed while the use would be refused: none held (kits a running repair job holds back do not count), a kit already runs, or no live section is damaged. A refused use plays the deny cue. The interface holds the game clock, so the kit runs once you close it. See [Consumables](../crafting/#consumables).

### Jettison and pickup

Undocked, with a working [cargo intake](../sections/cargo-intake/), a click on an item in your column opens a **Jettison** form. **Confirm** takes the whole quantity out of your hold at once and packs it into canisters queued on the intake. A canister holds at most 1,000 kg across all its stacks, so a larger quantity fills several: 6 torpedoes of 150 kg, 50 rail slugs of 20 kg, 100 hull plates of 10 kg or 5,000 PDC rounds of 0.2 kg each. A confirmed stack first tops up the last waiting canister where whole items still fit, even in the same frame. **Confirm** is greyed while the quantity is more than you hold, with the `/ stock` red.

Once you close the interface, the intake's door folds open and the canisters leave through it one at a time. To take a canister back in, or one a [mining beam](../sections/mining-beam/) cut loose, bring the intake's door within 40 m of it and the open door pulls it in. A canister you dropped is pulled only after it has drifted out of that 40 m. [Cargo intake](../sections/cargo-intake/) has the door, the pickup sight, the drop and the canister rules.

## Crafting

The **Craft** view turns what your hold carries into ammunition, parts and repair kits. Open it with **Craft** at the right end of the Inventory pane's filter row, docked or in flight; it covers the Inventory pane. **Back** or <kbd>Esc</kbd> returns to the Inventory, and a pane switch or closing the interface closes it. Each tile is a recipe: its output's icon and name, how many one batch makes and where it runs, `SHIP` or `STATION`. A **SHIP** recipe runs anywhere, in flight too. A **STATION** recipe runs only while you are docked at a living station that offers Craft, and pays that station a fee for each batch. The base waystation offers Craft, and so does a generated station of a civilization at advancement 0.25 or more. A STATION recipe also needs a station at its advancement: docked at a station below it, the recipe has no tile, so rail slugs show only at a station of 0.5 or more, the railgun's advancement. The chips over the tiles filter them by category. A tile that cannot start one batch is greyed: a STATION recipe while you are undocked, a recipe your hold or credits cannot cover, and every tile while a job runs.

The tiles share the width evenly. Click a tile to open its card beside them. The card shows the whole job for the batch count: the output and how many the batches make, such as `PDC round x200`, each input as what your hold has over what the batches take, such as `12 / 2`, the **Fee** of a STATION recipe for the batches, such as `60 cr`, and the time the batches run. A count you are short of, and a fee over your credits, turns red. Under it, **Batches** is an amount row: type the count, drag the slider, turn the wheel over it or press **Max**, from 1 to every batch your hold and your credits cover, shown as `/ 6`. **Craft** at its bottom right starts the job, and is greyed when it cannot start. Starting takes nothing.

One craft job runs at a time, and only while you fly: close the interface and the HUD chip over your ship reads `CRAFT PDC ROUND 40%`. At the end of each batch its inputs leave your hold, its output goes in and the fee goes to the station. The card shows the running job in its **Running** block: the batch it is on, the seconds left, its bar and **Cancel**. The HUD notice reads `CRAFTED 200 PDC ROUND` when the job ends. A batch your hold or your credits cannot cover stops the job and keeps the batches already made: `CRAFT STOPPED: NO STONE ORE, 100 MADE`. Undock and a STATION job stops at once. **Cancel** ends the job and keeps the batches already made. A running job is saved with your ship.

[Crafting and consumables](../crafting/#recipes) lists the base recipes.

<!-- View: crates/nova_interface/src/craft.rs (`manage_craft_view`,
     `update_craft_view`, `apply_craft_start_commands`, `on_craft_cancel`).
     Esc: crates/nova_interface/src/terminal/input.rs `InterfaceLevels`. Checks and batch plan:
     crates/nova_gameplay/src/crafting.rs `plan_craft_start`,
     `craft_batches_max`, `plan_craft_batch`.
     Job: crates/nova_ship/src/sections/craft_job.rs
     `advance_ship_craft_jobs`. HUD: crates/nova_hud/src/hud_notice.rs
     `craft_job_end_line`, crates/nova_hud/src/flight_status.rs
     `drive_repair_chip`. Base recipes:
     crates/nova_authoring/src/base_content/recipes.rs `recipe_catalog`.
     Gate: `CRAFT_ADVANCEMENT` in crates/nova_world_base/src/station_layout.rs. -->

## The command shell

Press <kbd>;</kbd> for `NOVA COMMANDS`: a text prompt on the ship's CRT monitor, over the interface, flight, the menus and the editor. It reads the run, changes your settings and prints the flight log. The [Commands](../commands/) page has the whole catalog, and the monitor's own knobs and sounds.
