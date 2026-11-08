# The Mod Editor

**Sandbox**, on the main menu, opens the Mod Hub. A mod holds all of its work:
scenarios, sections and every other kind of content. This page is the map of the
panels and the keys you use most. The
[full key list](../../wiki/keybinds/#editor) is in the wiki.

## The panels

Nothing docks. Drag the strip's top edge to make it taller or shorter. **View**
hides the Hierarchy, the Inspector and the strip's body.

| Panel | Where | What it does |
| --- | --- | --- |
| Mod Hub | first screen | Lists your mods, most recent first. **New** makes one from a template, **Import** installs a zip, and a row's right-click menu holds Open, Duplicate and Delete. |
| Top bar | top | One bar: the File, Edit, View and Add menus, the view switch, the tools, Axes, Snap, Undo, Redo and Play. Inside a ship it adds the Ship menu, Parts and Leave Ship. A menu shows only the verbs of the open document. A verb that fits but cannot run now is grey, and its hover says why. |
| View switch | top bar | The views of the open document only: **Scene** and **Events** for a scenario, **Ship** inside a ship, **Section** for a section, and **Form** for every other kind. A view change is not an edit: it adds no undo step. |
| Hierarchy | left, in Scene | The tree of the document. |
| Scene | centre | The stage: ships, objects and their handles. A right click without a drag opens the stage menu: the Add verbs of the view (an object lands at the clicked point), Paste, and Frame and Delete on the node under the pointer. |
| Inspector | right, in Scene | The fields of the selected node. In Form view the form of the open asset takes the main area, and the stage and its tools hide. A UI theme's palette is a grid of named swatches. A dim name is inherited until you pick it here. Press a swatch for the picker: a square, a hue strip and a hex field. A colour with alpha also gets an alpha strip. The pick shows live. One pick is one undo step, and <kbd>Esc</kbd> takes it back. |
| Events | whole width | The script as blocks, beside a palette. The palette groups the blocks in build order: Events (the hats a script starts with), Conditions, Actions, Flow, and Values. Each entry is one short line in its block's shape. The rail at the palette's left edge has one dot per group: click it to scroll to that group. The rail marks the group in view. Click a group's heading to fold it. A filter shows the matches from every group. A hover says where a block goes. Drag a block and every place it fits lights up; only an Events block stands alone on the canvas. Fill a value on the block; its `...` chip opens a pop-up with every field and the block's kind. A right click holds Paste and Clean Up, which stands the scripts in one column as one undo step, and Duplicate and Delete on a block. The tools, Parts, the Hierarchy and the Inspector stay away here. |
| Assets | strip | Every item of the mod and every file in its folder as a tile, in one grid sorted by kind, with a count per kind. A double click or <kbd>Enter</kbd> opens a tile in its view: a scenario on Scene, a ship inside its scenario, a section on Section, and any other kind on its Form. A right click holds the tile's verbs, and a hover shows the whole id. **+** adds a blank item with a new id. A section first asks for its kind. Problems lists what the blank still needs, and Save, Play and Package wait until it is filled. |
| Assets files | strip | Images show themselves, meshes their rendered picture, and other files a glyph. A double click views an image or a mesh large and plays a sound. A file's menu holds Reveal Path, Rename, Include in Package, Delete and Import File (desktop only). The package holds only the files the manifest declares: a file it leaves out wears a dotted mark, and Include in Package adds it. Rename and Delete refuse a file that content names, and say which item. Delete takes a declared file out of the package as one undo step; a second Delete removes it from the folder, and no undo puts it back. |
| Problems | strip | The live lint of what Play would load. A row selects the node it names. Empty when there is nothing to report. |
| Ship | strip, inside a ship | The skin, the styles and Generate. |
| Log | strip | What the status line said, and every warning and error the game logged. Clear it, filter it by level or by text; the newest line stays on top. |
| Parts gallery | over the stage, inside a ship, in Scene | **Parts** or <kbd>Tab</kbd> opens it. Take a part and click a free face. |

## The keys

| Key | Action |
| --- | --- |
| <kbd>Q</kbd> <kbd>W</kbd> <kbd>E</kbd> <kbd>R</kbd> | Select, Move, Turn, Scale. Scene only. |
| <kbd>X</kbd> | Swap the handles between world and local axes. Scene only. |
| right mouse held | Look about |
| middle drag, or <kbd>Alt</kbd> + left drag | Orbit the pivot |
| <kbd>Shift</kbd> + middle drag | Pan |
| <kbd>F</kbd> | Frame the selection |
| right click, no drag | The menu of the stage, the canvas or the tile under the pointer |
| <kbd>Tab</kbd> | Open or close the parts gallery, inside a ship, in Scene |
| <kbd>Ctrl</kbd>+<kbd>Z</kbd>, <kbd>Ctrl</kbd>+<kbd>Y</kbd> | Undo, redo |
| <kbd>Ctrl</kbd>+<kbd>C</kbd>, <kbd>Ctrl</kbd>+<kbd>V</kbd>, <kbd>Ctrl</kbd>+<kbd>D</kbd> | Copy, paste, duplicate |
| <kbd>Del</kbd> | Delete the selection |
| <kbd>Ctrl</kbd>+<kbd>S</kbd> | Save the mod |
| <kbd>Enter</kbd> | Open the marked asset tile |
| <kbd>Esc</kbd> | Back out one step. It puts down a block or a part you hold, it takes back what a block's pop-up did, and it takes back the colour picker's pick. |
| <kbd>F1</kbd> | Return from Play to the editor |

## Mods written by hand

**Import** on the Mod Hub brings a hand-written mod into the editor. One edit
writes that edit. Each file that you did not change keeps its bytes. A file that
you changed is written again from its items, so its comments and its layout go.

A hull stays as the file wrote it when a ship node cannot hold it all: a stock,
a section patch, a spawn id of its own, or a design that the editor did not
write. It stands on the stage and it moves. Its Inspector says
`preserved as authored`, and you cannot go inside it. Edit that hull in the file.

Play does not carry a scenario's generated `world`. A handler that names the
`bounds` area is saved, and Play leaves it out.
