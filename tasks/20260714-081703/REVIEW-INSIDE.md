# Editor review, from inside the code

Read for this pass: the whole of `crates/nova_editor/src` - `lib.rs`, `node.rs`,
`placement.rs`, `inspect.rs`, `config.rs`, `frame.rs`, `gizmo.rs`, `keybind.rs`,
`snap.rs`, `attitude.rs`, `probe.rs`, `preview.rs`, `ui/mod.rs`, `ui/rail.rs`,
`ui/menu.rs`, `ui/inspector.rs`, `ui/window.rs` and `gallery/{mod,input,ui,catalog}.rs`
- plus the widget kit and theme they draw with in `crates/nova_ui/src`
(`widget/text_field.rs`, `widget/button.rs`, `theme.rs`), and `TASK.md` for the
settled scope. Nothing was built or run.

Every finding below stays inside the settled scope: no generic ECS inspector, no
asset browser, no project-file tree, no scale row.

Sizes: S is under an hour, M is half a day, L is more.

## 1. Verbs reachable one way and not the other

**1.1 Nothing on the keyboard deletes.**
- `crates/nova_editor/src/ui/mod.rs:107-114`, `crates/nova_editor/src/placement.rs:221-236`
- Today: Edit > Delete is the only way to remove a ship or an object. There is
  no `KeyCode::Delete` and no `KeyCode::Backspace` anywhere in the crate.
- Should be: Del runs `delete_selected_node`, and the menu row reads "Del" in
  its right column the way Frame Selection reads "F".
- Size: S

**1.2 The delete brush can be put down by key but never picked up by key.**
- `crates/nova_editor/src/ui/mod.rs:152-160`, `crates/nova_editor/src/placement.rs:244-252`, `crates/nova_editor/src/lib.rs:565-569`
- Today: Escape disarms the brush. Only the Ship > Delete Parts row arms it, so
  the gesture is asymmetric: a key takes the tool away, a menu gives it back.
- Should be: Del inside a ship arms the brush, matching the same key's meaning
  outside one (TASK slice 6 names exactly this).
- Size: S

**1.3 File > Save advertises a key that is bound to nothing.**
- `crates/nova_editor/src/ui/mod.rs:81-90`
- Today: the greyed row reads "Save   Ctrl+S". Pressing Ctrl+S does nothing and
  says nothing. The shortcut column promises a key that does not exist.
- Should be: the greyed row carries no shortcut until the key exists. A greyed
  verb may keep its name; it must not keep a key that is a lie.
- Size: S

**1.4 A rebind conflict cannot be resolved from the menu that started it.**
- `crates/nova_editor/src/ui/mod.rs:163-167`, `crates/nova_editor/src/keybind.rs:285-291`
- Today: Ship > Rebind Key arms the capture. The only on-screen record that a
  capture is running is the section's chip reading "press key"; the menu closes
  behind it, so the verb and its feedback are never on screen together.
- Should be: the armed rebind shows in the one persistent readout (see 2.1), so
  the builder is told what the editor is waiting for wherever they are looking.
- Size: S

**1.5 The placement verbs live only in a legend that can be switched off.**
- `crates/nova_editor/src/ui/mod.rs:1311-1332`, `crates/nova_editor/src/placement.rs:38-42`
- Today: R roll, F socket, Q pipette, wheel roll and Ctrl+wheel socket exist
  only as text in the key legend. View > Key Legend hides that line, after which
  the editor never mentions them again.
- Should be: the Ship menu lists the placement verbs with their keys, greyed
  unless a part is in hand. The menu becomes the durable record; the legend
  stays the fast one.
- Size: M

**1.6 Escape has five rungs and the legend names one of them.**
- `crates/nova_editor/src/lib.rs:540-570`, `crates/nova_editor/src/ui/mod.rs:1311-1332`
- Today: Escape closes an open menu, then the gallery, then a pending rebind,
  then the armed part, then the context, then pauses. The legend says only
  "Esc pause", "Esc leave" or "Esc put down" depending on the tool.
- Should be: the legend's Escape text names the rung the next press will take,
  including "Esc cancel" while a rebind is armed.
- Size: S

## 2. States the editor enters and never reports

**2.1 There is one status line on screen and only placement ever writes to it.**
- `crates/nova_editor/src/placement.rs:711-745`, `crates/nova_editor/src/ui/mod.rs:611-654`
- Today: `set_status` is called from `sync_placement_ghost` and nowhere else.
  Every other refusal in the editor goes to the log: Play inside a ship
  (`placement.rs:315-325`), a rebind conflict (`keybind.rs:287-290`), a rejected
  checkbox or segment (`ui/inspector.rs:818-820`, `838-840`), a colour slider
  that will not write (`ui/window.rs:487-495`), Add with no document
  (`placement.rs:192`), a minted id with no counter (`node.rs:628-631`).
- Should be: one "the editor says" line, written by every refusal, tinted the
  way the placement verdict already is. The widget exists; it needs a shared
  entry point and callers.
- Size: M

**2.2 Ship > Delete Parts writes an empty mark where the View rows write "off".**
- `crates/nova_editor/src/ui/menu.rs:443-463` against `crates/nova_editor/src/ui/menu.rs:303-325`
- Today: the tool row's right column reads "on" when armed and "" when not. The
  three View toggles read "on" and "off". Same column, same idea, two answers.
- Should be: "off" - or, better, the checkbox glyph for all four (see 6.4).
- Size: S

**2.3 A checkbox or segment that refuses an edit gives no sign at all.**
- `crates/nova_editor/src/ui/inspector.rs:805-841`
- Today: a text field writes its refusal under itself as `TextFieldError`
  (`apply_inspector_edits`, `ui/inspector.rs:780-800`). The flag and choice
  handlers beside it only `warn!`, so the widget just does not move.
- Should be: the same error line under the failed widget, or the shared status
  line from 2.1.
- Size: S

**2.4 An armed tool over empty space is invisible.**
- `crates/nova_editor/src/placement.rs:1198-1236`, `crates/nova_editor/src/ui/mod.rs:1330`
- Today: with the delete brush in hand, the only on-screen evidence is the red
  box `draw_delete_target` paints while the pointer is over a section, and the
  legend line. Point at space and the editor looks idle while the next click
  destroys a part.
- Should be: the armed tool is named in the persistent line: "delete brush" or
  the part's own name. A mode that changes what a click does must be readable
  without moving the pointer.
- Size: S

**2.5 Leaving a ship silently empties your hand.**
- `crates/nova_editor/src/placement.rs:878-882`
- Today: `disarm_outside_ship` drops the armed part or brush the moment the
  context leaves a ship. The reasoning is sound; the builder is not told.
- Should be: say it once, in the status line - "put the part down on the way
  out".
- Size: S

**2.6 A rebind conflict leaves the chip prompting forever.**
- `crates/nova_editor/src/keybind.rs:39`, `285-291`
- Today: press a key already driven by something else and the code logs
  "already driven by X - pick another key". The chip still reads "press key".
  The two presses that failed look exactly like the two the editor is waiting
  for.
- Should be: the chip says which action holds the key, then goes back to
  prompting.
- Size: S

**2.7 The move and turn handles vanish in three states with no explanation.**
- `crates/nova_editor/src/gizmo.rs:332-344`
- Today: `shown_on` returns `None` inside a ship, with any part armed, and with
  the gallery open. The rig simply disappears from the selected node.
- Should be: at least the armed-part case is worth saying, because it is the one
  the builder caused by accident - the status line already has to report the
  armed tool (2.4), which explains the missing handles for free.
- Size: S

**2.8 The greyed Play button never says why it is greyed.**
- `crates/nova_editor/src/ui/mod.rs:1084-1100`, `crates/nova_editor/src/placement.rs:315-325`
- Today: Play is disabled inside a ship, which is right. The refusal text -
  "Play compiles the whole scenario - leave the ship first" - sits in an
  observer that `InteractionDisabled` makes unreachable from the pointer.
- Should be: that sentence reaches the screen on hover, or the button reads
  "Play (leave the ship)".
- Size: S

**2.9 A minted id with no counter silently duplicates.**
- `crates/nova_editor/src/node.rs:620-635`
- Today: `mint_id` logs an error and returns ordinal 0. The tree then shows two
  rows with identical text and the builder has no way to tell them apart.
- Should be: route the error to the status line. The clash is rare; a silently
  broken save is not a cheap failure.
- Size: S

## 3. Dead ends

**3.1 Add offers the whole world palette while you are inside a ship.**
- `crates/nova_editor/src/ui/mod.rs:162-183`, `crates/nova_editor/src/placement.rs:178-206`
- Today: none of the six Add rows is context-gated. Inside a ship, pressing
  Asteroid spawns a node under the scenario, selects it, and hands you a
  selection you cannot see: `sync_ship_focus` (`node.rs:484`) has taken the
  world off the stage, `wanted_rows` (`ui/mod.rs:896-906`) does not list world
  objects inside a ship, and `shown_on` gives it no handles. The breadcrumb then
  reads "[SHIP] scenario / ship_1   selected asteroid_3".
- Should be: grey those rows inside a ship. `ShipMenuItem` and `sync_ship_menu`
  (`ui/menu.rs:414-431`) already do exactly this greying for the Ship menu, in
  the opposite direction. TASK slice 7 wants the menu to offer the ship's parts
  instead; the greying is the first half and it is nearly free.
- Size: S

**3.2 Add > Ship inside a ship silently moves you to a different ship.**
- `crates/nova_editor/src/placement.rs:141-161`, `crates/nova_editor/src/node.rs:839-864`
- Today: `spawn_ship_node` ends with `context.enter(ship)`. Press Add > Ship
  while editing `ship_1` and the stage, the tree and the rail all swap to
  `ship_2` with no prompt. The work is not lost, but nothing said it moved.
- Should be: gated by 3.1, or announced in the status line.
- Size: S

**3.3 The scenario node's Inspector is a titled empty box.**
- `crates/nova_editor/src/ui/inspector.rs:147-150`
- Today: selecting the root, or leaving a ship, gives a panel with the title
  "SCENARIO  editor_sandbox" and no rows. The comment explains why the panel
  stays; it does not fill it.
- Should be: the document's own facts - how many ships, how many objects, which
  ship the player flies. All of it is already queryable in `Document`.
- Size: M

**3.4 Selecting a part greys Edit > Delete without saying what to use instead.**
- `crates/nova_editor/src/ui/menu.rs:374-394`
- Today: `deletable` is ships and objects only. Select a thruster and the one
  row named "Delete" is grey, while the verb that removes it is called
  "Delete Parts" and lives in a different menu.
- Should be: Del and Edit > Delete arm the Delete Parts brush when the selection
  is a section, which is what TASK slice 6 asks for. Failing that, the grey row
  should say where the verb went.
- Size: S

**3.5 Double-clicking a world object repeats the single click.**
- `crates/nova_editor/src/ui/mod.rs:1034-1072`
- Today: a first click selects and frames; a second click inside 0.5s falls
  through the ship branch and selects and frames again. The gesture that means
  "enter" everywhere else means nothing here.
- Should be: the double-click frames tight on the object and gives the Inspector
  the focus, or is refused out loud. TASK slice 3 asks for something rather than
  nothing.
- Size: M

**3.6 File > New Scenario destroys the document with no confirmation and no undo.**
- `crates/nova_editor/src/node.rs:684-696`, `crates/nova_editor/src/ui/mod.rs:76-80`, `100-106`
- Today: the row is live, it is the first item in the first menu, and it
  despawns every root immediately. Undo and Redo directly under it are greyed,
  so there is no way back. Save is greyed too, so there is nothing on disk.
- Should be: a confirm step - the window host from `ui/window.rs` already exists
  and holds one tenant. Back to Main Menu (`ui/menu.rs:466`) tears down the same
  document and wants the same guard.
- Size: M

## 4. The Inspector: fields that deserve a typed editor

**4.1 A position is three numbers in one text box.**
- `crates/nova_editor/src/inspect.rs:894-913`, `crates/nova_editor/src/ui/inspector.rs:383-397`
- Today: `Position` and `Heading` are each a single `RowValue::Text` holding
  "12.00, 0.58, 1.17". There is no label on any axis, no placeholder, and one
  malformed character rejects all three numbers with "wants x, y, z". The code
  comment at `inspect.rs:883-890` argues for the single box on width grounds.
- Should be: three narrow boxes with x, y and z prefixes, per TASK slice 8. The
  panel is 240px with a 92px label column (`ui/inspector.rs:48-51`), so the
  value column is ~140px; three dense boxes fit if the axis letter is the
  prefix, not a second label.
- Size: M

**4.2 The Heading row never states its unit or its order.**
- `crates/nova_editor/src/inspect.rs:904-916`
- Today: the row reads "0.00, 45.00, 0.00". It is degrees of yaw, pitch and roll
  - a fact that lives only in a doc comment.
- Should be: "deg" after the box, or a "yaw, pitch, roll" placeholder. If 4.1
  lands, the three prefixes carry it.
- Size: S

**4.3 A section's Key row is dead text beside a live verb.**
- `crates/nova_editor/src/inspect.rs:850-853`
- Today: `Key` is a `RowValue::Fixed` readout. To change it the builder must
  find Ship > Rebind Key in a different panel, with the same section still
  selected.
- Should be: the Key row is the button that arms the rebind. Every guard it
  needs is already written in `sync_rebind_button` (`ui/mod.rs:1164-1190`).
- Size: S

**4.4 An object can be renamed and a ship cannot.**
- `crates/nova_editor/src/inspect.rs:831-842` against `867-881`
- Today: `object_rows` opens with a `Name` row; `ship_rows` opens with `Driver`
  and has no name row at all. Ships are identifiable only by their minted id.
- Should be: ships carry a name row too, so the two node kinds under the
  scenario read alike.
- Size: S

**4.5 The name you can edit is shown nowhere else.**
- `crates/nova_editor/src/inspect.rs:867-875`, `crates/nova_editor/src/ui/mod.rs:896-916`, `crates/nova_editor/src/ui/inspector.rs:171-183`
- Today: `ObjectNode.name` is written by the Inspector and read by nothing. The
  tree row, the breadcrumb and the Inspector title all use `NodeId`. A fresh
  asteroid is named "Asteroid" (`node.rs:285-288`) while its row says
  "asteroid_3", so one node carries two names and only the useless one is
  visible.
- Should be: the tree row shows the name and reveals the id on hover - the hover
  channel already exists (`ui/rail.rs`, `SceneRowHint`).
- Size: M

**4.6 Numbers are bare text with no unit and no bound.**
- `crates/nova_editor/src/inspect.rs` (`walk`, `number_text`), `crates/nova_editor/src/ui/inspector.rs:383-397`
- Today: a light's illuminance ("9000"), an asteroid's radius ("3"), an anchor's
  mass and a salvage area radius are all the same 64-character text box. Nothing
  says lux, metres or tonnes, and nothing rejects a negative radius until the
  scenario runs.
- Should be: a unit suffix per field and a clamp on the ones with a floor. The
  refusal path already exists; it only needs something to refuse.
- Size: M

## 5. Words

**5.1 "Delete" and "Delete Parts" are two gestures with one name.**
- `crates/nova_editor/src/ui/mod.rs:107-114`, `152-160`
- Today: Edit > Delete removes the selection at once. Ship > Delete Parts arms a
  brush that removes whatever is clicked next. Two menus, one word, opposite
  models.
- Should be: name the mode - "Delete Parts" becomes "Demolish" or "Delete
  Brush", and its right column marks it on or off like the tool it is.
- Size: S

**5.2 The breadcrumb repeats the selection it already showed.**
- `crates/nova_editor/src/ui/mod.rs:1132-1158`
- Today: with a ship selected at the scenario node the line reads
  "[SCENARIO] scenario   selected ship_1", and inside it
  "[SHIP] scenario / ship_1   selected ship_1" - the same id twice on one line.
- Should be: drop the "selected" clause when it names the node the path already
  ends at.
- Size: S

**5.3 The tree speaks identifiers.**
- `crates/nova_editor/src/ui/mod.rs:755-760`
- Today: `tree_text` splits only on `_section_`, so a part row reads
  "thruster  3" while every other row keeps its raw id: "beacon_veil",
  "sandbox_rim", "picket_warden", "ship_1". The trail column is used by one node
  kind out of four.
- Should be: split on the last underscore for minted ids, and show the authored
  name for authored ones (see 4.5).
- Size: M

**5.4 Two different things are both called "Ship".**
- `crates/nova_editor/src/ui/mod.rs:554` and the `MenuId::Ship` dropdown at `147-168`
- Today: the rail panel header "Ship" holds the attitude readout, the skin
  toggle and the look list. The top bar menu "Ship" holds Parts, Delete Parts
  and Rebind Key. Nothing distinguishes them.
- Should be: the rail block is the ship's settings and the menu is its verbs.
  Name the rail header "Ship Settings" - the component behind it is already
  called `ShipSettings`.
- Size: S

**5.5 The menu's right column carries three unrelated vocabularies.**
- `crates/nova_editor/src/ui/menu.rs:303-325`, `443-463`, `crates/nova_editor/src/ui/mod.rs:117-140`
- Today: the same column holds keys ("Ctrl+S", "F", "Tab"), toggle state
  ("on", "off"), and nothing. A builder scanning down View sees "on" where File
  taught them to expect a shortcut.
- Should be: state gets a glyph at the head of the row; the tail column is
  shortcuts only.
- Size: M

**5.6 An empty hull reports its turn rate as a dash.**
- `crates/nova_editor/src/attitude.rs:73-79`
- Today: a ship with no parts reads "Turn  -", which looks like a readout that
  failed. The neighbouring case says "Turn  no computer" in words.
- Should be: "Turn  no parts yet", matching the sentence beside it.
- Size: S

**5.7 Rebind Key opens a modal capture and takes no ellipsis.**
- `crates/nova_editor/src/ui/mod.rs:163-167`
- Today: "Parts...", "Save As..." and "Open..." take ellipses; "Rebind Key"
  starts a state that swallows the next keypress and takes none.
- Should be: "Rebind Key..." - it is the row on the list that most changes what
  the next input means.
- Size: S

## 6. Glyphs

**6.1 The Add menu has no icons while the tree it feeds has one per kind.**
- `crates/nova_editor/src/ui/mod.rs:176-183` against `798-808`
- Today: the six Add rows are bare text. `object_mark` already returns the exact
  glyph each of those objects will wear in the tree one second later.
- Should be: put the glyph in the row's lead column. The mapping is written, the
  row already has a left column, and it teaches the tree's alphabet at the one
  moment the builder is looking at both.
- Size: S

**6.2 Two glyphs each mean two things.**
- `crates/nova_editor/src/ui/mod.rs:777-808`
- Today: `!` is TORPEDO and BEACON; `o` is CONTROLLER and ASTEROID. The comment
  argues the two sets never share a tree, which holds today. It stops holding
  the moment a ship is inspected from the world list, and `#` (SPACESHIP) is
  already a world object that is a ship.
- Should be: one glyph per kind across both vocabularies. There are eleven kinds
  and the printable set is not short.
- Size: S

**6.3 The View toggles say "on" and "off" where the kit has a checkbox.**
- `crates/nova_editor/src/ui/menu.rs:303-325`
- Today: three rows spell the state as a word in the shortcut column.
  `nova_ui`'s `checkbox_glyph` exists and the editor's own rail uses `checkbox`
  for the skin toggle.
- Should be: the glyph, in the lead column, leaving the tail for keys. Fixes 2.2
  and 5.5 in the same pass.
- Size: S

**6.4 Entering the player's ship hides the fact that it is the player's.**
- `crates/nova_editor/src/ui/mod.rs:857-869`
- Today: the lead glyph is `@` for the entered ship and `>` or `-` for the
  player or AI driver. The three are mutually exclusive, so inside a ship the
  tree stops saying who flies it.
- Should be: `@` in the lead column and the driver mark in the trail column,
  which is empty for ship rows anyway.
- Size: S

**6.5 The menus have no icon column at all.**
- `crates/nova_editor/src/ui/menu.rs` (`menu_item_row`)
- Today: every row is a label and a right column. The tree, the rail and the
  gallery all carry marks; the menus are the one surface that does not.
- Should be: a lead column, filled first by the toggles (6.3) and the Add
  palette (6.1). It is one layout change and then per-row opt-in.
- Size: M

## 7. Affordances the code supports and never advertises

**7.1 The colour swatch is a button and does not look like one.**
- `crates/nova_editor/src/ui/inspector.rs:399-427`, `crates/nova_editor/src/ui/window.rs:175-200`
- Today: the swatch carries `Button` and `Hovered` and opens the picker; a
  second press closes it. It is painted as a flat block of colour with no
  border change, no cursor change and no hint.
- Should be: paint the hover, the way `list_row_colors` paints tree rows.
- Size: S

**7.2 Double-clicking the root row is the pointer's only way out of a ship.**
- `crates/nova_editor/src/ui/mod.rs:1050-1060`, `crates/nova_editor/src/ui/mod.rs:1318-1321`
- Today: inside a ship the legend says "Esc leave". The root row's
  double-click does the same thing and nothing says so, while the legend for the
  scenario context does advertise "LMB x2 enter".
- Should be: the inside-a-ship legend names both, or the root row's hover says
  "double-click to leave".
- Size: S

**7.3 Turning the skin on reveals a list nobody was promised.**
- `crates/nova_editor/src/ui/mod.rs:583-596`
- Today: the ship look list is spawned under the skin toggle and shown only
  while the toggle is on. A builder who never turns cladding on never learns
  that the shipped looks exist.
- Should be: keep the list visible and greyed when the skin is off. The greyed
  row is the advertisement.
- Size: S

**7.4 The pipette works on two surfaces and is announced on one.**
- `crates/nova_editor/src/placement.rs:423-458`, `crates/nova_editor/src/gallery/input.rs:100-120`, `crates/nova_editor/src/ui/mod.rs:1318-1330`
- Today: Q takes a part from a tile in the gallery and from a section on the
  stage. Both legends mention it, but the gallery's says "hover + Q: take it"
  and the stage's says "Q pick" and "Q pick a part" - three phrasings of one
  verb.
- Should be: one wording for the one gesture, in all three lines.
- Size: S
