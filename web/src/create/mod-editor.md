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
| Mod Hub | first screen | Lists your mods, most recent first. **New** makes one from a template (<kbd>Up</kbd> and <kbd>Down</kbd> pick it, <kbd>Enter</kbd> creates, <kbd>Esc</kbd> cancels), **Import** installs a zip, and a row's right-click menu holds Open, Duplicate and Delete. |
| Top bar | top | One bar: the File, Edit, View and Add menus, the view switch, the tools, Axes, Snap, Undo, Redo and Play. In the Ship editor the tools, Axes and Snap give way to the Ship menu and **Back to {name}**, which stands in their place so Play does not move. **Parts** stays on the bar there. Over a scenario, Back follows the tools. A menu shows only the verbs of the open document. A verb that fits but cannot run now is grey, and its hover says why. |
| View switch | top bar | The views of the open document only: **Scene** and **Events** for a scenario, **Ship** for a ship, **Section** for a section, and **Form** for every other kind. A view change is not an edit: it adds no undo step. |
| Hierarchy | left, in Scene | The tree of the document. |
| Scene | centre | The stage: ships, objects and their handles. A right click without a drag opens the stage menu: the Add menu (what it adds lands at the clicked point), Paste, and Frame and Delete on the node under the pointer. |
| Add menu | top bar, <kbd>Shift</kbd>+<kbd>A</kbd> on the stage | What the open document takes, in groups. A scenario has Ship (New Ship, then each ship design of the mod, placed by reference), Objects, Lights and Markers. A ship has Parts. A section has Mesh, Collider and Socket. Every document has Asset, which starts a new item of any kind. A group opens on hover or <kbd>Right</kbd>. Type to search every group at once. <kbd>Shift</kbd>+<kbd>A</kbd> opens it at the pointer, and what it adds lands there. |
| Inspector | right, in Scene | The fields of the selected node. In Form view the form of the open asset takes the main area, and the stage and its tools hide. A UI theme's palette is a grid of named swatches. A dim name is inherited until you pick it here. Press a swatch for the picker: a square, a hue strip and a hex field. A colour with alpha also gets an alpha strip. The pick shows live. One pick is one undo step, and <kbd>Esc</kbd> takes it back. A list field, such as a section's build **Parts**, a start role's **Stock** or a campaign's **Scenarios**, has a row per entry: pick the id, type the count where the list counts, and **x** removes the row. **+ Add** at the foot searches the ids the mod sees; a build recipe takes only Parts items. A repeat, an unknown id or a count outside 1 to 9999 is refused, and the status line says why. Each change is one undo step. |
| Package | File menu | A window first: the version, with **Patch**, **Minor** and **Major** buttons that show the next version (a bump is one undo step), the output folder, the file name, the check, and the files. A file the package leaves out wears the dotted mark. An error blocks Package: the window gives the count and the first errors, and Problems lists every one. After a package the window gives the path and the size, with **Copy Path** and **Open Folder**. A failure stays in the window with its reason. <kbd>Esc</kbd> closes it. Desktop only. |
| Mod Settings | File menu | A window for the manifest: name, description, author and version, the icon (none, or one of the mod's images) and the screenshots, picked from the mod's images. Each dependency is a row of name, id and version with an **x**. **+ Add dependency** searches the other mods the editor knows that are not listed yet. A refusal says why: a cycle, a mod the content still uses, or a mod that is not installed. Each change is one undo step. |
| Events | whole width | The script as blocks, beside a palette. The palette groups the blocks in build order: Events (the hats a script starts with), Conditions, Actions, Flow, and Values. Each entry is one short line in its block's shape. The rail at the palette's left edge has an **All** row and one dot per group. Click a group to show only its blocks. Click it again, or click **All**, to show every group. The rail marks the group it shows, and the group and the filter both apply. Click a group's heading to fold it. A filter opens every folded group it matches in. A hover says where a block goes. Drag a block and every place it fits lights up; only an Events block stands alone on the canvas. Fill a value on the block; its `...` chip opens a pop-up with every field and the block's kind. A right click holds Paste and Clean Up, which stands the scripts in one column as one undo step, and Duplicate and Delete on a block. The tools, Parts, the Hierarchy and the Inspector stay away here. |
| Assets | strip | Every item of the mod and every file in its folder as a tile, in one grid sorted by kind, with a count per kind. A double click or <kbd>Enter</kbd> opens a tile in its view: a scenario on Scene, a ship alone in the Ship editor, a section on Section, and any other kind on its Form. A ship a scenario made is listed under that scenario. Drag a ship design onto the stage, or right click it and choose **Place in Scenario**: the scenario spawns the design by its id, so an edit to the design reaches every placement. A right click holds the tile's verbs, and a hover shows the whole id. A right click on empty space holds New Asset and Import File. **+** opens New Asset: a blank item of a kind, with a new id. A section first asks for its kind in a window: a tile per kind says what it does; the arrows pick, <kbd>Enter</kbd> makes it, <kbd>Esc</kbd> cancels. Problems lists what the blank still needs, and Save, Play and Package wait until it is filled. |
| Assets files | strip | Images show themselves, meshes their rendered picture, and other files a glyph. A double click views an image or a mesh large and plays a sound. A file's menu holds Reveal Path, Rename, Include in Package, Delete and Import File (desktop only). The package holds only the files the manifest declares: a file it leaves out wears a dotted mark, and Include in Package adds it. Rename and Delete refuse a file that content names, and say which item. Delete takes a declared file out of the package as one undo step; a second Delete removes it from the folder, and no undo puts it back. |
| Problems | strip | The live lint of what Play would load. A row selects the node it names. Empty when there is nothing to report. Rows copy out like the Log's, and a copied row ends with its location, such as `(at ship/section)`. |
| Ship | strip, in the Ship editor | The skin, the styles and Generate. |
| Log | strip | What the status line said, and every warning and error the game logged. Clear it, filter it by level or by text; the newest line stays on top. Click a row to mark it, <kbd>Shift</kbd>+click to mark a range, and <kbd>Ctrl</kbd>+<kbd>A</kbd> marks every row the filters show. **Copy** or <kbd>Ctrl</kbd>+<kbd>C</kbd> copies the marked rows as plain text, one line each, or every shown row when none is marked. A right click on a row holds **Copy line**. The status line gives the count. The web build has no clipboard, and the status line says so. |
| Parts gallery | over the stage, in the Ship editor | **Parts** or <kbd>Tab</kbd> opens it. Take a part and click a free face. |

## The keys

| Key | Action |
| --- | --- |
| <kbd>Q</kbd> <kbd>W</kbd> <kbd>E</kbd> <kbd>R</kbd> | Select, Move, Turn, Scale. Scenario only. |
| <kbd>X</kbd> | Swap the handles between world and local axes. Scenario only. |
| right mouse held | Look about |
| middle drag, or <kbd>Alt</kbd> + left drag | Orbit the pivot |
| <kbd>Shift</kbd> + middle drag | Pan |
| wheel | Dolly, always |
| double click, or <kbd>Enter</kbd> on the stage | Open the selected ship alone in the Ship editor |
| <kbd>Q</kbd> | Ship editor: pick up the part under the pointer |
| <kbd>R</kbd>, <kbd>Shift</kbd>+<kbd>R</kbd> | Ship editor: roll the part in hand, or roll it back |
| <kbd>F</kbd>, <kbd>Shift</kbd>+<kbd>F</kbd> | Ship editor, part in hand: cycle its mating socket, or step back |
| <kbd>F</kbd> | Frame the selection |
| right click, no drag | The menu of the stage, the canvas or the tile under the pointer |
| <kbd>Shift</kbd>+<kbd>A</kbd> | Open the Add menu at the pointer, over the stage |
| <kbd>Up</kbd> <kbd>Down</kbd>, <kbd>Right</kbd> <kbd>Left</kbd>, <kbd>Enter</kbd>, <kbd>Esc</kbd> | In a menu: move, open or close a group, add, close. Type to search. |
| <kbd>Tab</kbd> | Open or close the parts gallery, in the Ship editor |
| <kbd>Ctrl</kbd>+<kbd>Z</kbd>, <kbd>Ctrl</kbd>+<kbd>Y</kbd> | Undo, redo |
| <kbd>Ctrl</kbd>+<kbd>C</kbd>, <kbd>Ctrl</kbd>+<kbd>V</kbd>, <kbd>Ctrl</kbd>+<kbd>D</kbd> | Copy, paste, duplicate |
| <kbd>Ctrl</kbd>+<kbd>A</kbd>, <kbd>Ctrl</kbd>+<kbd>C</kbd> over the Log or Problems | Mark every shown row; copy the marked rows as text |
| <kbd>Del</kbd> | Delete the selection |
| <kbd>Ctrl</kbd>+<kbd>S</kbd> | Save the mod |
| <kbd>Enter</kbd> | Open the marked asset tile |
| <kbd>Esc</kbd> | Back out one step. It puts down a block or a part you hold, it takes back what a block's pop-up did, and it takes back the colour picker's pick. Last, it goes Back to the document you came from. |
| <kbd>F1</kbd> | Return from Play to the editor |

## Mods written by hand

**Import** on the Mod Hub brings a hand-written mod into the editor. One edit
writes that edit. Each file that you did not change keeps its bytes. A file that
you changed is written again from its items, so its comments and its layout go.

A hull stays as the file wrote it when a ship node cannot hold it all: a stock,
a section patch, a spawn id of its own, or a design that the editor did not
write. It stands on the stage and it moves. Its Inspector says
`preserved as authored`. Edit that hull in the file.

Play does not carry a scenario's generated `world`. A handler that names the
`bounds` area is saved, and Play leaves it out.
