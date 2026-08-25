# Everything left on the editor, in one list

Merged from `REVIEW-INSIDE.md` (43 findings, read off the code) and
`REVIEW-OUTSIDE.md` (44 findings, read off the screens), plus the queued slices
and the task's own open spine. Ten findings are the same defect seen twice and
are collapsed here; those carry **[both]**. The rest carry **[code]** or
**[eyes]** for which pass found them, and **[owner]** for items added by hand
afterwards. Sizes are the reviews' own: S under an
hour, M half a day, L more.

Nothing below is built. Slices 1-4 of the editor pass ARE built and are not
repeated here.

## 0. The plan, in order

Each number is one commit-sized step. Findings are named by their id in the
catalogue below.

1. **The key legend, whole.** Bound it between the rail and the Inspector and
   wrap instead of truncating; draw keys as the chips the gallery already has;
   make Escape name the rung the next press takes; give `F` one meaning or make
   the legend name the mode; move `Ship Skin reflows as you aim` onto the skin
   row; one grammar and one verb per gesture across the legend, the gallery
   footer and the hints. F1, F2, F3, F8, 1.6, 7.4/G6. S-M
2. **Greyed rows stop lying.** Drop `Ctrl+S` from a row bound to nothing; mark
   the unbuilt rows `soon` in the column they already have; Play says why on
   hover instead of only in a log line. 1.3/F5, 2.8/F4. S
3. **Del deletes (slice 6).** Del runs `delete_selected_node` at the scenario
   node and arms the delete brush inside a ship; Edit > Delete arms the brush
   when the selection is a section rather than greying with no forwarding; the
   rows carry the key. 1.1, 1.2, 3.4. S
4. **Add obeys the context (slice 7).** Grey the world palette and Add > Ship
   inside a ship - the greying machinery already exists in `sync_ship_menu`.
   Then the second half: inside a ship, Add offers that ship's parts.
   3.1/F7, 3.2. S then M
5. **One line the editor speaks through.** `set_status` becomes shared and every
   refusal calls it: Play inside a ship, a rebind conflict and which action
   holds the key, a refused checkbox or segment, a colour slider that will not
   write, Add with no document, an id clash, the name of the armed tool, the
   hand emptied on the way out of a ship, the handles suppressed by an armed
   part. 2.1, 2.3, 2.4, 2.5, 2.6, 2.7, 2.9. M
6. **Nothing destroys a document without asking.** A confirm on File > New
   Scenario and on Back to Main Menu, in the window host that already exists.
   3.6. M
7. **A selection you can see.** A phosphor outline on the selected node in every
   context, including inside a ship where the handle rig is deliberately
   suppressed. Then cross-highlight both ways: hover a tree row and the thing
   lights on the stage, hover the stage and the row lights. D1, D2. M each
8. **Transform in three boxes (slice 5).** Position and rotation each become
   three axis-tinted boxes matching the handle colours; the row called `Heading`
   is renamed `Rotation`, which is what it is; degrees and yaw/pitch/roll stated
   on screen; a `TRANSFORM` heading over the pair. 4.1/C1, 4.2/C2, 4.7, C3. M
9. **Numbers that know what they are.** A unit per field and a floor on the ones
   that have one, so a negative radius is refused where it is typed rather than
   at run time. 4.6. M
10. **Names that agree.** A name row for ships; the tree shows the authored name
    with the id on hover; the Key row becomes the button that arms the rebind.
    4.3, 4.4, 4.5. M
11. **The scenario node stops being an empty box.** Ships, objects, and which
    one the player flies. 3.3/C4. M
12. **One icon per kind, everywhere a mark is drawn.** Kill the shared `o` and
    `!`; line-art phosphor glyphs; the same glyph on the Add rows; a lead icon
    column in the menus, which lets the toggles use the checkbox glyph and frees
    the tail column for shortcuts alone; `@` and the driver mark stop competing
    for one column. 6.2/B1, 6.1, 6.4, 6.5, 2.2/6.3/C11, 5.5. M
13. **A tree you can read.** An ellipsis instead of a cut glyph; a stable
    ordinal order; a mark for a row that holds children; names rather than raw
    ids. B2, B3, B4, 5.3. M
14. **Placement that says what to do.** The refusal beside the ghost instead of
    at the window edge; every refusal names the key that resolves it; the
    neutral mate readout labelled and out of the error's slot. E1, E2, E3. S-M
15. **Sockets you can see.** Link points brighter and screen-space sized, with
    the one the ghost would take marked apart from the rest; keybind chips off
    the part they name, with a leader. E4, E5. M
16. **The gallery pass.** `1 part`; the filter visible when it is set;
    thumbnails framed to the part's bounds; sockets drawn on the focus preview;
    one case rule for stat keys; the focus card's empty half; plural chips
    against singular subtitles; and one look at the stray pixel.
    G1-G5, G7, G9, G10. M
17. **The wording pass.** One noun per thing: scenario (not SCENE / scenario /
    [SCENARIO]), skin (not look / style / cladding), Ship Settings against the
    Ship menu, `Add > New Ship`; Delete Parts named as the mode it is; Rebind
    Key takes its ellipsis; one treatment for the kind tag.
    B5, 5.4/F6, C8, 5.1, 5.7, C5. S each
18. **The rest of the rail.** The attitude readout names its number and its fix
    and gains mass, thrust, hp and part count; an empty hull says "no parts yet";
    the look list stays visible and greyed with a swatch per row; `Placeholder`
    goes behind a debug feature; the colour swatch paints its hover.
    C6, C7, 5.6, 7.3, C9, C10, 7.1. M
19. **Navigation extras.** A clickable breadcrumb path; front / top / side / iso
    presets; an axis rose in the viewport corner; a double-click on a world
    object doing something. F9, D4, D3, 3.5. M

## 1. The task's own open spine

Not editor polish - the scenario work this task was opened for.

- **Slice 1 tail: save and reload.** Lives in `20260824-120524`, which now has
  world nodes to save. The rest of slice 1 landed; trigger-area gizmos are done
  as of the stage pass.
- **Slice 2: objectives and win/lose.** Attach a simple objective set (destroy
  X, reach Y, survive T) with the posted-objective actions and play it through.
  HELD until `Sequence` lands - see below.
- **Slice 3: events surfacing.** Expose the event/handler list without drowning
  the panel. HELD: it leans on `Sequence` (`20260820-223059`), and surfacing
  nineteen handlers where a story beat should be one is the wrong panel to
  build. Slices 2 and 3 stay OPEN and unstarted until that task lands.

## 2. The editor pass: slices still open

- **Slice 5. More per-object settings.** The tail of "an inspector for an
  object's own fields" owed by slice 1. Folds into the typed editors. Sections
  4 and 5 below are its content.
- **Slice 6. Keys for the verbs that have none.** Queued. Section 3 below.
- **Slice 7. Add obeys the context.** Queued. Items 3.1/F7 and 3.2 below.
- **Slice 8. Polish, from this review.** Queued. Everything else below.

## 3. Keys, and the legend that teaches them

- **[code] 1.1 Nothing on the keyboard deletes.** No `KeyCode::Delete` in the
  crate; Edit > Delete is the only way. Del should run `delete_selected_node`
  and the row should read "Del". S
- **[code] 1.2 The delete brush is put down by key and picked up only by menu.**
  Escape disarms it; only Ship > Delete Parts arms it. Del inside a ship should
  arm it. S
- **[both] 1.3/F5 File > Save advertises Ctrl+S, bound to nothing.** A greyed
  verb may keep its name; it must not keep a key that is a lie. Greyed
  Save/Open/Undo/Redo also read as "you cannot save" - they want a `soon` mark
  in the column they already have. S
- **[code] 1.4 A rebind conflict cannot be resolved from the menu that started
  it.** The menu closes behind the capture, so the verb and its feedback are
  never on screen together. S
- **[code] 1.5 The placement verbs exist only in a legend you can switch off.**
  R, F, Q, wheel and Ctrl+wheel are named nowhere else. The Ship menu should
  list them, greyed unless a part is in hand. M
- **[code] 1.6 Escape has five rungs and the legend names one.** It should name
  the rung the next press takes, including "cancel" during a rebind. S
- **[eyes] F1 The legend runs past the window edge and is cut mid-word**
  (`Esc le`), and draws across the Inspector's border. Bound it to the gap
  between rail and Inspector and wrap to two lines. S
- **[eyes] F2 Keys are plain text where the app already has key chips.** The
  gallery draws boxed chips for Esc and Enter; the legend is the one surface
  that does not. S
- **[eyes] F3 `F` means "cycle socket" armed and "frame" selecting, and the
  legend never says so.** Two keys, or a legend that names the mode. S
- **[eyes] F8 A sentence hides in the key legend** (`Ship Skin reflows as you
  aim`). Move it to the skin row. S
- **[both] 7.4/G6 One gesture, three grammars.** `hover + Q: take it`,
  `Q pick`, `Q pick a part`; and `arrows: select` against `LMB place`. One form,
  one verb per act. S

## 4. Modes and states nobody reports

- **[code] 2.1 One status line exists and only placement writes to it.** Play
  inside a ship, rebind conflicts, rejected checkboxes and segments, a colour
  slider that will not write, Add with no document, a minted id with no counter
  - all go to the log. One "the editor says" line, one entry point, many
  callers. M
- **[code] 2.3 A checkbox or segment that refuses an edit gives no sign.** Text
  fields write a `TextFieldError`; the flag and choice handlers only `warn!`. S
- **[code] 2.4 An armed tool over empty space is invisible.** The delete brush
  shows only as a red box when the pointer is already over a part. Name the
  armed tool in the persistent line. S
- **[code] 2.5 Leaving a ship silently empties your hand.** `disarm_outside_ship`
  is right; say it once. S
- **[code] 2.6 A rebind conflict leaves the chip prompting forever.** The log
  knows which action holds the key; the chip does not say. S
- **[code] 2.7 The move and turn handles vanish in three states with no
  explanation.** Inside a ship, with a part armed, with the gallery open. The
  armed-part case is the accidental one. S
- **[both] 2.8/F4 Play greys with no reason on screen.** The sentence exists in
  an observer `InteractionDisabled` makes unreachable. Hover text, or
  "Play (leave the ship)". S
- **[code] 2.9 A minted id with no counter silently duplicates.** Two rows,
  identical text, no way to tell them apart. S
- **[both] 2.2/6.3/C11 Three vocabularies for one mark.** Delete Parts writes
  "" where the View rows write "off"; the skin checkbox is an `x`, which in a
  terminal reads as "clear". One glyph, in a lead column, for all of them. S

## 5. Navigation, context and dead ends

- **[both] 3.1/F7 Add offers the whole world palette inside a ship.** Pressing
  Asteroid there spawns a node under the scenario, selects it, and hands you a
  selection the stage has hidden, the tree does not list and the gizmo will not
  handle. The greying machinery already exists in `sync_ship_menu`. S for the
  greying, M for Add offering the ship's parts instead.
- **[code] 3.2 Add > Ship inside a ship silently moves you to a different
  ship.** `spawn_ship_node` ends with `context.enter`. S
- **[code] 3.4 Selecting a part greys Edit > Delete and does not say where the
  verb went.** Del and Edit > Delete should arm the brush when a section is
  selected. S
- **[code] 3.5 Double-clicking a world object repeats the single click.** The
  gesture that means "enter" everywhere else means nothing here. M
- **[code] 3.6 File > New Scenario destroys the document with no confirm and no
  undo,** with Save greyed so there is nothing on disk. The window host exists
  and holds one tenant. Back to Main Menu wants the same guard. M
- **[code] 7.2 Double-clicking the root row is the pointer's only way out of a
  ship, and nothing says so.** S
- **[eyes] F9 The breadcrumb is a label, not a control, in the far corner.**
  This editor's whole model is enter and leave; the path is the natural control
  for it. The selection fact wants its own chip. M
- **[code] 5.2 The breadcrumb repeats the selection it already showed**
  (`[SHIP] scenario / ship_1   selected ship_1`). S
- **[eyes] D4 No view presets.** Front/top/side/iso. Sockets are axis-aligned,
  so an axis-true view is how a mate gets checked. S

## 6. The Inspector

- **[both] 4.1/C1 A Vec3 is one text box.** `Position` and `Heading` each hold
  "x, y, z" in one field; real values wrap to two lines and break the row
  rhythm on the only row that matters. Three boxes, tinted to the handle
  colours (red X, phosphor Y, blue Z). Drag-to-scrub on the label is the
  Blender habit and is nearly free once the boxes exist. M
- **[both] 4.2/C2 Heading states no unit and no order.** Degrees, yaw/pitch/roll
  - both facts live only in a doc comment. S
- **[owner] 4.7 `Heading` is the wrong word - rename it `Rotation`.** It is the
  node's world rotation, and every other editor calls that rotation. Folds into
  4.1. S
- **[code] 4.3 A section's Key row is dead text beside a live verb.** The row
  should be the button that arms the rebind; every guard is already written. S
- **[code] 4.4 An object can be renamed and a ship cannot.** S
- **[code] 4.5 The name you can edit is shown nowhere else.** `ObjectNode.name`
  is written by the Inspector and read by nothing - the tree, the breadcrumb and
  the title all use the minted id, so one node carries two names and only the
  useless one is visible. M
- **[code] 4.6 Numbers are bare text with no unit and no bound.** Illuminance,
  radius, mass and area radius are the same 64-character box; nothing rejects a
  negative radius until the scenario runs. M
- **[both] 3.3/C4 The scenario node's Inspector is a titled empty box.** The
  document's own facts - ships, objects, which one the player flies - are all
  queryable. A panel that goes blank reads as the panel breaking. M
- **[eyes] C3 The transform rows get no group heading while nested rows do.** S
- **[eyes] C5 The kind tag is drawn three ways:** `[SHIP]` bracketed in the
  crumb, `SHIP` bare amber in the title, `TURRET` bare grey in the hint. S
- **[eyes] C6 The attitude readout is a diagnosis with no remedy.**
  `3.48 rad/s2` over `structure-limited`, two cramped lines, no label on the
  number. Name the number and name the fix. M
- **[eyes] C7 One number for a whole ship.** Mass, thrust, hp and part count
  belong beside it; Nova already computes them. M
- **[code] 5.6 An empty hull reports its turn rate as `-`,** which looks like a
  failed readout, next to a case that says "no computer" in words. S
- **[eyes] C9 A purely visual choice is five text rows.** The look list wants a
  swatch or thumbnail per row; the gallery already renders part thumbnails. M
- **[eyes] C10 `Placeholder` ships in the look list.** S
- **[code] 7.3 Turning the skin on reveals a list nobody was promised.** Keep it
  visible and greyed when the skin is off - the greyed row is the
  advertisement. S
- **[code] 7.1 The colour swatch is a button and does not look like one.** No
  hover paint, no border change. S

## 7. The scene tree

- **[both] 6.2/B1 Glyphs: 11px muted ASCII, and two kinds share one.** `o` is
  controller AND asteroid, `!` is torpedo AND beacon; `#` is already a world
  object that is a ship, so the "the two sets never share a tree" argument is
  thin. One icon per kind, phosphor line art - a stroked 12px glyph is still a
  terminal. M
- **[code] 6.1 The Add menu has no icons while the tree it feeds has one per
  kind.** `object_mark` already returns the exact glyph. Put it in the row's
  lead column and Add teaches the tree's alphabet. S
- **[code] 6.4 Entering the player's ship hides the fact that it is the
  player's.** `@` replaces `>`/`-`; the trail column is empty for ship rows. S
- **[code] 6.5 The menus have no icon column at all** - the one surface with no
  marks. M
- **[eyes] B2 Labels clip mid-glyph with no ellipsis** (`basic_cont`,
  `reinforced_`). The tail is the part that differs between rows. S
- **[eyes] B3 The ordinal column is unsorted and gappy** (`1, 7, 2, 4...`), and
  it is the only thing telling six `reinforced_` rows apart. S
- **[eyes] B4 A row with children looks exactly like a row without.** A
  disclosure mark, or a child count in the trailing column. M
- **[code] 5.3 The tree speaks identifiers.** `tree_text` splits only on
  `_section_`, so parts read "thruster 3" and everything else keeps its raw id.
  The trail column is used by one node kind out of four. M
- **[eyes] B5/[code] 5.4 Three names for the same node, two menus called
  "Ship".** The panel says `SCENE`, the row says `scenario`, the crumb says
  `[SCENARIO]`. Separately: the rail block "Ship" (settings) and the menu "Ship"
  (verbs) share a name, and `Add > Ship` sits one row away from the `Ship` menu.
  S each.

## 8. The stage

- **[eyes] D1 A selection inside a ship is invisible.** `gizmo.rs` suppresses
  the rig whenever a ship is entered and there is no outline anywhere in the
  editor. The strongest habit a Godot/Blender/Unity user brings, and the one
  that is missing. M
- **[eyes] D2 Hovering a tree row does nothing on the stage.** Cross-highlight
  both ways - on a hull of nine identical rows it is the only way to tell which
  row is which. M
- **[eyes] D3 Inside a ship there is no spatial reference at all.** The world
  grid is suppressed there, correctly; an always-on axis rose in a viewport
  corner would cost the aesthetic nothing. M
- **[eyes] E5 Keybind chips cover the part they name.** The amber pill sits over
  the turret and is the only amber thing on the stage. Several bound parts close
  together will stack into a pile. S

## 9. Placement feedback

- **[eyes] E1 The refusal chip is 200px from the part it refuses.** Colour and
  redundancy are right; only the distance is wrong. S
- **[eyes] E2 Refusals state a verdict, never a fix.** "socket is ambiguous" is
  a compiler error; "two sockets are equally close - roll (R) to choose" is
  actionable. The editor already knows which key resolves each case. S
- **[eyes] E3 The neutral mate readout sits in the error's slot,** as raw ids,
  so the eye has to re-read it to find out whether this is news or a fault. M
- **[eyes] E4 Link points are near-invisible.** 6-10px mid-green ticks on a grey
  hull read as scratches, and they vanish when zoomed out. They are the only
  thing placement snaps to: brighter, screen-space sized, with the socket the
  ghost would take marked differently. M

## 10. The parts gallery

- **[eyes] G1 `1 parts page 1/1`.** S
- **[eyes] G2 An active filter is invisible except in the count.** S
- **[eyes] G3 Thumbnails are not framed to the part** - a hull fills its tile, a
  PDC turret is a speck. Tile size reads as part size and is wrong. M
- **[eyes] G4 The preview does not show sockets,** which is the question the
  preview exists to answer. The gizmos already exist on the stage. M
- **[eyes] G5 Stat keys are lowercase here and Title Case in the Inspector.** S
- **[eyes] G7 The focus card leaves ~400px of black under it.** S
- **[eyes] G8 Three part-naming conventions in one grid** (`Basic Thruster
  Section`, `PDC Turret (Kinetic)`, `Racer // Wing Starboard`). Content, not
  code, but it is what the user reads first. S
- **[eyes] G9 Category chips are plural, tile subtitles singular.** S
- **[eyes] G10 A stray lit pixel under the header,** same spot in both gallery
  shots, so not a starfield speck. S

## 11. Words

- **[code] 5.1 "Delete" and "Delete Parts" are two gestures with one name** -
  one immediate, one a brush. Name the mode. S
- **[eyes] C8 Four words for one thing:** the row says `Ship Skin`, the list is
  a look list, the values are style ids, the comments say cladding. S
- **[code] 5.5 The menu's right column carries three unrelated vocabularies** -
  keys, toggle state and nothing. Shortcuts only; state gets a lead glyph. M
- **[code] 5.7 Rebind Key takes no ellipsis** while "Parts..." and "Save As..."
  do, and it is the row that most changes what the next input means. S

## 12. Ruled out, on purpose

From the outside pass, kept here so they are not re-proposed: dockable
rearrangeable panels; a generic property grid over reflected components; grey
IDE chrome and icon-only ribbons; a Blender-style modal keymap. The editor
already has two modal traps and would need a much louder mode readout before it
earned more.
