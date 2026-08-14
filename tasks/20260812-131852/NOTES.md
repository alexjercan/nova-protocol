# Notes - editor parts gallery (first draft)

## Shape delivered

A full-screen gallery MODE, not a panel in the drawer. The owner's open
question ("separate tab for the parts, or keep them in the side menu list")
is answered as BOTH, so the two can be judged side by side:

- the rail keeps `Components`, which toggles the existing card drawer;
- the rail gains `Parts Gallery`, which opens the overlay.

Why a mode: a tile needs a real 3D preview, and a 280px drawer cannot hold one.
The overlay hides the editor's own rail + drawer while it is up
(`EditorChrome`), because the tile grid has to stay TRANSPARENT for the
previews behind it to show - UI always draws over 3D, so chrome behind the
gallery cannot be covered, only hidden.

## How the 3D works

- No second camera and no render layers. The gallery parks the editor's own
  free-fly camera (`ParkedPose` + removing `WASDCameraController`, the same
  move the scenario's cinematic camera makes) on a stage 2000 units above the
  build area, and restores it on close. The skybox is the backdrop.
- Layout owns the grid: each tile's preview is placed by unprojecting its UI
  cell's centre (`Camera::viewport_to_world`) at a fixed ray distance, so all
  tiles are the same apparent size and the grid stays responsive. Previews
  spawn hidden and appear once their cell has been measured.
- Previews are real preview sections through the shared
  `preview::insert_preview_section` (extracted from `placement.rs`), so a tile
  and a placed section can never render differently.

## Decisions to review

1. Two pickers (gallery + drawer) or one. Cheap to delete either.
2. Category ROW, not a dropdown. Six kinds fit across the header, and
   `nova_ui` has no dropdown widget; a row costs no new widget family. The
   ship family (racer / cargoa / cargob) is reachable through the text filter
   (it matches the catalog id) rather than as a second dropdown.
3. Click a tile = FOCUS it, not place it. The flow is browse -> filter ->
   focus -> place, which is the DoD's flow, but it costs mouse users one extra
   click per part. The drawer list is the one-click path.
4. The filter field is not `bevy_ui_widgets`' `EditableText`: the gallery owns
   the keyboard while it is up, so the box only has to SHOW what was typed.
   Adopting the real widget means pulling in `InputFocus` + IME plumbing.
5. Tiles fit to the part's authored COLLIDER extent, not its mesh bbox. Right
   for the semantic parts (tight primitive colliders); approximate for the
   standard sections, whose Kenney meshes are smaller than their unit-cube
   colliders.

## Coverage

- `nova_editor` unit tests: filtering (hidden prototypes, category, filter
  case/ID), selection clamping and resolution through the filter, and the
  filter field's typing rules.
- `examples/ui/editor.rs` walks the gallery with real gestures: open, browse
  (tiles are up), filter (typing narrows the grid), focus (the card names the
  part and reads its stats), place (arms the tool and closes), then a click on
  the ship builds that part. Run: `NOVA_AUTOPILOT=1 cargo run --example editor
  --features debug` (needs a display).
- New harness gesture `nova_autopilot::input::type_text`: `press_key` writes
  only the held-key state, so it cannot fill a text field.
- Figures: the walk shoots `editor-gallery.png` and
  `editor-gallery-focus.png` under `NOVA_CAPTURE`.

## Left for the land, not done here

- `feature-editor.png` (site index) now misses the new rail row. Refreshing
  the shipped web assets waits on the owner's verdict on the gallery's shape.
- Keybind wiki page: the gallery's keys (arrows / PgUp / PgDn / Enter / Esc /
  type) are on-screen in the hint line but not yet in `web/src/wiki/keybinds.md`,
  for the same reason.
