# Upgrades: add a section at a free attachment point

- STATUS: OPEN
- PRIORITY: 75
- TAGS: v0.16.0, wizard

## User facts
- Epic `20261012-100008`. Decision 6, second rung: add a section at a free attachment point of the hull, station only, validated by post-build checks; an occupied point refuses and nothing changes.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Agent findings
- Attachment points are implicit in generated layouts (`crates/nova_world_base/src/ship_layout.rs`); authored hulls have none declared. Bindings come from the hull's sections.
- Worker report 2026-10-14. Code: the rule is `crates/nova_ship/src/sections/section_add.rs` (`free_link_points`, `section_add_fit`, `add_build_fault`, `added_section_binds`, `added_section_id`, `plan_section_add`); the socket choice is `natural_link_point` in `link_points.rs`, shared with the editor's `snap.rs`. The spawn is the `add_design_section` observer in `crates/nova_scenario/src/objects/spaceship.rs`; the Ship pane list is `crates/nova_interface/src/ship/refit.rs`.
- Proof: `examples/systems/system_station_add.rs`. Base round: `block_workship` with no bindings; the add at the taken socket `HULL-1 positive_x` is refused (note `add: HULL-1 positive_x is taken`, design, credits and hold unchanged); the PDC is added through the pane at `HULL-1 negative_x` as `PDC-1` for 100 cr with an empty binding; it survives freeze and thaw. Generated round: an Armored hull (world seed 20261014, hull seed 3) gets a PDC at `HULL-1 positive_x` as `PDC-3` with the sibling turrets' `Mouse Left`; it survives freeze and thaw. Each add asserts the pose, no build fault on the live ship, fresh health, `OnAdded` fields and causes `Add`/`Add`. Run under Xvfb and lavapipe at 1920x1080: `autopilot: cycle complete, no panic (t=18.4s)`. Unit tests: six in `section_add.rs`.
- Ship side panel scrolls now (`Overflow::scroll_y`, wheel over the panel scrolls it and does not zoom the view; `scroll_ship_panel` in `crates/nova_interface/src/ship/app.rs`). This also fixes the swap task's clip gap for the rebuild and swap lists. The Map pane keeps a clipped panel.
- Gaps: the Add list sits under the Swap list, so on a hull section it is long (nine fitting parts) and needs a scroll. The editor's `snap.rs` still folds its own overlap refusal; a shared fit check with `section_add_fit` is follow-up debt. No roll or socket cycling at a station.
- Design 2026-10-14 (slice 3, editor-style placement in the Ship view). End state: the Ship panel shows one station catalogue in place of the Swap list and the socket-code Add list. A row shows a part, its price or payment refusal, a Place button and, when the part can replace the selected section, a Swap button. Place puts the part in hand. A ghost block follows the pointer over the schematic and snaps to the free socket nearest the pointer on the block under it. A click adds the part when the ghost is valid; a refused ghost click changes nothing and shows the refusal in the note line. The refusal or the price shows in a callout beside the ghost. The view look stays: same blocks, outlines, badges and camera.
  - Dies: `ShipAddPoints`, `AddPoint`, `on_ship_add_point_button`, `ShipRuntime::add_point`, the `ShipAddPoint-*` and `ShipAdd-*` buttons, `ShipSwapList`/`Header`/`Rows`, `update_ship_swap_list`, `update_ship_add_list`, `AddFitCache`, the editor's private `nearest_socket`. Socket ids leave the player text (`add_refusal_text`, the add note).
  - Catalogue scope: a part is listed when it is not `hide_in_editor` and the player's hold plus the station's stock hold every part of its recipe (`plan_recipe_payment` gives no `MissingParts`). No new content field. Credits do not filter: an unaffordable part shows its credit refusal.
  - Shared in `nova_ship` (`link_points.rs`): `link_point_pose(position, rotation, &LinkPoint) -> (Vec3, Vec3)` (socket position and outward normal in the pose's space) and `nearest_link_point(position, rotation, &[LinkPoint], candidates, hit) -> Option<usize>`. The editor's `snap::solve` and `section_add_fit` both use them. `section_add.rs` gains `section_add_pose(design, section, point, offered) -> Result<(Vec3, Quat), SectionAddFaultType>` (free point, natural socket, roll 0); `section_add_fit` calls it first, then the overlap and post-build checks. The ghost needs the pose of a refused add too, which `section_add_fit` does not return.
  - New in `nova_interface/src/ship/`: `catalogue.rs` (`ShipCatalogue`, `ShipCatalogueHeader`, `ShipCatalogueRows(Vec<CatalogueRow>)`, `spawn_ship_catalogue`, `update_ship_catalogue`, `on_ship_place_button`, buttons `ShipPlace-{part}` and `ShipSwap-{part}`) and `ghost.rs` (`ShipPlacement { section, point, prototype, pose, verdict: Result<String, String> }` in resource `ShipPlacementPreview`, `pick_ship_block(Ray3d, &[ShipSectionView]) -> Option<(Entity, Vec3)>`, `update_ship_placement`, `sync_ship_ghost`, components `ShipGhostBlock` and `ShipGhostCallout`, viewport observers `on_ship_viewport_pointer` (Over/Move set, Out clears) and `on_ship_viewport_click`, and `pub fn ship_view_window_px(&World, Vec3) -> Option<Vec2>` for driven runs). `ShipRuntime` gains `held: Option<String>` and `pointer: Option<Vec2>`.
  - Click rules: with a part in hand, a primary click anywhere on the viewport (a badge too) places it; badges do not select while a part is in hand. With no part in hand, a click on a block (not on a badge) selects its section. Place on the held row puts the part down. A hull buy, closing the pane or undocking puts it down.
  - Before: `update_ship_add_list` -> `free_link_points` -> point buttons -> `section_add_fit` per catalogue part -> `plan_add` -> Add button -> `SectionAddCommand` -> `apply_ship_add_commands`. `update_ship_swap_list` -> `swap_offers` -> `plan_swap` -> Swap button -> `SectionSwapCommand`.
  - After: `update_ship_catalogue` -> `plan_recipe_payment` filter -> row (Place, Swap if in `swap_offers` of the selection) -> `runtime.held`. `on_ship_viewport_pointer` -> `runtime.pointer`. `update_ship_placement` -> `viewport_to_world` -> `pick_ship_block` -> `free_link_points` -> `nearest_link_point` -> `section_add_pose` (ghost pose) -> `section_add_fit` (cached per section, point and part) -> `plan_add` -> `ShipPlacementPreview`. `sync_ship_ghost` -> ghost block (family fill, Nominal or Danger outline) and callout. `on_ship_viewport_click` -> `SectionAddCommand` -> `apply_ship_add_commands` (unchanged rule: recipe price, family rule, post-build checks, `OnAdded`). Swap: row Swap -> `SectionSwapCommand` -> `apply_ship_swap_commands` (unchanged, `OnSwapped`).
  - Proof: `system_station_add` holds the PDC, moves the pointer to the free socket's window pixel (`ship_view_window_px`), asserts the callout price and a valid preview, clicks, and asserts the add (pose, code, binding, `OnAdded`); the refusal at a taken point stays a command beat. `system_station_swap` swaps through `ShipSwap-{part}` in the catalogue. The sandbox walk places its part through the ghost. Unit test: `pick_ship_block` takes the nearest block a ray enters. Fail loudly: a click with a preview whose section or socket is gone panics in `apply_ship_add_commands` as now.

## Agent decision
Owner told the orchestrator to work the list without a stop at the gate. The agent chose the marked options and implemented them (2026-10-14).

- Attachment points. Survey: every section carries `LinkPoint` sockets with a position and an outward normal (`crates/nova_ship/src/sections/link_points.rs`); `snap_placement` already turns a target socket plus a part socket into a pose, and the editor builds with it (`crates/nova_editor/src/snap.rs`). That is enough data to place a part.
  - (A, chosen) A free attachment point is a socket of a live section that no section of the design (live or lost) mates. No new content. Consequence: base hulls, generated hulls and mod hulls all have points with no authoring, and a lost section's socket stays reserved for its rebuild.
  - (B) Authored attachment points per hull design, linted for overlap. Consequence: new content on nine base hulls and every mod hull; generated hulls would need the generator to emit them.
  - (C) Generated hulls only. Consequence: the player's base start hull could never grow.
- Placement. The player picks a free point on the selected section, then a part. The part mates the point with the socket that faces most nearly opposite (it keeps the orientation it was drawn in, the editor's default), roll 0. No roll or socket cycling at a station.
- Fit, in order: the point is not occupied; the part has a socket; its collider overlaps no design section it does not mate (`section_colliders_overlap`, the lint and editor rule); then the generator checks on the live ship, refused only for a fault the add makes: one socket graph (also catches a socket with two suitors), clear exit lanes (`placement_blocks_an_exit`), the thrust floor and the flight computer count. Role, size ceiling and symmetry are not checked, as for swap. A refusal changes nothing.
- Shared build check: `SwapBuildFaultType` and `SwapPlacedSection` become `RefitBuildFaultType` and `RefitPlacedSection`, used by swap and add.
- New section id: the greatest design id of its class followed by `+` (or `added_<class>` when the class is absent), with more `+` until unique. Codes count a class by id, so the new section takes the next code and no live code moves.
  - Rejected: `added_<n>`. Consequence: it sorts before most ids and would take a code a live section already holds.
- Binding:
  - (A, chosen) The new section takes the binding of the first live section of its kind (by id) that has one. With none: a weapon gets an empty binding that Rebind (B) sets; a thruster stays unbound so the flight computer drives it; a mining emitter takes the first of V, B, N, C that no emitter holds, and the add refuses when all four are held.
  - (B) Always unbound. Consequence: the Ship pane cannot rebind a section with no binding component, and a player mining emitter must have a key (the spawn panics).
  - (C) The kind's default key (LMB, F, R, Space). Consequence: a second copy of the editor's default table, and a bound drive leaves flight-computer control.
- Price: the new part's full recipe through `plan_recipe_payment`. Station only, Refit service.
- Event: `OnAdded` (`onadded`) beside `OnSwapped`: `id`, `section` (new code), `part`, `at` (code of the section it mounts on), `point` (socket id), `parts`, `price_cr`. Credit cause `Add`.
- UI: the Ship pane's Add list under the Swap list: point buttons for the selected section (first free point picked by default), then every catalogue part that fits there with its price or payment refusal and an Add button.

- Orchestrator close 2026-10-14: landed in 85d7d130b. A free point is a socket of a live section that no design section mates (no authored content; a lost section's socket stays reserved). Fit order: free point, opposite-facing socket at roll 0, no overlap, delta-only post-build check (`section_add.rs`; the swap and add checks renamed Refit*). The new section takes the next code of its class and copies the first sibling's binding. Price is the full recipe. New `OnAdded` event and credit cause `Add`. The Ship side panel now scrolls, which also fixes the rebuild and swap clip. Proof: `examples/systems/system_station_add.rs` (refusal at a taken socket, an add on a base hull and on a generated hull, each surviving freeze and thaw). The orchestrator reviewed the Add list frame (point buttons, nine parts with prices) and re-ran `content lint`: 0 errors. Gaps: the Add list on a hull is long; no roll or socket cycling; `snap.rs` keeps its own overlap check; the Map panel still clips.

## Owner feedback (2026-10-14, reopens the task)
- Ships for sale, Escorts for hire and Buy station do not belong in the Ship tab. Move them to a new tab. Only refit (swap, add, rebuild) stays in Ship.
- Refit picks rows named by socket code (`positive_x_a`), which means nothing to a player. Make it work like the editor: a section catalogue, limited to what the station sells, with the selected section as a ghost on the cursor; the player places it. Keep the current Ship view look, which the owner likes.
- Bugs that must be reproduced, then fixed:
  1. after placing a section, the other tabs do not open until the HUD is closed and opened again;
  2. the HUD element for the new section does not update;
  3. plating and greebles do not rebuild locally around the new section.
- The owner rates these Ship HUD problems as the worst.

- Owner feedback 2026-10-14, second round: the add ghost can select a section inside the ship, which has no reachable face, so nothing can attach there. Only sections with a free, unobstructed link point on the outer surface may be targeted.

- Owner feedback 2026-10-14, third round (the owner says it already works much better):
  - rebind sections in the Ship view (weapon and tool bindings);
  - while placing, roll the part and cycle its attach link point with R and F, as in the editor (`crates/nova_editor/src/placement.rs:44-46`). R and F tilt the Ship view camera today. Orchestrator default: while a part is held, R and F act as in the editor; with no part held, they tilt;
  - quick buy: when a part cannot be placed because recipe parts are missing, offer one action that buys the missing parts from the station and places it;
  - while docked, the Ship view shows the docked station around the ship, not editable, and refuses a placement that would collide with it.

## Delivery gate
- Propose explicit attachment points on a hull design (content), lint for overlap, and the bind rule for a new section. Stop for the owner on whether base hulls get authored points or only generated ones.

## Verification
- Asserted example: add at a free point, post-build checks pass, binding exists; add at an occupied point refuses with nothing changed.

## Done when
- Add works in a live docked run on one base hull and one generated hull.

## Owner feedback (2026-10-15, fourth round)
- Rebind mode must swallow key presses: rebinding `R` must not tilt the camera. Show that rebind mode is active (the button lit, a prompt on screen).
- The refit catalogue must look and work like the editor's section catalogue. A button switches the Ship view into an editor-style catalogue mode (sections list, editor layout). The Ship view's right panel stays clean and minimal, with a Delete button for the selected section.
- Delete a section: select one of the player's sections and recycle it (parts back by the recipe rules).
- `Q` picks the hovered section like the editor pipette and holds it for placing. Q/E no longer turn the camera; use the editor's camera controls.
- While a part is held, the hovered face of the ship is drawn opaque (the real surface), not the transparent green tint, so the exact placement face is visible.
- Bug: the Map tab's zoom leaks into the Ship view; zooming the map zooms the Ship view too.

## Fourth-round choices (2026-10-15)
- Map zoom leak. Cause: the interface pauses `Time<Virtual>`, so the fixed
  step that ages messages stops and an inactive pane's wheel reader keeps
  the travel. Options: (a) a zoom resource per pane; (b) drain the pointer
  readers every frame at the input owner. Chosen (b), `active_pointer`:
  the orbits were already per pane, only the reader leaked.
- Rebind swallow. Cause: the capture closed on the press frame and the view
  read the still-held key the frames after. Options: (a) consume the key in
  `ButtonInput`; (b) a `rebind_swallow` flag that keeps `rebind_armed()` true
  until every button is up. Chosen (b): it also covers Esc, and other
  readers keep a true input state. The prompt is a banner over the view
  (the panel note line is empty while armed); Rebind wears `Selected`.
