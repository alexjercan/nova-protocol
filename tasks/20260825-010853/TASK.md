# Editor UX: hierarchy tree, context actions, focused editing

- STATUS: CLOSED
- PRIORITY: 82
- TAGS: v0.12.0,editor,ui

Child of the v0.12.0 editor epic (`20260812-131912`). The editor UI panels
the foundations task (`20260824-120520`) noted were owned by no task, plus
the interaction polish the owner asked for on 2026-08-25. First half of the
editor UX pass; the engineer readout (`20260824-120535`) stays its own task
and lands inside the layout this one builds.

## Goal

The WIP furniture becomes an editor that reads like one: a real hierarchy
tree, actions split per context, and an edit context you can see.

## Decisions (owner, 2026-08-25)

- Single click in the hierarchy is the gesture: a ship row enters, the
  scenario row leaves, a section row selects. Double-click dies.
- The hierarchy is a TREE: scenario root, ships nested, the entered ship's
  sections nested under it, ASCII connectors and per-kind glyphs.
- Entering a node FOCUSES it: the viewport shows only the entered ship.
  The scenario context shows everything.
- Actions split per context in a top bar: scenario = Add Ship + Play,
  ship = Parts + Delete + Rebind (acts on the selected section). The rail
  keeps the tree; ship settings (skin, look, attitude) show only inside a
  ship.
- Add Ship creates a BLANK ship; the first armed part founds it at the
  ship origin. The seeded V1/V2 buttons die.
- With no part armed, a world click SELECTS: a section inside a ship, a
  ship at the scenario node. Entering by world click dies; the hierarchy
  is the door. Rebind moves from click-on-section to the top-bar action.
- One transform gesture now: dragging a ship at the scenario node moves
  it on the ground plane, to find how the rest will look.

## Out of scope (second half or later)

- Node settings and the section inspector; enter at section level.
- Non-ship object kinds and the add-object menu beyond the one Ship entry.
- Rotation and scale gizmos; only the plane drag lands now.
- Converting the sandbox scene into the editor.

## Landing order

1. The hierarchy tree, single-click enter and select.
2. Top bar, per-context panels, blank Add Ship, world select, Rebind.
3. Focus isolation, ship dragging, live-range coverage.

## Done when

- The tree shows scenario, ships and sections with the entered branch
  open, and one click enters, leaves or selects.
- Inside a ship only that ship is visible; at the scenario node all are.
- Add Ship starts blank and the first part founds it at the origin.
- The driven editor range covers: found a blank ship, select in the world,
  enter via the tree, isolation, and a plane drag. It stays green.

## Proof (first half, landed 2026-08-25)

Landed as `5198d3de` (tree + top bar + blank Add Ship + world select) and the
focus + drag commit that follows it. All three driven ranges green under
Xvfb, EXIT=0:

- system_ship_editor: `founded the ship with basic_controller_section_1` at
  the ship origin; `select mode marked 'reinforced_hull_section_3' and placed
  nothing`; inside ship_2 the probe lists `visible_ships == ["ship_2"]` and
  both ships back at the scenario node; `clicked ship_1 in the world -
  selected, not entered`; re-entry through `Scene Row ship_1` finds the same
  8 ids; `dragged ship_1 from (0,0,0) to (1.085,0,0)` with altitude
  unchanged; the flown ship re-derives the same 7 mates in 26 plates.
- screenshot_editor and bug_sandbox_soak reworked for the new doors (both
  were latently red on master: they still pressed Play from inside a ship).
- 103 unit tests in nova_editor; `cargo check --workspace --all-targets`
  clean. The captured `feature-editor.png` shows the bar, the tree and the
  clipped one-line rows.

Decisions taken in flight:

- Leaving a ship PUTS THE TOOL DOWN (`disarm_outside_ship`): placement and
  delete are ship-context verbs, and a part silently in hand at the scenario
  node blocked both select and drag while showing no tool anywhere.
- "Empty space" for the founding click means the nearest picking hit is the
  WINDOW: bevy_picking targets the window when nothing is under the pointer,
  so any other hit is a click on something.
- Focus isolation writes two facts: `Visibility` on the ship node, and
  `Pickable::IGNORE` on the hidden views - the picking ray does not care
  what renders, and an invisible collider still eats clicks.

## Owner feedback round (2026-08-25, after the first half)

The first half landed clunky. Fixed in one revision on master (the commit
after `c462de5e`); proof: all three driven ranges green under Xvfb EXIT=0
(system_ship_editor now founds ship_2 at x=24 and roofs a hull onto its top
socket at ship-local (0,1,0), and finds "Sandbox Ship ship_2" on the range
after Play), 109 unit tests. The defects, as found:

- Focus isolation DID NOT WORK live: section nodes spawned with
  `Visibility::Visible`, which overrides a hidden ancestor, so hiding the
  ship node hid nothing. The unit tests and the probe read the ship's own
  component and stayed green. Fixed: sections are `Inherited`; regression
  test pins the component.
- The second ship could only mate one link point: the solver got the picking
  hit in WORLD space against ship-LOCAL sockets. Ship_1 at the origin masked
  it. Fixed: `ship_local_hit` maps the hit through the inverse ship pose.
- The second ship never spawned on Play: only the Player-driver ship was
  lowered. Fixed: `lower_fleet` lowers every non-empty design; AI ships spawn
  neutral under AI at their stage offset from the player.
- The Scene tree came back EMPTY after Play (the quick-review critical):
  `Local<ShownScene>` outlived the DespawnOnExit list. Fixed with the
  `Added<SceneList>` override, live-tree test first.
- The footer legend was wrong per level (told the scenario node about the
  pipette, told the ship that Esc pauses). Now keyed on context + tool.
- Zero context feedback. The breadcrumb now reads `[SHIP] scenario / ship_1
  selected hull_3` in phosphor.
- The camera never moved: entering ship_2 at x=24 showed empty space. Now
  the stage camera snaps on context change (enter/exit/re-entry), via the
  gallery's remove/re-insert `WASDCameraController` move. NOT on selection:
  a selection snap breaks grab-after-select (the drag walk holds the pointer
  over the ship it just selected).
- The founding pose was invisible. An empty ship with a part armed now draws
  a pad at the origin: socket ring + the armed part's bounds.
- Back to Main Menu now DELETES the document (owner decision) - a session
  ends at the menu; Play round-trips still keep it.
- RMB release no longer drops an LMB ship drag (DragEnd filters by button).
- Tab only opens the gallery inside a ship (still closes anywhere): parts
  are ship verbs, and a part armed at the scenario node disarmed instantly.
- File / Edit / View placeholder menu bar on the top bar, greyed.

## Second half (not started)

- Node settings and the section inspector; enter at section level.
- The add-object menu beyond the one Ship entry, and non-ship kinds.
- Rotation and XYZ gizmos with real handles; camera snap on SELECTION once
  it cannot fight the drag gesture.
- The owner wants the Blender/Godot/Unity look and feel in NovaOS theme;
  the bevy `jackdaw` editor is the inspiration to study.
- Long ids clip at the rail edge; the rail's width wants a look when the
  inspector lands.
