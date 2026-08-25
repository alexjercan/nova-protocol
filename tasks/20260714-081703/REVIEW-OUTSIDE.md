# Editor review: outside eyes

A fresh-eyes usability pass on the Nova editor. The reviewer has used Godot,
Blender and Unity and has never seen this program. The question is what the
editor teaches in five minutes.

Looked at: five screenshots at 1024x768 - the ship context with the look list
(`editor-skin.png`), a hovered tree row (`tmp-hint.png`), a refused placement
(`editor-placement-refused.png`), the parts grid (`editor-gallery.png`) and the
parts focus card (`editor-gallery-focus.png`). Then, for vocabulary only,
`ui/mod.rs`, `ui/rail.rs`, `ui/menu.rs`, `ui/inspector.rs`, `inspect.rs`,
`snap.rs`, `gizmo.rs`, `keybind.rs` and `gallery/catalog.rs`.

Nothing was built or run. Findings only.

Out of scope per TASK.md and not proposed anywhere below: a generic ECS
component inspector, an asset browser, a project-file tree, a scale handle.

## Top ten by value per effort

1. The key legend runs off the right edge and is cut mid-word (F1).
2. Position and Heading are one text box holding three numbers (C1).
3. A selection inside a ship is invisible on the stage (D1).
4. `Turn 3.48 rad/s2 / structure-limited` is a diagnosis with no remedy (C6).
5. Tree rows lead with ASCII glyphs that two kinds share (B1).
6. Play greys with no reason given (F4).
7. The refusal chip is 200px away from the part it refuses (E1).
8. Tree labels clip mid-glyph with no ellipsis (B2).
9. Four words for one thing: skin, look, style, cladding (C8).
10. Link points are near-invisible ticks on a grey hull (E4).

## A. What already works

Worth keeping as-is, so the list below is not read as a rewrite.

- Play dead centre of the bar, present in every context, is the right call and
  matches where a Godot or Unity user looks.
- The hover hint that reveals a clipped id plus its kind word is the correct
  answer to a 150px rail. It reads instantly.
- The parts grid is the strongest screen in the editor: real thumbnails, a
  category row, a filter, a focus card with stats and one primary action.
- Refusal is shown three ways at once - a red bounds box, a red chip, red text.
  The redundancy is right, only the placement is wrong (E1).

## B. The scene tree

### B1. Type marks are ASCII glyphs, and two kinds share one
- Where: left rail, `SCENE`, the muted column in front of every label.
- See: `*` scenario, `@` entered ship, `>` player ship, `-` AI ship, `=` hull,
  `o` controller, `^` thruster, `+` turret, `!` torpedo, `?` unknown part; and
  for objects `x` anchor, `o` asteroid, `#` spaceship, `!` beacon, `%` salvage,
  `~` light. At 11px in muted phosphor they are almost not there. `o` means two
  things and so does `!`.
- Expect: one icon per kind, in phosphor line art. A terminal skin does not
  force ASCII - a 12px stroked glyph is still a terminal. Godot's tree is
  scanned by icon, never by the first character of the label.
- Size: M.

### B2. Labels clip mid-glyph, with no ellipsis
- Where: tree rows.
- See: `basic_cont` clipped so the next glyph's stem reads as a colon, and
  `reinforced_` ending on an underscore. The row looks like an unfinished word
  rather than a shortened one.
- Expect: a trailing ellipsis, or middle truncation that keeps the tail. The id
  tail is the part that differs between rows.
- Size: S.

### B3. The ordinal column is unsorted and gappy
- Where: right edge of each tree row.
- See: `1, 7, 2, 4, 5, 6, 8, 9` down one ship. Reads as a list that lost a row.
- Expect: a stable order - kind then ordinal, or placement order. Six rows all
  reading `reinforced_` are told apart by that column alone, so it has to be
  countable.
- Size: S.

### B4. A row with children looks exactly like a row without
- Where: tree, at the scenario node.
- See: `ship_1` and `ship_2` are flat rows. Only the entered ship expands.
- Expect: the Godot disclosure triangle, or at least a child count in the
  trailing column. Isolation on enter is a good rule, but a builder still has
  to know which nodes hold something before entering them.
- Size: M.

### B5. Three names for the same node
- Where: rail header, root row, breadcrumb.
- See: the panel says `SCENE`, the root row says `scenario`, the breadcrumb tag
  says `[SCENARIO]`.
- Expect: one noun everywhere. `SCENARIO` is the honest one - this editor edits
  a scenario, and borrowing Godot's word for it buys nothing.
- Size: S.

## C. The Inspector

### C1. A Vec3 is one text box
- Where: right panel, `Position` and `Heading`.
- See: `0, 0, 0` in one field. With real values, `3.039, 3.389, 0` wraps to two
  lines and grows the box, so the row rhythm breaks on the only row that
  matters.
- Expect: three boxes. Tint them to the handle colours already defined in
  `gizmo.rs` - red X, phosphor Y, blue Z - so the field and the arm you drag
  are the same colour. Drag-to-scrub on the label is the Blender and Unity
  habit and costs nothing extra once the boxes exist.
- Size: M.

### C2. Heading carries no unit and no axis names
- Where: right panel, `Heading`.
- See: `0, 0, 0`. Degrees per the code, yaw/pitch/roll order, none of it on
  screen.
- Expect: `Heading (deg)` and per-box marks. Degrees against radians is the one
  ambiguity a builder cannot recover from by experiment.
- Size: S.

### C3. The transform rows get no group heading, nested rows do
- Where: right panel.
- See: `Driver`, then `Position` and `Heading` bare; deeper config rows draw an
  uppercase heading with a rule under it.
- Expect: a `TRANSFORM` heading over the two pose rows. Same rule for every
  group, or the headings stop meaning "a group starts here".
- Size: S.

### C4. The Inspector is empty at the scenario node
- Where: right panel, at the root.
- See: the header and a title, nothing under them.
- Expect: the scenario's own fields, or one muted line saying it has none yet.
  A panel that goes blank when you back out reads as the panel breaking.
- Size: S.

### C5. The kind tag is drawn three different ways
- Where: breadcrumb, inspector title, row hint.
- See: `[SHIP]` in brackets, `SHIP` bare in amber, `TURRET` bare in muted grey.
- Expect: one treatment - the kind icon plus the word, same colour, same case,
  in all three places.
- Size: S.

### C6. The attitude readout is a diagnosis with no remedy
- Where: left rail, `SHIP` block.
- See: `Turn  3.48 rad/s2` over `structure-limited`, muted, two cramped lines,
  no label on the number.
- Expect: name the number and name the fix. `structure-limited` tells a builder
  the hull is the problem but not what to do about it; `no computer` at least
  implies one. A short meter with the limiter named under it would say both
  faster than the two lines do.
- Size: M.

### C7. One number for a whole ship
- Where: left rail, `SHIP` block.
- See: turn rate only.
- Expect: mass, thrust, hp and part count beside it. This is the block a
  builder watches while placing, and it currently answers one question out of
  four. Nova already computes these for the sim.
- Size: M.

### C8. Four words for one thing
- Where: rail, key legend, code.
- See: the row says `Ship Skin`, the list is a look list, the values are style
  ids, the doc comments say cladding, and the legend says
  `Ship Skin reflows as you aim`.
- Expect: one noun on screen. `Skin` is fine; then the list header is
  `Skin`, not a fifth word.
- Size: S.

### C9. A purely visual choice is five text rows
- Where: rail, under `Ship Skin`.
- See: `Industrial`, `Armoured`, `Civilian`, `Salvage`, `Placeholder`.
- Expect: a swatch or a small thumbnail per row. The editor already renders
  part thumbnails in the gallery, so the machinery exists. Choosing a look by
  reading its name is the one case where text is strictly slower.
- Size: M.

### C10. `Placeholder` ships in the look list
- Where: rail, last look row.
- See: a debug-sounding entry sitting beside four authored names.
- Expect: hidden outside a debug feature, or renamed to something a player
  would pick on purpose.
- Size: S.

### C11. The skin checkbox glyph is an `x`
- Where: rail, `Ship Skin` row.
- See: a bordered box with `x` in it.
- Expect: a filled block or a check. In a terminal skin an `x` in a box also
  reads as "close" or "clear", which is the opposite of on.
- Size: S.

## D. The stage

### D1. A selection inside a ship is invisible on the stage
- Where: 3D viewport, inside a ship.
- See: nothing marks the selected section. `gizmo.rs` suppresses the handle rig
  whenever a ship is entered, and there is no outline anywhere in the editor.
  The only feedback is the tree row and the Inspector.
- Expect: a phosphor outline or rim glow on the selected node in every context.
  This is the single strongest habit a Godot, Blender or Unity user brings, and
  it is the one that is missing.
- Size: M.

### D2. Hovering a tree row does nothing on the stage
- Where: tree and viewport.
- See: the hint panel appears beside the row; the ship does not react.
- Expect: hover in the tree dims-or-glows the thing in the viewport, and hover
  in the viewport marks the row. On a hull of nine identical `reinforced_` rows
  this is the only way to tell which row is which.
- Size: M.

### D3. Inside a ship there is no spatial reference at all
- Where: viewport, ship context.
- See: a grey hull on pure black. The world grid is suppressed inside a ship.
- Expect: an always-on axis rose in a viewport corner, and some ground or
  bounding reference in the ship context too. A wireframe rose in phosphor is
  exactly on theme - this is the one borrowing from Blender that costs the
  aesthetic nothing.
- Size: M.

### D4. No view presets
- Where: `View` menu.
- See: Key Legend, Link Points, World Grid, Frame Selection.
- Expect: front / top / side / iso. Sockets are axis-aligned, so an axis-true
  view is how a builder checks a mate. Numpad-style keys optional; the menu
  rows are the cheap half.
- Size: S.

## E. Placement and feedback

### E1. The refusal chip is far from the refused part
- Where: bottom centre of the screen, y=727 of 768.
- See: `socket occupied` in a red pill at the bottom edge while the red bounds
  box is in the middle of the screen. The eye is on the ghost, not the footer.
- Expect: the message beside the ghost, or following the pointer. It is already
  correct in colour and in redundancy - only the distance is wrong.
- Size: S.

### E2. Refusals state a verdict, never a fix
- Where: same chip.
- See: `socket is ambiguous`, `nothing may block an exit`,
  `this part has no sockets`.
- Expect: reason plus remedy. "two sockets are equally close - roll (R) to
  choose" is actionable; "socket is ambiguous" is a compiler error. The editor
  already knows which key resolves each case.
- Size: S.

### E3. The neutral mate readout is raw ids in the error's slot
- Where: same row, when placement is legal.
- See: `<target socket id> <- <part socket id>`, muted, in exactly the place a
  red refusal appears a moment later.
- Expect: label it (`mating`), and put the neutral readout somewhere the eye
  does not have to re-read to find out whether this is news or an error.
- Size: M.

### E4. Link points are near-invisible
- Where: viewport, green ticks around the hull.
- See: 6-10px mid-green dashes on a light grey hull. They read as scratches on
  the model, not as sockets, and they vanish when zoomed out.
- Expect: brighter rings or discs, screen-space sized so they hold at any zoom,
  with the socket the ghost would take marked differently from the rest.
  These are the only thing placement snaps to.
- Size: M.

### E5. Keybind chips cover the part they name
- Where: viewport, the amber `LMB` pill on the turret.
- See: the pill sits over the turret and hides its art. It is the only amber
  thing on the stage, so it wins attention over the model.
- Expect: an offset with a short leader line, or chips revealed on hover or
  while the Ship menu is open. Several bound parts close together will stack
  into an unreadable pile as drawn.
- Size: S.

## F. Menus, keys and wording

### F1. The key legend runs off screen and is cut mid-word
- Where: bottom strip, left of the rail edge to past the window edge.
- See: `... Rebind acts on the selection   Esc le` - the line ends at x=1024.
  In the placement shot it also crosses the Inspector's left border and draws
  over the panel edge.
- Expect: the legend bounded by the space between the rail and the Inspector,
  wrapping to two lines rather than truncating. A cut key name is worse than a
  dropped one. This is the clearest defect in the shots.
- Size: S.

### F2. Keys are plain text where the editor already has chips
- Where: bottom strip.
- See: `LMB place   wheel roll   Ctrl+wheel socket   R roll   F socket ...` -
  a wall of 12px muted phosphor with only spacing separating key from verb.
- Expect: the boxed key chip the gallery header already draws for `Esc` and the
  focus card for `Enter`. The chip form exists in this codebase; the legend
  is the one place that does not use it.
- Size: S.

### F3. `F` means two different things and the legend never says so
- Where: bottom strip, two modes.
- See: `F socket` while a part is armed, `F frame` while selecting.
- Expect: two keys, or a legend that names the mode. A key that silently
  changes meaning is the worst thing to hand a five-minute learner.
- Size: S.

### F4. Play greys with no reason
- Where: top bar centre, inside a ship.
- See: a dim `Play`. Nothing says why. The reason exists in a log line
  ("leave the ship first").
- Expect: the reason on hover, or a muted line under the button. Greyed and
  silent is what makes a user think the build is broken.
- Size: S.

### F5. Greyed Save reads as "you cannot save"
- Where: `File` menu.
- See: `Save Ctrl+S`, `Save As...`, `Open...` all dim. `Edit` shows `Undo` and
  `Redo` dim as well.
- Expect: a `soon` mark in the right-hand column those rows already have. The
  intent - say what the editor will be - is right; the execution reads as data
  loss to someone who has been building for ten minutes.
- Size: S.

### F6. Two menu entries named "Ship", one row apart, meaning different things
- Where: top bar.
- See: the `Ship` menu holds verbs of the ship you are inside; `Add > Ship`
  creates one.
- Expect: rename the menu to the context it acts on, or the item to
  `New Ship`. Two identical words a centimetre apart is a coin flip.
- Size: S.

### F7. `Add` offers what cannot go where you are standing
- Where: `Add` menu, inside a ship.
- See: Ship plus the five object kinds, none of which can be a child of a ship.
- Expect: Add lists the children of the node you are in - the parts catalog
  inside a ship. Confirmed independently from the outside; already on the
  slice list.
- Size: M.

### F8. A sentence hides in the key legend
- Where: bottom strip, ship context.
- See: `Ship Skin reflows as you aim` between two key bindings.
- Expect: move it to the skin row as a hint, or drop it. It is not a key, and
  it makes the reader parse the whole line to find out.
- Size: S.

### F9. The breadcrumb is a label, not a control, and it is in the far corner
- Where: top right.
- See: `[SHIP] scenario / ship_1`, and with a selection, three spaces then
  `selected ship_1` in the same string.
- Expect: clickable crumbs - press `scenario` to leave the ship. Every editor
  named in this review makes that path navigable, and this editor's whole model
  is enter and leave. The selection fact belongs in its own chip, not appended
  to a sentence.
- Size: M.

## G. The parts gallery

### G1. `1 parts page 1/1`
- Where: gallery header, after a filter narrows the grid.
- Expect: `1 part`. One line, and it is the first thing the eye lands on after
  typing a filter.
- Size: S.

### G2. An active filter is invisible except in the count
- Where: gallery header.
- See: `reinforced` typed in the field, `All` still lit, the grid down to one
  tile. Nothing says the grid is filtered except the number.
- Expect: the field lit while non-empty, a clear affordance, and per-category
  counts on the chips.
- Size: S.

### G3. Thumbnails are not framed to the part
- Where: gallery grid.
- See: a hull cube fills its tile, a PDC turret is a speck in the middle of
  its own, the basic thruster overflows its.
- Expect: frame each thumbnail to the part's bounds, the way Unity and Blender
  asset previews do. As drawn, tile size reads as part size and is wrong.
- Size: M.

### G4. The preview does not show sockets
- Where: focus card preview.
- See: the part turning on black, with `sockets 6` written on the card.
- Expect: draw the link points on the preview. Where a part can attach is the
  question the preview exists to answer, and the editor already draws these
  gizmos on the stage.
- Size: M.

### G5. Stat keys are lowercase here, Title Case in the Inspector
- Where: focus card versus right panel.
- See: `kind`, `size`, `hp`, `sockets`, `role` against `Driver`, `Position`,
  `Heading`.
- Expect: one case rule for field names across the app.
- Size: S.

### G6. Two grammars for keys, one app
- Where: gallery footer versus editor legend.
- See: `hover + Q: take it`, `arrows: select`, `Enter: place` in the gallery;
  `LMB place`, `Q pick`, `Tab parts` in the editor.
- Expect: one form. Also one verb per act - the same gesture is called take,
  pick and place in three places.
- Size: S.

### G7. The focus card leaves half the screen empty
- Where: focus view.
- See: the card top-right, the preview left of centre, roughly 400px of black
  under the card.
- Expect: centre the preview in the space the card leaves, and let the stat
  block breathe into the empty height. It currently reads as a layout that ran
  out of content.
- Size: S.

### G8. Three part-naming conventions in one grid
- Where: gallery tiles.
- See: `Basic Thruster Section`, `PDC Turret (Kinetic)`, `Racer // Wing
  Starboard`, `Torpedo Bay (Serpent)`.
- Expect: one convention. Content, not code, but it is what the user reads
  first and it makes the catalog look assembled from three sources.
- Size: S.

### G9. Category chips are plural, tile subtitles are singular
- Where: gallery header versus tiles.
- See: `Weapons` on the chip, `weapon` under the tile; `Structure` against
  `structure`.
- Expect: match them, so a builder can tell that the chip and the subtitle name
  the same set.
- Size: S.

### G10. A stray dot under the header
- Where: gallery, roughly x=150 y=63, in both gallery shots at the same spot.
- See: one lit pixel just below the header rule, at the left edge of the grid
  area. Fixed position across shots, so not a starfield speck.
- Expect: nothing there. Worth one look at whatever node sits at the top-left
  of the grid.
- Size: S.

## H. What to borrow, and what not to

Worth borrowing here:

- Per-type icons in the tree (Godot). Line-art phosphor glyphs, not silhouettes.
- Three-box vectors with axis colours matched to the handles (all three).
- Cross-highlight between tree and viewport (all three).
- A clickable breadcrumb path (Godot). This editor's model is enter and leave;
  the path is the natural control for it.
- An axis rose in the viewport corner (Blender). A wireframe rose is on theme.
- Axis-true view presets (Blender, Godot).
- Thumbnails framed to bounds (Unity, Blender).
- Disclosure of which nodes hold children (Godot).

Wrong for this editor:

- Dockable, resizable, rearrangeable panels. Three fixed regions answering
  three questions is a better fit for a scenario editor than a layout the user
  has to build first.
- A generic property grid over reflected components. Already ruled out, and
  correctly - the Inspector's value is that it shows a ship's fields, not a
  ship's components.
- Grey chrome, tool ribbons, icon-only toolbars. The phosphor terminal is the
  identity. Icons belong in the tree and in gizmos, not as a wall of
  unlabelled buttons.
- Modal tool palettes and a Blender-style keymap. This editor already has two
  modal traps (the delete tool, an armed part); adding more would need a much
  louder mode readout than the bottom-edge legend it has now.
