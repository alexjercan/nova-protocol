# UI app variants sketch, 2026-09-26

A playable Bevy sketch of themed, click-based map, ship and inventory screens
from the owner UI review in `TASK.md`. It is the design artifact for that
direction. It is not runtime UI and settles no open decision.

Run it:

```text
cargo run --example ui_app_variants --features debug
```

Code: `examples/playable/ui_app_variants.rs`. Registered in `Cargo.toml`
beside `widget_zoo`.

## What it is

- The real app: `AppBuilder` with the game plugins, a fixture scenario, the
  mod UI themes, the shared `nova_ui` widget factories, and a live
  Phosphor/Hardware repaint without a rebuild.
- The scenario puts a catalog picket (`block_picket`) under the player, with
  a raider (`block_gunship`), a hauler (`block_hauler`) and two rocks in
  range.
- The simulation is paused while the screens are up, which in this example is
  always. The view ease and the map camera run on real time; clicks, drag and
  wheel are pointer events, so they do not need the simulation clock.
- Player control is suspended while the screens are up, so the flight context
  is off and no key or mouse input reaches the ship.
- Map and Ship are 3D views. Each pane is its own camera drawing into an
  image. The contact and section models, map rings, framing, zoom limits,
  orbit gesture and center ease come from NOVA OS (`nova_os_ui` prelude).
  The palette, icons, blips, legends, selection and pane lifecycle belong to
  the example.
- Three mock contexts, picked in the top bar: Undocked, Station and Boarded.
  Station is a fixture market (`Mock station`); Boarded is the parked raider's
  hold. Nothing docks or boards: the context is an example-local flag, and a
  switch keeps the open view, its 3D scene, camera and orbit.
- Cargo, credits, prices, section condition and the repair bay are
  example-local fixture state (`SketchFixture`), from 1200 cr. Transactions
  change only that state: gameplay health and cargo are never written, and
  nothing persists.
- NOVA OS behavior and its TAB binding are unchanged. The sketch only adds
  public exports of the NOVA OS viewport helpers and zoom limits it uses.

## What each screen shows

- **Top bar and context line**, on every view: the top bar picks the view,
  context and theme. Under it, the context line names the station or the
  raider and shows the fixture credits and the last transaction's result, at
  one fixed height. It holds no control.
- **Map**: range rings on the orbit center, and one icon blip and code per
  contact (`SELF`, `ALLY-1`, `HOST-1`, `AST-1`, `AST-2`). Blips are
  example-local icons by body and stance: a ship arrow tinted own, ally or
  hostile, an asteroid and a planet. The legend lists only the kinds that are
  plotted, so this scenario shows own, ally and hostile ship and asteroid.
  The readout shows the selected contact: code, name, stance and range.
  Click another contact to ease the view onto it. Drag to orbit, turn the
  wheel to zoom. W/A/S/D move the camera across the plane relative to its
  heading, Space moves it up and Shift down. Reframe returns to the opening
  framing on the player. The map has no dock, trade or repair control.
- **Ship**: the picket's blocks, tinted by section kind, with a themed icon
  badge per section (weapon, thruster, controller, hull, docking) and a 3D
  bow arrow ahead of the foremost block. Prev, Next, Fit and Reset sit
  centered under the view, and the kind legend sits left of them, or on its
  own row under them when narrow. A themed detail panel sits to the right,
  and under the view when narrow: the selected section's icon, code, name,
  family, status, a condition bar and the repair slot. Click a badge
  to select it; the view eases onto it. Prev/Next step through the sections
  in order and wrap. Fit frames the whole hull and keeps the angles; Reset
  also restores the opening angles. The fixture seeds wear on `PDC-1` (45%)
  and `THR-1` (20%).
- **Repair**, in the ship panel: only at the station, and only while the
  fixture repair bay is on. It shows `Repair <code>  <price> cr` at 12 cr per
  missing point, and `Intact` at 100%. With the bay off it says so; undocked
  or boarded it says repair is at a station only. The slot has one fixed
  height in every state, so the scene keeps its size.
- **Inventory**: the picket's hold under a carried/capacity weight bar
  (`10.5 t / 12.0 t`), and category icon filters: All, Food, Ammo, Repair,
  Raw, Fuel and Parts. At the station the market shows as its own panel,
  with fixture prices and no weight bar. Boarded, the raider's hold shows
  with its own weight bar. The picket's hold takes the left half and the
  other store the right half in every context; undocked the right half
  stays empty, so a context switch does not resize the hold.
  Every line is one fixed-height row with a kind icon, name, quantity and
  mass.
- **Inspector**: click a row to select it. It shows the kind icon, name,
  category, unit mass, stock in that store, and the fixture price, or
  `No price` where no deal is allowed. The facts stay while a deal is open,
  and the deal's controls show below them.
- **Deals**: a row click opens the one deal the context allows on it at one
  unit: Buy on a market item and Sell on an own item at the station (paid),
  Loot on a raider item when boarded (free). Own cargo has no deal when
  boarded, and undocked the inspector only informs. The confirmation sets
  one quantity four ways: the wheel over the quantity row steps it, a
  slider runs from zero (an empty track) to the stock, a field takes a typed
  number, and All takes the whole stock. Over a stock of one the slider
  hides and the lines below close up. While the field holds no number it is
  marked red and the slider keeps the last quantity. It shows the price or
  `Free` and the hold after.
  Nothing changes until Confirm, which then closes the deal; the row must be
  clicked again to trade again, so a double click trades once. Confirm
  refuses an empty or zero quantity, more than the store holds, a full hold
  and too few credits, and the typed text stays until it is corrected.
  There is no Cancel: the selection is the deal, and another row or a
  context change replaces or closes it. A closed deal gives the keyboard
  back.
- **Sound**: example-local calls to the game's interface cues, one per
  activation. A row click that changes the selection or opens a deal, every
  tab, context, theme and filter button, map contacts, Reframe, ship
  sections, Prev, Next, Fit, Reset, the bay switch and All click
  (`menu_select`). A done deal or repair clicks, and a refused one only
  buzzes (`editor_deny`). A new quantity ticks (`ui_tick`). Text that is
  not a number is silent until Confirm.
- **Updates in place**: the context line, the inspector, the row selection
  and the filter marks are built once and change their nodes in place. A
  store's rows are rebuilt only when its stock, the filter or the context
  changes.

## Frames

Captured under Xvfb with lavapipe by the harnessed walk. Desktop is
1600x900, mid is 1120x820, narrow is 720x1000. Frames are in
`ui-app-variants/`, with the prefix `ui_app_variants-`.

| View | Phosphor, desktop | Hardware, desktop | Narrow |
| --- | --- | --- | --- |
| Map | `phosphor-map` (on `HOST-1`), `phosphor-map-recentered` (on `AST-2`) | `hardware-map-station` | `narrow-hardware-map-station`, `narrow-phosphor-map-undocked` |
| Ship | `phosphor-ship` (on `PDC-1`), `phosphor-ship-station` (after repair) | `hardware-ship-station`, `hardware-ship-boarded`, `mid-hardware-ship-station` | `narrow-hardware-ship-station`, `narrow-phosphor-ship-boarded`, `narrow-phosphor-ship-undocked` |
| Inventory | `phosphor-inventory` (inspector), `phosphor-inventory-station` (refused buy), `phosphor-inventory-boarded` (after loot) | `hardware-inventory-station`, `hardware-inventory-boarded` | `narrow-hardware-inventory-station`, `narrow-hardware-inventory-deal` (water deal), `short-hardware-inventory-deal` (720x760, scrolled to Confirm), `narrow-hardware-inventory-boarded`, `narrow-phosphor-inventory-boarded` |

## What the harnessed walk proves

`NOVA_AUTOPILOT=1` drives synthesized pointer and key input. It asserts:

- each view shows exactly its panes, laid out inside the window, with one
  live 3D scene; the pane and the scene keep their rectangles across views,
  contexts and repair states at each window size;
- a context switch keeps the same body, scene root, camera and orbit, the
  context line holds no control, and the map carries no dock, trade, repair
  or transfer control;
- a clicked contact or section sets the readout or detail, and the orbit
  center eases onto it by real-time ease steps;
- a drag turns the orbit by the NOVA OS gesture amount and moves the blips;
  the wheel zooms by the NOVA OS amount and stops at the NOVA OS ceiling;
  Reframe restores the opening angles, radius and center;
- W, D and Space move the map camera forward, right and up, and Shift moves it
  down, with its angles and zoom unchanged;
- the map legend lists exactly the plotted kinds, each blip shows its kind's
  icon and tint, and no projectile kind shows;
- Next and Prev step the ship selection, Fit frames the hull, and Reset
  restores the opening view; each section kind has its icon and tint, the
  panel preview follows the selection, and the bow arrow points to -Z ahead
  of the foremost block;
- the inspector shows the ore as `Nickel ore`, `Raw`, `1.0 t`,
  `4 t in Picket hold` and `No price`, with no deal undocked and one click
  cue; the Ammo
  filter leaves only the slugs, and All restores every row;
- selecting the ore, and at the station opening water, wheeling, sliding and
  typing its quantity and refused Confirms, keep every node under the
  inventory card and the context line: no row, inspector part or
  context-line text is respawned;
- the two store columns have equal widths in every context, the picket's
  column keeps its place across contexts, and each shown store fills its
  column;
- every hold shows the fixture's load text and a weight bar filled to
  carried over capacity, and every line is one 28 px row, in order;
- a row click opens exactly the deal its context allows, at one unit, and
  changes nothing until Confirm, with the item's facts above it;
- from 1200 cr at the station: a double click on the market water opens it
  once with one click cue; one wheel notch makes 2 t without scrolling the
  page; buying 2 water is refused (`Picket hold full`, hold after
  `12.5 t / 12.0 t`); one tonne fills a sixth of the slider; its empty left
  edge makes 0 t, which Confirm refuses (`quantity is zero`); the slider at
  60% makes 4 t; typed empty, `0` and
  `999` are refused (`enter a quantity`, `quantity is zero`,
  `only 6 t Water left`) and stay in the field, and only the empty field is
  marked; a typed `1` and a double-clicked Confirm buy 1 t once for 10 cr
  and close the deal with the field unfocused and the facts shown; a new
  click on the water opens it again at 1 t; All fills the slider and
  sells all 4 ore for 120 cr into the market; 2 pumps
  are refused (`need 1400 cr, have 1310 cr`);
- every control sounds exactly its cue, once: a click on a row, All, a
  done deal, each view tab, context, theme and filter button, map contact,
  Reframe, ship section, Prev, Next, Fit, Reset, bay switch and repair; a
  tick per new quantity; and only a buzz per refused deal or repair;
- boarded: the picket's own slugs open no deal and click once; the
  raider's one core hides the slider, keeps the field and All, and leaves no
  gap under the quantity row; its 60 slugs show the slider over 0 to 60;
  looting all 60 is free and moves them into the picket's hold;
- repair on `PDC-1` costs 660 cr and then reads `Intact`; repair on `THR-1`
  is refused (`need 960 cr, have 650 cr`); with the bay off the panel says so
  and the fixture refuses;
- in each context, the fixture refuses the deals and repairs of the other
  contexts (`not at a station`, `not boarded`, `repair needs a station`);
- every refusal leaves cargo, credits, condition and the bay unchanged;
- the theme flip repaints the 3D materials in place;
- at 1120 px the ship panel and inspector stay beside their views, and at
  720 px they stack; no node leaves the window or its parent's box;
- in a 720x760 window the ship panel starts below the window, and the wheel
  over the context line scrolls the page until the panel is in view;
- at 720 px a water deal shows Confirm under the facts inside the 350 px
  inspector, and in a 720x760 window the wheel over the context line
  scrolls the page until Confirm is in view;
- after the scenario loads, `Time<Virtual>` and `Time<Physics>` stay paused,
  no fixed step runs, and no contact moves, through every step, a NOVA OS
  open and close, and a lone Escape. The cursor stays free.
- player control stays suspended, and on no frame is the flight context live
  or a burn, RCS, thruster or turret input set, including while the map
  camera flies. W held under NOVA OS does not move the sketch camera.

Before the pause guard, the walk failed as soon as the scenario released
its load hold (`ui-app-variants/before-pause-guard.txt`).

## Limits

- The pause is an example-only guard in `Last` that pauses both clocks while
  the sketch root exists. It is not a `FreezeOwner` hold: the example has no
  menu or screen plugin to own one, so while the sketch is idle no named
  owner holds the freeze. A production screen must take a named hold
  instead.
- TAB opens NOVA OS over the sketch, and Escape closes it. The example has no
  pause menu, so a lone Escape does nothing. The station UI entry key stays
  an open decision.
- Input ownership is example-only: while the sketch root exists, a `Last`
  system calls `suspend_player_control` when control is not already
  suspended, which covers the resume the scenario loader queues. The map
  camera reads keys directly. A production screen needs a named input owner.
- No GOTO, route, trajectory or time control exists; the map only looks.
- Contact codes are minted by NOVA OS, so which rock is `AST-1` and which is
  `AST-2` can differ between runs.
- Section, category and map icons are example-local procedural masks, not
  meshes or shipped art. The map legend and blips come from example-local
  body markers, not from a NOVA OS contact kind.
- A rebuilt map scene opens framed on the player, as Reframe does. Only a
  pick of a different contact recenters, so a second click on the selected
  contact after Reframe does nothing.
- The page scrolls only when the window is shorter than the views need.
- No station, inventory, docking, boarding, persistence or service runtime,
  content or schema exists. The fixture's goods, prices, capacities, wear and
  repair bay are mock data, not a proposed economy or entitlement. Boarded
  loot is free because the fixture says so, not by a rule.
- A deal is one line at a time. There is no multi-select or split stack.
- Enter in the quantity field only leaves the field; it does not confirm.
