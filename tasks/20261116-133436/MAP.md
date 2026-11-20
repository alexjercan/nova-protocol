# Catalogue UX: code map and live walk (Delivery step 1)

Mapped 2026-11-20 at `752e4d52c`. Mapping only: no game code changed.
Frames: `before/flow-sheet-1.png` (steps 00-12), `before/flow-sheet-2.png`
(13-25), `before/flow-sheet-3.png` (open catalogue at 1024x768). Full-size
frames: `~/Pictures/nova-catalogue-map/NN-<step>-<WxH>.png`.

Walk setup: a throwaway copy of `system_station_add` (waystation + mining
tug, 20000 cr, PDC parts in the hold), Xvfb + lavapipe, sandboxed XDG
dirs, autopilot harness keys and clicks. Each step logged the gallery,
input mode, pause state and held part. Both resolutions gave the same
state path.

## Corrections to TASK.md agent findings

- Line refs `catalogue.rs:300-346,424` are stale; see the map below.
- "TAB inside the catalogue closes the catalogue and the interface
  together" is FALSE. While the catalogue is open TAB and M do nothing
  (frames 15, 16). TAB closes the interface when the catalogue is NOT
  open, and that drops a part in hand (frame 22). This is the most likely
  source of "pressing tab takes you out of the UI".

## Code map

### Owner and entry

- Pane host: `crates/nova_interface/src/ship/catalogue.rs`. Shared widget:
  `crates/nova_ship/src/gallery/` (`mod.rs`, `input.rs`, `ui.rs`,
  `scene.rs`), also hosted by the editor.
- Only entry: the Ship pane footer `Catalogue` button
  (`ShipCatalogueButton`, `GalleryAction::Open`, `pane.rs:1844-1845`).
  No key opens it.
- The button is HIDDEN (`Display::None`), not disabled, unless refits are
  active, no subject is open and the site passes the Refit gate
  (`catalogue.rs:258-267`). Undocked or at a non-refit station there is no
  Catalogue at all.
- `GalleryAction::Open` resets filter, filter focus, available-only,
  selection and focus; it keeps the category (`gallery/ui.rs:1042`,
  `GalleryState::opened` `gallery/mod.rs:363-369`).

### What it covers

- `manage_ship_gallery` spawns `ShipGalleryHost` as an absolute full-size
  node with `ZIndex(10)` under the pane body (`catalogue.rs:398-424`). It
  covers the ship view, the side panel and the footer. The card head (pane
  tabs, station tags, credits, `M Next`, `Close Tab`) stays visible.
- Host config: `close_keys: "Esc"`, `close_verb: false`
  (`catalogue.rs:407-409`). So the gallery draws no Close button; the only
  visible close verb is the top bar `Close Tab`, which closes the whole
  interface (`pane.rs:528`).
- The 3D tiles render through an RTT camera (`ShipCatalogueCameraMarker`,
  `catalogue.rs:73-77`).

### How it closes

- Gallery Esc ladder (`gallery/input.rs:58-67`): filter caret, then focus
  card, then close.
- Place: sets `HeldPart`, closes the gallery (`gallery/ui.rs:1069`).
- Swap (Alternate): writes `GalleryAlternate`, closes the gallery
  (`gallery/ui.rs:1078`); `apply_ship_gallery_alternates` turns it into
  `SectionSwapCommand` for the selected section (`catalogue.rs:429-447`).
- Teardown (`catalogue.rs:316`): camera exists and (host gone, or state
  closed, or no refits) -> `open = false`, despawn camera and host. A pane
  tab switch rebuilds the pane body, the host goes, the gallery closes.

### State and ordering

- `GalleryState` (open, category, filter, filter_focused, available_only,
  selected, focused) `gallery/mod.rs:341-358`; `HeldPart`
  `gallery/mod.rs:84-93`; `GalleryLayout` `gallery/mod.rs:266-303`.
- `claim_gallery_keyboard` claims `InputMode::Browse` while open with a
  host, in PreUpdate before `InputModeSystems` (`gallery/mod.rs:424-432,480`).
- Gallery chain in `PartsGallerySystems`, run if any `GalleryHost`;
  `gallery_keyboard` runs if `owns_or_enters(Browse)` (`gallery/mod.rs:466-533`).
- `(manage_ship_gallery, update_ship_gallery_listing)` after
  `update_ship_panel`, in `ShipPaneSystems`, before `PartsGallerySystems`
  (`ship/mod.rs:230-237`).
- `manage_ship_scene` clears `HeldPart` and the selection whenever the
  Ship pane goes inactive or active (`ship/scene.rs:381-385`).

### Input routing while open (InputMode::Browse)

| Input | Owner | Effect while catalogue open | Evidence |
|---|---|---|---|
| TAB | `toggle_interface`, Normal only | nothing | `lib.rs:185-187`, frame 15 |
| M | `next_interface_pane`, Normal only | nothing | `lib.rs:188`, frame 16 |
| Esc | gallery ladder; `close_surface_from_menu_keys` returns in Browse | filter -> card -> close | `terminal/input.rs:118-149`, frames 06, 09, 17 |
| Enter | gallery | filter: top hit; grid: focus; card: Place | `gallery/input.rs:68-86` |
| Q | gallery pipette over hovered tile | take part, close | `gallery/input.rs:91-104` |
| / | gallery | focus filter | `gallery/input.rs:26` |
| Arrows, PgUp/PgDn | gallery | step grid / page | `gallery/input.rs:112-132` |
| Wheel | `drive_focus_view` | card: zoom; grid: drained, no paging | `gallery/scene.rs:299-345`, frame 07 |
| Left drag | `drive_focus_view` | card: turn model | `gallery/scene.rs:299-345` |
| `[` `]`, B, Delete, R, F | `ship_input`, Normal only | dead | `ship/mod.rs:200,206` |
| Pane tab click | `on_interface_tab_click` | switches pane, catalogue closes | `pane.rs:1167`, `catalogue.rs:316` |
| Top bar Close | `on_interface_close_click` | closes whole interface | `pane.rs:528`, frame 24 |

With the catalogue closed (Normal): TAB closes the interface, and the pane
inactive edge drops `HeldPart` (frame 22). Esc in the Ship pane: a held
part first, then the hull-info subject, then the interface
(`terminal/input.rs:62-83`, frames 13, 25).

### Gallery UI

- Header (`gallery/ui.rs:180-298`): `PARTS`, category chips (All,
  Structure, Propulsion, Control, Weapons, Ordnance, Docking, plus mods),
  filter box with `x`, `Available` chip, count `N of 26 parts`, page,
  `<` `>`. Close button only if `close_verb` (`ui.rs:290`).
- Grid: 5x2 at 1280x720 (page 1/3 for 26 parts), 4x2 at 1024x768 (1/4).
  Placeable tiles show the price; muted tiles have grey titles; hover
  shows the reason (`NOT BUILT`, frame 03 at 1024).
- Hint row (`ui.rs:322-379`): grid `Q pick, Arrows select, Enter focus,
  / filter, Esc close`; card `Drag turn, Wheel zoom, Arrows next, Enter
  place, Esc back`; filter `Enter top hit, Esc done`.
- Focus card (`ui.rs:746-920`): turntable, 320 px card with recipe rows,
  description, stats, price, verbs `Back` / `Swap <code>, <price>` /
  `Place Enter` (`ui.rs:881-904`). Swap shows only with a selected section
  the station can swap (`catalogue.rs:89-148`, `:136`).
- The Ship pane footer hints hide while the gallery is open
  (`pane.rs:1714-1724`).

### HeldPart, ghost and the return trip

- `on_ship_viewport_click` -> `commit_placement` writes `SectionAddCommand`
  or denies (`ship/ghost.rs:147-201`). The part STAYS in hand after an add
  (frame 12, `held=Some`, sections 28 -> 29, 20000 -> 19900 cr).
- Placing panel (`ship/held.rs`): price, hold, bought, socket, roll. No
  put-away button; Q over empty space or Esc puts it away.
- A second, different part: Esc (or Q) to put away, then click `Catalogue`
  again, which reopens on the same category with the filter cleared
  (frame 14). Picking a new part with a part in hand needs the same
  footer button (frames 10 and 12 show Catalogue visible while holding).
- Swap: select the section in the ship view first (`[` `]` or click),
  then open the Catalogue and focus a compatible tile. Selection keys are
  dead while it is open. After Swap the gallery closes (frame 20).

### Undocked or no refit station

- Catalogue button hidden (`catalogue.rs:258-267`); if refits drop while
  open, teardown closes it (`catalogue.rs:316`). Not walked live.

### Other consumers

- Editor: `crates/nova_editor/src/gallery.rs` hosts the same gallery
  with `close_keys: "Esc/Tab"`, `close_verb: true` (`:114-121`);
  `toggle_gallery` makes TAB open AND close it (`:193-207`). The editor
  reads `GalleryState` / `HeldPart` in `probe.rs`, `placement.rs`,
  `ui/shell.rs:512`. A gallery change lands in both hosts.
- Crafting UI (task 20261116-132140, CLOSED): `craft.rs` `CraftViewHost`
  (`:174`) covers the Inventory body the same way but does NOT reuse
  gallery code and claims no input mode. Esc closes it through
  `InterfaceLevels` (`craft.close()`, `terminal/input.rs:85-86`); TAB in
  the craft view closes the whole interface. A fix that lives in the
  gallery host does not reach it; a fix in the interface Esc/TAB levels
  does.

### Keybinds (`crates/nova_interface/src/bindings.rs`)

- `interface_toggle` TAB, context Always (`:50-53`); `interface_next_tab`
  M (`:54-57`); viewer Q/E/R/F/W/S/A/D, T, `[` `]` (`:60-103`);
  `ship_mates` G, `ship_repair` P, `ship_rebind` B, `ship_delete` Delete
  (`:122-134`); `ship_part_roll` R, `ship_part_socket` F (`:139-148`);
  `ship_pick` Q (`:152-156`).
- Gallery keys are hard-coded, not bindings: Q, /, Esc, Enter, arrows,
  PgUp/PgDn (`gallery/input.rs:22,26`).
- Ship hints: `SHIP_KEY_HINTS` (`pane.rs:1409-1442`).

## Step table (frames at 1280x720; `*` also at 1024x768)

| # | Input | Screen | State change | Frame |
|---|---|---|---|---|
| 00 | - | flight view, DOCKED WAYSTATION | Unpaused | 00-docked-flight |
| 01 | TAB | interface opens on Map | Interface, pane Map | 01-interface-map |
| 02 | click Ship tab | ship view, panel, footer with Catalogue/Fit/Reset | pane Ship, CGO-1 selected | 02-ship-pane |
| 03* | click Catalogue | gallery covers body; card head stays | Browse, open, All | 03-catalogue-open |
| 04* | click Weapons | 6 of 26 parts | category Weapons | 04-filter-weapons |
| 05* | `/`, type `kinetic` | 2 parts, caret, filter hints | filter set, focused | 05-filter-typed |
| 06 | Esc | caret gone, text stays | filter_focused false | 06-esc-leaves-filter |
| 07 | wheel -3 over grid | no change | none | 07-wheel-over-grid |
| 08* | click PDC tile | focus card, Back / Place | focused | 08-focus-card |
| 09 | Esc | grid, orange focus ring on tile | focused false | 09-esc-on-card |
| 10* | tile, Place | gallery gone, PLACING panel | Normal, held PDC | 10-place-in-hand |
| 11 | aim pointer | ghost + `100 cr` tag | none | 11-ghost-valid |
| 12 | click | PDC-1 added, still PLACING | sections 29, 19900 cr, held kept | 12-placed |
| 13 | Esc | PDC-1 info panel | held None | 13-esc-puts-part-away |
| 14* | click Catalogue | Weapons kept, filter empty | Browse | 14-catalogue-reopened |
| 15 | TAB | no change | none | 15-tab-while-open |
| 16 | M | no change | none | 16-m-while-open |
| 17 | Esc | ship view | gallery closed, Normal | 17-esc-closes-catalogue |
| 18 | (PDC-1 already selected) | PDC-1 info | none | 18-pdc-selected |
| 19* | Catalogue, Twin PDC tile | card with `Swap PDC-1, 100 cr` | focused | 19-swap-card |
| 20 | click Swap | PDC-1 now Twin PDC | 19800 cr, gallery closed | 20-swapped |
| 21 | Catalogue, PDC, Place | PLACING again | held PDC | 21-second-part-in-hand |
| 22 | TAB | flight view | Unpaused, held None | 22-tab-with-part-in-hand |
| 23 | TAB | Ship pane, CGO-1, view reframed | Interface, select reset | 23-tab-again |
| 24 | Catalogue, top bar Close | flight view | Unpaused | 24-close-button-while-open |
| 25 | Catalogue, Esc, Esc | flight view | Unpaused | 25-esc-twice |

## Awkward points (worst first)

1. TAB with a part in hand (Normal mode) closes the interface and throws
   the part away with no cue (frame 22; `lib.rs:185-187`,
   `ship/scene.rs:381-385`). On return the selection resets and the view
   reframes (frame 23).
2. The only visible close verb over the catalogue is the top bar
   `Close Tab`; it closes the whole interface, not the catalogue (frame
   24; `catalogue.rs:409`, `pane.rs:528`). The catalogue Esc is only in
   the small hint row.
3. TAB means different things: editor = parts gallery toggle
   (`nova_editor/src/gallery.rs:193-207`); game = close interface. The
   top bar advertises `Close Tab` while the catalogue is open, but TAB
   does nothing there (frame 15). The label is wrong in that state.
4. Every Place or Swap closes the catalogue (`ui.rs:1069,1078`). Each new
   part costs: Esc to put away (or keep), footer Catalogue click,
   re-filter (filter reset, frame 14), find tile, Place. No key reopens
   it.
5. Swap needs the section selected BEFORE opening the catalogue; inside,
   `[` `]` and click-select are dead and the ship is hidden. Nothing in
   the grid says which section a Swap targets until a card is focused
   (frame 19).
6. The catalogue hides the whole ship. The player cannot see where the
   part will go or what is selected while browsing.
7. Esc order differs by place: gallery Esc closes the gallery, but a
   second Esc in the Ship pane (no part in hand) closes the interface
   (frame 25). Two quick Esc presses leave the UI.
8. A part stays in hand after a click-add (frame 12): good for repeats,
   but the PLACING panel has no put-away button; Q/Esc are only in the
   hints.
9. The wheel over the grid does nothing (frame 07; `scene.rs:299-345`);
   paging is `<` `>`, PgUp/PgDn or arrows only.
10. With no refit the Catalogue button disappears instead of showing why
    (`catalogue.rs:258-267`). Not walked live.

## Tests and examples that pin current behaviour

- `crates/nova_interface/src/pane/tests.rs:473`
  `the_catalogue_button_opens_the_gallery_and_clicks_once`
- `pane/tests.rs:523` `the_craft_button_opens_the_craft_view_and_a_tab_switch_closes_it`
- `pane/tests.rs:732` `tab_typed_into_a_focused_field_does_not_close_the_interface`
- `pane/tests.rs:955` `the_next_tab_key_steps_through_jobs_always_and_station_only_while_docked`
- `crates/nova_interface/src/terminal/tests/toggle.rs:7` `tab_opens_and_closes_the_interface`
- `toggle.rs:307` `escape_puts_a_held_part_away_before_it_closes_the_interface`
- `toggle.rs:389` craft view Esc
- `crates/nova_interface/src/ship/tests.rs:1188` `a_held_part_draws_its_target_block_and_face_opaque`
- `crates/nova_interface/src/ship/delete.rs:674` `a_part_in_hand_takes_delete_away`
- `crates/nova_interface/src/ship/catalogue.rs:518-648` (listing: muted,
  shortfall, payment split, credit short)
- `crates/nova_ship/src/gallery/mod.rs:634` `an_open_gallery_with_a_host_claims_browse`,
  `:708` `clearing_the_filter_keeps_the_category`, `:739`
  `browsing_a_kind_opens_the_gallery_on_that_kind`, plus stepping, filter
  resolution and grid layout tests
- `crates/nova_ship/src/gallery/input.rs` tests: filter typing, arrows
  (`:322`), Q pick, `place_enter_and_pick_leave_the_hand_empty_for_a_muted_or_short_entry` (`:432`)
- No test pins TAB, M or Esc while the catalogue is open inside the
  interface, and none pins TAB dropping a held part.
- Examples: `examples/systems/system_station_add.rs` (catalogue ->
  Weapons -> PDC -> Place -> click), `system_station_swap.rs` (claim
  "a tab switch closes the catalogue and its camera" `:21,:1008-1030`,
  Swap via `Gallery Alternate Button` `:1083`),
  `system_station_swap_intake.rs`, `examples/screenshots/news_0160_refit_catalogue.rs`,
  `examples/playable/station_docked_sandbox.rs` (Esc close `:3248-3262`;
  closest playable, README row `examples/playable/README.md:50`),
  `examples/playable/shipyard_sandbox.rs`.
- Docs: `docs/sections.md:196-215` (Swapping a section).

## Design questions (no decisions)

1. Should TAB be inert, close the catalogue, or close the interface while
   a part is in hand? Should a held part survive an interface close?
2. Should the catalogue sit beside the ship view (side drawer) so the
   player keeps the ship, the selection and the ghost in sight?
3. Should Place keep the catalogue open (or reopen it after the add) for
   a multi-part session, and should the filter persist between opens?
4. Should the catalogue get a key (for example the editor's TAB, or a
   ship-pane binding) and a visible close verb of its own, instead of the
   top bar Close?
5. Should Swap target follow the live selection inside the catalogue, so
   `[` `]` work while browsing?
6. Should the top bar Close hint reflect the active layer (catalogue
   open: `Esc` closes catalogue; else `Tab` closes interface)?
7. Does the craft view take the same rules? It has its own host and no
   input mode today, so a shared fix belongs in the interface Esc/TAB
   levels, not only in the gallery.
8. With no refit, hide the Catalogue button or show it disabled with the
   reason (owner rule: UI implies state, no prose)?

## Not verified

- Undocked and non-refit behaviour (code only).
- Wheel over a multi-page grid (walk scrolled a 1-page filtered grid;
  code drains the wheel in the grid).
- Pane tab click while the catalogue is open (code and
  `system_station_swap` claim 0 only).
- Frames 24 and 25 do not show the catalogue open before the close; the
  Catalogue click step passed, and the same click opened it in steps 03,
  14 and 21.
- Mouse drag on the focus card (not driven).
