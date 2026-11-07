# The Mod Editor

**Sandbox**, on the main menu, opens the Mod Hub. A mod holds all of its work:
scenarios, sections and every other kind of content. This page is the map of the
panels and the keys you use most. The
[full key list](../../wiki/keybinds/#editor) is in the wiki.

## The panels

The panels are fixed: nothing docks or drags. **View** hides the Hierarchy, the
Inspector and the strip's body.

| Panel | Where | What it does |
| --- | --- | --- |
| Mod Hub | first screen | Lists your mods, most recent first. **New** makes one from a template, **Import** installs a zip, and a row's right-click menu holds Open, Duplicate and Delete. |
| Top bar | top | One bar: the File, Edit, View, Add and Ship menus, the **Scene** and **Events** tabs, the tools, Axes, Snap, Undo, Redo, Parts, Play and the camera speed. |
| Hierarchy | left, in Scene | The tree of the document. |
| Scene | centre | The stage: ships, objects and their handles. |
| Inspector | right, in Scene | The fields of the selected node, or the form of an open asset. |
| Events | whole width | The script as blocks, beside a palette. Drag a block and every place it fits lights up. Fill a value on the block; its `...` chip opens a pop-up with every field and the block's kind. The tools, Parts, the camera speed, the Hierarchy and the Inspector stay away here. |
| Assets | strip | Every item of the mod as a file tile, with a count per kind. A double click opens a tile, a right click holds its verbs, **+** adds an item, and a hover shows the whole id. A section opens on the bench, where you edit its sockets and mesh. |
| Problems | strip | The live lint of what Play would load. A row selects the node it names. |
| Ship | strip, inside a ship | The skin, the styles and Generate. |
| Log | strip | What the status line said. |
| Parts gallery | over the stage, inside a ship, in Scene | **Parts** or <kbd>Tab</kbd> opens it. Take a part and click a free face. |

## The keys

| Key | Action |
| --- | --- |
| <kbd>Q</kbd> <kbd>W</kbd> <kbd>E</kbd> <kbd>R</kbd> | Select, Move, Turn, Scale. Scene only. |
| <kbd>X</kbd> | Swap the handles between world and local axes. Scene only. |
| right mouse + <kbd>W</kbd> <kbd>A</kbd> <kbd>S</kbd> <kbd>D</kbd>, <kbd>Q</kbd> <kbd>E</kbd> | Fly the camera at one constant speed |
| middle drag, or <kbd>Alt</kbd> + left drag | Orbit the pivot |
| <kbd>Shift</kbd> + middle drag | Pan |
| <kbd>F</kbd> | Frame the selection |
| <kbd>Tab</kbd> | Open or close the parts gallery, inside a ship, in Scene |
| <kbd>Ctrl</kbd>+<kbd>Z</kbd>, <kbd>Ctrl</kbd>+<kbd>Y</kbd> | Undo, redo |
| <kbd>Ctrl</kbd>+<kbd>C</kbd>, <kbd>Ctrl</kbd>+<kbd>V</kbd>, <kbd>Ctrl</kbd>+<kbd>D</kbd> | Copy, paste, duplicate |
| <kbd>Del</kbd> | Delete the selection |
| <kbd>Ctrl</kbd>+<kbd>S</kbd> | Save the mod |
| <kbd>Esc</kbd> | Back out one step. It puts down a block or a part you hold, and it takes back what a block's pop-up did. |
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
