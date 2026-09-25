# UI app variants sketch, 2026-09-25

A playable Bevy sketch of the themed, click-based map, ship, inventory and
docked screens from the owner UI review in `TASK.md`. It is the design
artifact for that direction. It is not runtime UI and settles no open
decision.

Run it:

```text
cargo run --example ui_app_variants --features debug
```

Code: `examples/playable/ui_app_variants.rs`. Registered in `Cargo.toml`
beside `widget_zoo`.

## What it is

- The real app: `AppBuilder` with its own game plugins, an empty scenario,
  the mod UI themes, the shared `nova_ui` widget factories, and a live
  Phosphor/Hardware repaint without a rebuild.
- Example-local state only: `SketchView` (Map, Ship, Inventory, Docked),
  `SketchContext` (Undocked, Docked) and `SketchPart` (the selected ship
  component). Every control is a real themed button that drives its resource
  through `button_on_setting`, the path the game's Settings use.
- Sample data only. No scanner, ship, cargo, station or dock state is read.
  The Undocked/Docked control is a local flag, not a dock joint.
- No production crate changed. NOVA OS and its TAB binding are untouched.

## What each screen shows

- **Map**: a 3x3 sector chart with sector boundary lines and grid labels,
  10 km and 20 km range rings, the ship at the centre with a forward mark,
  and five named contacts (Tollgate Relay, K-7 Field, Heron, Marrow,
  Beacon B-12). A contacts list gives kind and range. No logs, no readout
  text.
- **Ship**: a block schematic of a Cargoa-like hull. Each component is a
  clickable button. The detail pane shows the selected component's status
  badge, integrity meter and three stats. Repair is drawn disabled.
- **Inventory**: an 8x4 slot hold with coloured stacks that span 1x1 to 2x2
  slots, a capacity meter and a manifest. Transfer and Sell are drawn
  disabled.
- **Docked**: undocked, a "no dock link" badge and five disabled service
  cards. Docked, the station name and berth, and the same five cards, still
  disabled and tagged "visual only".

## Frames

Captured under Xvfb with lavapipe by the harnessed walk. Desktop is
1600x900, narrow is 720x1000. The walk switches to Docked before it switches
to Hardware. So the Phosphor Map, Ship and Inventory frames are Undocked
("Dock to use this."), and every Hardware and narrow frame is Docked
("Visual only: no service runtime.").

| View | Phosphor, desktop | Hardware, desktop | Hardware, narrow |
| --- | --- | --- | --- |
| Map | `ui-app-variants/ui_app_variants-phosphor-map.png` | `ui-app-variants/ui_app_variants-hardware-map.png` | `ui-app-variants/ui_app_variants-narrow-map.png` |
| Ship | `ui-app-variants/ui_app_variants-phosphor-ship.png` | `ui-app-variants/ui_app_variants-hardware-ship.png` | `ui-app-variants/ui_app_variants-narrow-ship.png` |
| Inventory | `ui-app-variants/ui_app_variants-phosphor-inventory.png` | `ui-app-variants/ui_app_variants-hardware-inventory.png` | `ui-app-variants/ui_app_variants-narrow-inventory.png` |
| Docked | `ui-app-variants/ui_app_variants-phosphor-docked-undocked.png`, `ui-app-variants/ui_app_variants-phosphor-docked.png` | `ui-app-variants/ui_app_variants-hardware-docked.png` | `ui-app-variants/ui_app_variants-narrow-docked.png` |

## What the harnessed walk proves

`NOVA_AUTOPILOT=1` clicks every tab, the dock context and the theme through
synthesized pointer input on the named controls. After each change it
asserts:

- the view resource matches the clicked tab;
- exactly the view's panes exist, laid out and inside the window, and no
  other view's pane exists;
- each drawn action (Repair, Transfer, Sell, the five services) exists once
  and carries `InteractionDisabled`;
- the part detail follows the clicked component;
- the dock status line changes with the context;
- the theme flip keeps the same view body entity and repaints the station
  panel fill, so the flip is a repaint and not a rebuild;
- no node leaves the window or its parent's box, and no text is wider than
  its own box.

It repeats every view at the narrow size, then exits.

The parent check catches a flex row whose measured height is shorter than
what it draws. The first frames showed the Manifest note drawn below its
panel (`ui-app-variants/before-manifest-overflow.png`); the check failed on
that layout (`ui-app-variants/before-parent-check.log`). The actions now
stack in a column, and the check passes.

## Limits

- TAB does nothing here. `toggle_nova_os` opens NOVA OS only with a player
  ship (`crates/nova_os_ui/src/terminal/input.rs:53-80`), and the scenario
  is empty. The station UI entry key stays an open decision.
- The ship is a flat button schematic. The owner's open decision between the
  block wireframe and an editor-style 3D look is not answered by it.
- The map is 2D UI nodes, not the NOVA OS offscreen `Camera3d`. Chart labels
  have no collision avoidance: the 10 km ring crosses the Tollgate Relay
  label.
- Disabled buttons use the shared widget paint. In the Hardware frames a
  disabled button reads mainly by its dimmer text.
- Nothing scrolls. The narrow size fits every view; a shorter window would
  clip the lower pane.
- No station, inventory, persistence or service runtime, content or schema
  exists. Every number on screen is sample data.
