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
- Editor controls in the Ship view. The editor binds RMB-hold look, WASD
  fly, Space/Shift up/down, F frame, Q pipette, R roll and F socket while
  armed. Options: (a) port the free-fly rig (loses the orbit around the
  selection, which the read-only Info view reuses); (b) keep the orbit, turn
  it by RMB drag as the editor looks, drop desk Q/E/R/F turning and keep the
  pad's D-pad, which has no pointer; (c) move turning to other keys (no
  editor precedent). Chosen (b). The Map keeps Q/E/R/F.
- Pipette. `ship_pick` (Q) follows `viewer_orbit_left`, as `ship_part_roll`
  follows tilt, so one rebind moves both and the table has no conflict; it
  is read off the desk only. It takes the hovered section's catalogue part
  (the editor's pipette takes the prototype, not the pose) at the default
  mount, only at a Refit station; elsewhere the note line says why. A part
  hidden from the catalogue is refused on the note line.
- Diagnostics: the sandbox adopts the short layer (30421fa6e): an intro
  sentence, 1-3 panel lines, the rest as Shift+F3 detail, hints only while
  the key applies, and free sockets only within 1 cell of the target socket.
- Rebind proof key. R no longer tilts, so the proof binds `T` (reset view),
  which is free of the flight table. `]` is the flight lock-next key, and
  the panel refuses it.
- Opaque target face. Options: (a) draw the target section's real model in
  the schematic (a second scene per section on the Ship layer); (b) an
  opaque fill on the target block plus an opaque accent plate on the face
  whose axis is nearest the target socket's normal. Chosen (b): the
  schematic stays one material family, and the plate marks the face where a
  flat opaque cube cannot. The target section's badge hides while aimed, as
  it sat on that face. Other blocks keep the translucent tint.
- Delete. Gate: options (a) anywhere; (b) docked at any station; (c) docked
  at a living station that offers Refit, as Swap and Add. Chosen (c). Rule:
  the refit build checks the add and the swap run (a removal can only fail
  the graph, the sockets, the thrust floor and the computer count), plus
  the last computer refused even on a ship already short, plus the port the
  dock holds by. Yield: options (a) the full recipe back; (b) the strip
  rule at the section's health share; chosen (b), so a delete never pays
  more than a wreck strip. A hold too full refuses (a strip pauses; a
  delete has no job to pause). Event: the existing `OnStripped`, `hull` the
  ship's own id; no new event. The design turns inline without the section
  and its binding leaves the table, so a rebuild does not list it and a
  save keeps it gone. The skin re-clads only the cells round the hole. The
  Delete button stays pressable when refused so the reason reaches the
  note line; the panel shows it only while docked at a Refit station.
- The note line moved from the panel's end (after the catalogue, off
  screen once it scrolls) to under the action buttons, so a refusal shows
  where the press was. The footer names the key `Remove`, as its keycap
  already reads `Delete`.
- Catalogue mode (item 5). Home: options (a) a new crate; (b) `nova_hud`
  (the editor does not depend on it); (c) `nova_ship`, the lowest crate both
  `nova_editor` and `nova_interface` depend on that holds `GameSections`.
  Chosen (c): `nova_ship::gallery`, plus a direct `nova_ui` dependency that
  was already in its graph through `nova_gameplay`. The editor's
  `gallery/` moves there (catalog, state, layout, overlay, tiles, focus
  view, keyboard); `insert_preview_section`, `PreviewRole` and
  `draw_socket` move with it. The editor keeps its host parts: the Tab
  toggle, the camera park, the stage clearance and the chrome hide.
- Seams. A `GalleryHost` component on a UI node: the camera that draws the
  tiles, the stage they stand on, their render layers and the close hint.
  The overlay lays itself out in that node, and a cell maps to the camera
  viewport by its place in the node, so a window host and an image host
  use one path. An optional `GalleryListing` on the host limits the parts
  and gives each a note line and an optional second verb
  (`GalleryAlternate`). The part in hand is one resource, `HeldPart`:
  `SectionChoice` and `ShipRuntime::held` both go. The gallery claims
  `InputMode::Browse` while open, so a host's Normal-only keys (the
  interface close, Tab, the Ship keys) stay quiet. The turntable counts real
  seconds, as the interface pauses virtual time.
- Ship pane. A Catalogue button (refit dock only) opens the gallery over the
  view and the panel, drawn by its own camera on render layer 24 into an
  image the host node shows. The listing is the old catalogue rule; the note
  is the price or the refusal; Swap is the second verb when the part fits
  the selection. Place puts the part in hand and closes. The panel keeps
  the name, the condition bar, the binding with Rebind, Delete and the note
  line; Repair and Rebuild show only when they apply. Prev, Next, the
  description, the integrity and ammunition rows and the list go.
- Footer. One line: each key shows only while it applies (Pick on a hovered
  section, Roll and Socket while holding, Repair, Rebind and Remove when
  the panel would allow them), and the click verb reads place or select.
  Mates and Reset leave the line (Reset has its button). The sandbox stops
  writing its own key hints, which repeated the footer.
- Shared gallery state. `GalleryState` and `HeldPart` are one resource for
  every host, so the Ship pane closes only a gallery it hosts. It first
  closed any open gallery each frame it was not docked, which shut the
  editor's browse.
- Delete state (item 7). Options: (a) keep the press and put the refusal on
  the note; (b) one rule, `plan_ship_delete`, read each frame for the button
  and again at the act. Chosen (b). The verdict no longer waits for a Refit
  dock, so a dock with no refit also shows its reason. While it refuses, the
  button carries `InteractionDisabled` (grey, no hover, no click), the
  reason sits under it in Danger, and the Delete key writes nothing. A
  refused command still re-checks and changes nothing, with no note. The
  reason keeps its `delete:` prefix, as the swap and repair lines do.
- Delete yield (item 8). The label is `Delete: +1 Frame spar, +3 Section
  panel`, or `Delete: nothing to salvage`; items, not credits, from
  `delete_yield_text`. The note after the act is `deleted PDC-1: ... to the
  hold`, without "to the hold" when nothing was salvaged.
- Q put-away (item 9). Q with no section under the pointer empties the hand
  in the Ship pane (`pick_ship_part`) and in the editor
  (`pick_section_under_pointer`); the ghost goes with it. Q over a section
  still picks. In the editor a hit on something that is not a section
  counts as empty space. Options were Q only, or a second key such as Esc
  (the gallery and the rebind own Esc); Q only, as asked.
- Place ease (item 10). The ease already existed: `ship_input` eases the
  orbit pivot onto every new selection at the same radius, and an add selects
  the new section. It did not move after a place because `ship_input`
  consumed the selection while `ShipSections` did not list the new section
  yet. Now it consumes a selection only when the section is listed. Options
  were that fix, or a separate place-only ease target set by the add. The fix
  is one rule for all selections. Open: the request says only a place eases,
  never a pick or a select. A pick never eases (it does not select). A select
  still eases, as it did before this task; removing that is the owner's call.
