# Keybinds

The full control reference, keyboard and gamepad side by side. Thrust is analog on the gamepad and on/off on the keyboard. The autopilot verbs each fly a whole maneuver and hand control back; CANCEL, a main-drive burn, a bound thruster key, entering RCS or the engaged verb pressed again disengages them immediately, while the mouse only swings the camera. New players should start with [Your first flight](../getting-started/), which teaches these one at a time. Every one of these is REBINDABLE in-game under **Settings > Controls**, reachable from both the main menu and the pause menu - one binding group at a time, keyboard and gamepad in their own columns. Pressing a key another action in the same live set already holds is refused by name, and `Reset Defaults` puts the whole table back. The MOUSE group in that same list holds no bindings - it is the three mouse sensitivity sliders, covered in [Settings](../settings/#mouse).

Q and E are now the default roll keys. If an older saved control setting assigned either key to another action, that conflicting override returns to its default on load. Check **Settings > Controls** to rebind the displaced action to a free key.

A dash means the action has no binding on that device.

## Flight

<table class="controls controls--split">
    <thead>
        <tr>
            <th>Action</th>
            <th>Keyboard &amp; mouse</th>
            <th>Gamepad</th>
        </tr>
    </thead>
    <tbody>
        <tr>
            <td>Main thruster burn</td>
            <td><kbd>W</kbd> / <kbd>Space</kbd></td>
            <td>
                <span
                    class="pf pf-shoulder-right"
                    role="img"
                    aria-label="Right bumper"
                ></span>
            </td>
        </tr>
        <tr>
            <td>Roll left / right - turn the hull about its nose</td>
            <td><kbd>Q</kbd> / <kbd>E</kbd> (hold)</td>
            <td>unbound</td>
        </tr>
        <tr>
            <td>GOTO - fly to the current lock</td>
            <td><kbd>G</kbd></td>
            <td>
                <span
                    class="pf pf-face-north"
                    role="img"
                    aria-label="North face button"
                ></span>
            </td>
        </tr>
        <tr>
            <td>ORBIT - park into orbit around a gravity well</td>
            <td><kbd>O</kbd></td>
            <td>
                <span
                    class="pf pf-face-south"
                    role="img"
                    aria-label="South face button"
                ></span>
            </td>
        </tr>
        <tr>
            <td>STOP - face retrograde and burn the speed off</td>
            <td><kbd>X</kbd></td>
            <td>
                <span
                    class="pf pf-face-east"
                    role="img"
                    aria-label="East face button"
                ></span>
            </td>
        </tr>
        <tr>
            <td>MATCH - match the travel lock's velocity on RCS only</td>
            <td><kbd>Shift</kbd> (hold) + <kbd>X</kbd></td>
            <td>
                <span
                    class="pf pf-stick-left-click"
                    role="img"
                    aria-label="Left stick click"
                ></span>
                (hold) +
                <span
                    class="pf pf-face-east"
                    role="img"
                    aria-label="East face button"
                ></span>
            </td>
        </tr>
        <tr>
            <td>CANCEL autopilot (resume manual)</td>
            <td><kbd>Z</kbd></td>
            <td>
                <span
                    class="pf pf-face-west"
                    role="img"
                    aria-label="West face button"
                ></span>
            </td>
        </tr>
        <tr>
            <td>RCS fine translation (unless a scenario withholds it)</td>
            <td>
                <kbd>Shift</kbd> (hold) +
                <span
                    class="pf pf-mouse-device"
                    role="img"
                    aria-label="Mouse motion"
                ></span>
                /
                <span
                    class="pf pf-mouse-scroll"
                    role="img"
                    aria-label="Scroll wheel"
                ></span>
            </td>
            <td>
                <span
                    class="pf pf-stick-left-click"
                    role="img"
                    aria-label="Left stick click"
                ></span>
                (hold) +
                <span
                    class="pf pf-stick-left"
                    role="img"
                    aria-label="Left stick"
                ></span>
            </td>
        </tr>
        <tr>
            <td>DOCK - clamp to the ship you have locked, press again to let go</td>
            <td><kbd>D</kbd></td>
            <td>
                <span
                    class="pf pf-stick-left-click"
                    role="img"
                    aria-label="Left stick click"
                ></span>
                (hold) +
                <span
                    class="pf pf-face-north"
                    role="img"
                    aria-label="North face button"
                ></span>
            </td>
        </tr>
        <tr>
            <td>HELM - take a docked pair's helm, press again to hand it back</td>
            <td><kbd>H</kbd></td>
            <td>
                <span
                    class="pf pf-stick-left-click"
                    role="img"
                    aria-label="Left stick click"
                ></span>
                (hold) +
                <span
                    class="pf pf-face-south"
                    role="img"
                    aria-label="South face button"
                ></span>
            </td>
        </tr>
    </tbody>
</table>

DOCK needs a docking port on both hulls, so it is offered only on a ship built with one: build a hull with a docking port in the editor, or fly the `docking_approach` example. A dock starts neutral: the other ship flies the pair until <kbd>H</kbd> takes the helm, and <kbd>D</kbd> again lets go. A dock with a derelict or neutralized hull, ship or station, gives you the helm at once; a living station keeps it and refuses <kbd>H</kbd>. See [Docking port](../sections/docking/).

Dock and Helm appear in **Settings > Controls > FLIGHT** with <kbd>D</kbd> and <kbd>H</kbd> as rebindable defaults.

On a gamepad DOCK and HELM are chords on the RCS button, like MATCH. GOTO and ORBIT do nothing useful while RCS is held, so while you hold the left stick click the GOTO button docks and lets go, and the ORBIT button takes and hands back the helm. The keybind dock draws both buttons on the DOCK and HELM chips. The chord follows the RCS, GOTO and ORBIT bindings, so <kbd>Shift</kbd> + <kbd>G</kbd> and <kbd>Shift</kbd> + <kbd>O</kbd> do the same on a keyboard. The gamepad column of the Dock and Helm rows in Settings stays empty until you bind a button of your own there.

Roll turns the view and the hull together at a quarter turn a second, through the same controller that points the nose, and draws no RCS. <kbd>Q</kbd> rolls left and <kbd>E</kbd> rolls right; holding both holds still. Roll does nothing in free look, with weapons raised, while RCS is held or while an autopilot verb is flying. Roll Left and Roll Right appear in **Settings > Controls > FLIGHT**; no gamepad button is free for them, and a ship's own section keys cannot take <kbd>Q</kbd> or <kbd>E</kbd>.

Mining has no flight key. Each mining beam section has its own key, like a weapon or a thruster: a ship's first beam answers <kbd>V</kbd> and the right trigger on a gamepad, on the mining tug's bow and on a beam placed in the editor alike. A second beam takes <kbd>B</kbd> and no gamepad button. The right trigger is also FIRE: with the weapons cold it only mines, and with them hot it does both. Hold a beam's key with an ore rock travel-locked, in front of that emitter and within 100 m; it opens, runs its tip out and cuts the rock once a second where it hits. Release the key and that beam retracts. Rebind each beam on the [Ship pane](../interface/#rebinding-a-section). See [Mining beam](../sections/mining-beam/).

RCS is a docking aid on top of standard flight. Every ship has it unless a scenario withholds it: the RCS chip in the keybind dock stays dark while it is withheld, and Basic Training hands it over at the beat that teaches it. See [Flight & autopilot](../flight-autopilot/#rcs-fine-docking-thrusters).

## Targeting and camera

<table class="controls controls--split">
    <thead>
        <tr>
            <th>Action</th>
            <th>Keyboard &amp; mouse</th>
            <th>Gamepad</th>
        </tr>
    </thead>
    <tbody>
        <tr>
            <td>Aim / look</td>
            <td>
                <span
                    class="pf pf-mouse-device"
                    role="img"
                    aria-label="Mouse motion"
                ></span>
            </td>
            <td>
                <span
                    class="pf pf-stick-right"
                    role="img"
                    aria-label="Right stick"
                ></span>
            </td>
        </tr>
        <tr>
            <td>Free look</td>
            <td><kbd>Alt</kbd> (hold)</td>
            <td>
                <span
                    class="pf pf-shoulder-left"
                    role="img"
                    aria-label="Left bumper"
                ></span>
                (hold)
            </td>
        </tr>
        <tr>
            <td>Raise weapons (combat stance)</td>
            <td>
                <span
                    class="pf pf-mouse-2"
                    role="img"
                    aria-label="Right mouse button"
                ></span>
                (hold)
            </td>
            <td>
                <span
                    class="pf pf-trigger-left"
                    role="img"
                    aria-label="Left trigger"
                ></span>
                (hold)
            </td>
        </tr>
        <tr>
            <td>Radar lock (hold to sweep)</td>
            <td><kbd>Ctrl</kbd> (hold)</td>
            <td>
                <span
                    class="pf pf-dpad-up"
                    role="img"
                    aria-label="D-Pad up"
                ></span>
                (hold)
            </td>
        </tr>
        <tr>
            <td>Clear lock (staged: combat, then nav)</td>
            <td><kbd>Ctrl</kbd> (tap)</td>
            <td>
                <span
                    class="pf pf-dpad-up"
                    role="img"
                    aria-label="D-Pad up"
                ></span>
                (tap)
            </td>
        </tr>
        <tr>
            <td>Cycle fine-lock component</td>
            <td>
                <kbd>[</kbd> <kbd>]</kbd> / weapons raised +
                <span
                    class="pf pf-mouse-scroll"
                    role="img"
                    aria-label="Scroll wheel"
                ></span>
            </td>
            <td>
                <span
                    class="pf pf-dpad-left"
                    role="img"
                    aria-label="D-Pad left"
                ></span>
                <span
                    class="pf pf-dpad-right"
                    role="img"
                    aria-label="D-Pad right"
                ></span>
            </td>
        </tr>
        <tr>
            <td>Zoom the chase camera (up = closer)</td>
            <td>
                <span
                    class="pf pf-mouse-scroll"
                    role="img"
                    aria-label="Scroll wheel"
                ></span>
            </td>
            <td>-</td>
        </tr>
    </tbody>
</table>

## Weapons

<table class="controls controls--split">
    <thead>
        <tr>
            <th>Action</th>
            <th>Keyboard &amp; mouse</th>
            <th>Gamepad</th>
        </tr>
    </thead>
    <tbody>
        <tr>
            <td>Fire turrets</td>
            <td>
                <span
                    class="pf pf-mouse-1"
                    role="img"
                    aria-label="Left mouse button"
                ></span>
                (default)
            </td>
            <td>
                <span
                    class="pf pf-trigger-right"
                    role="img"
                    aria-label="Right trigger"
                ></span>
            </td>
        </tr>
        <tr>
            <td>Launch torpedo</td>
            <td><kbd>F</kbd> (default)</td>
            <td>
                <span
                    class="pf pf-shoulder-left"
                    role="img"
                    aria-label="Left bumper"
                ></span>
            </td>
        </tr>
        <tr>
            <td>Fire railgun</td>
            <td><kbd>R</kbd> (default)</td>
            <td>
                <span
                    class="pf pf-select"
                    role="img"
                    aria-label="Select button"
                ></span>
            </td>
        </tr>
        <tr>
            <td>Run a mining beam out (hold)</td>
            <td><kbd>V</kbd> (default)</td>
            <td>
                <span
                    class="pf pf-trigger-right"
                    role="img"
                    aria-label="Right trigger"
                ></span>
            </td>
        </tr>
    </tbody>
</table>

One weapon, one button. A gun takes its own key the moment it is placed or generated, so a hull you never drew can be flown as it comes: the thrusters answer <kbd>Space</kbd>, the PDC turrets the left mouse button, the tubes <kbd>F</kbd>, a [railgun](../sections/railgun/) <kbd>R</kbd>, a [mining beam](../sections/mining-beam/) <kbd>V</kbd>. The railgun key is a tap that commits the charge rather than a burst you hold, and a torpedo only launches while you hold a raised (red) combat lock.

On a gamepad the torpedo button is the left bumper and the railgun button is Select, the buttons free look and the HUD toggle use. A torpedo and a railgun only answer while the weapons are hot, so while they are hot the left bumper does not free look and Select does not toggle the HUD; <kbd>Alt</kbd> and <kbd>`</kbd> still do. A ship in a save from an earlier version keeps the bindings it had: rebind it on the [Ship pane](../interface/#rebinding-a-section).

None of this is fixed: in the editor, click a section to bind it to any key or mouse button, so a ship's controls are whatever its build assigns.

## Interface

<table class="controls controls--split">
    <thead>
        <tr>
            <th>Action</th>
            <th>Keyboard &amp; mouse</th>
            <th>Gamepad</th>
        </tr>
    </thead>
    <tbody>
        <tr>
            <td>Cycle HUD (On / Cinematic)</td>
            <td><kbd>`</kbd> (grave / tilde)</td>
            <td>
                <span
                    class="pf pf-select"
                    role="img"
                    aria-label="Select button"
                ></span>
            </td>
        </tr>
        <tr>
            <td>
                <a href="../interface/">Interface</a> (Map, Ship, Inventory and Jobs panes;
                pauses, frees the cursor; <kbd>Esc</kbd> also closes it)
            </td>
            <td><kbd>Tab</kbd></td>
            <td>
                <span
                    class="pf pf-stick-right-click"
                    role="img"
                    aria-label="Right stick click"
                ></span>
            </td>
        </tr>
        <tr>
            <td>Switch interface pane (Map / Ship / Inventory / Jobs / Station)</td>
            <td><kbd>M</kbd></td>
            <td>
                <span
                    class="pf pf-face-north"
                    role="img"
                    aria-label="North face button"
                ></span>
            </td>
        </tr>
        <tr>
            <td>
                <a href="../commands/">Command shell</a> (inspect, settings,
                the flight log, armed cheats; works with no ship, and over the
                menus and the interface)
            </td>
            <td><kbd>;</kbd></td>
            <td>-</td>
        </tr>
        <tr>
            <td>Pause menu</td>
            <td><kbd>Esc</kbd></td>
            <td>
                <span
                    class="pf pf-start"
                    role="img"
                    aria-label="Start button"
                ></span>
            </td>
        </tr>
        <tr>
            <td>Back to editor (editor only)</td>
            <td><kbd>F1</kbd></td>
            <td>
                <span
                    class="pf pf-stick-left-click"
                    role="img"
                    aria-label="Left stick click"
                ></span>
            </td>
        </tr>
    </tbody>
</table>

## Editor

The editor is mouse-first and has no gamepad bindings. Every key below is also
a row on the top bar - **File**, **Edit**, **View**, **Add**, **Ship** - with
the key printed beside it, so this table is a shortcut list and not a second
interface. The legend on the bottom line shows the gestures that apply to
whatever you are holding right now, and what <kbd>Esc</kbd> does next. Under a
block's pop-up it shows only what works there. **View > Key Legend** hides it.

### The panels

The panels are fixed: nothing docks or drags. **View** hides the Hierarchy, the
Inspector and the strip's body, and so do <kbd>Ctrl</kbd>+<kbd>1</kbd>,
<kbd>Ctrl</kbd>+<kbd>2</kbd> and <kbd>Ctrl</kbd>+<kbd>3</kbd>.

- **Top bar**: the menus, the view switch, the four tools, Axes, Snap, Undo,
  Redo and Play; in the Ship editor Parts and **Back to {name}** instead of the
  tools, Axes and Snap. Each button does what its key does. Snap inverts Ctrl:
  with it on, a handle snaps until you hold Ctrl. A menu shows only the verbs of
  the open document, and the Ship menu shows only in the Ship editor. A verb that fits but cannot run now is grey, and its
  hover says why.
- **View switch**: the views of the open document only. A scenario has Scene
  and Events, a ship has Ship, a section has Section, and every other kind has
  Form. A view change adds no undo step.
- **Hierarchy**, on the left in Scene: the tree of the document. A caret opens a
  row, the filter box finds a row under any shut one, <kbd>Ctrl</kbd>+click and
  <kbd>Shift</kbd>+click mark many, and right-click opens the row's menu: Rename,
  Duplicate, Delete, Frame and what can be added under it.
- **Inspector**, on the right in Scene: the selected node's fields. Its header
  carries the kind, the name, the id and a validity word, and a group heading
  folds. In Form view the open asset's form takes the main area instead, and
  the stage and its tools hide. A UI theme's palette is a grid of named
  swatches; press one for the colour picker.
- **Events** fills the window above the strip with a block palette and a canvas.
  The palette groups the blocks in build order: Events (the hats a script
  starts with), Conditions, Actions, Flow, and Values and logic. Only an Events
  block stands alone on the canvas. A right click on the canvas holds Paste,
  Clean Up (the scripts in one column, one undo step), and Duplicate and Delete
  on a block. It has no Hierarchy and no Inspector: fill a value on its block,
  and a block's `...` chip opens a pop-up with every field. The four tools, Axes and Snap hide,
  and <kbd>Q</kbd> <kbd>W</kbd> <kbd>E</kbd> <kbd>R</kbd> <kbd>X</kbd> do nothing.
- **Strip**, along the bottom: **Assets** shows the mod's items and the files in
  its folder as tiles, sorted by kind, with a count per kind. A double click or
  <kbd>Enter</kbd> opens a tile in its view, views an image or a mesh, or plays a
  sound. A right click on a tile holds its verbs, and on empty space New Asset
  and Import File. **+** adds a blank item, and a hover shows the whole id. A
  new section asks its kind first: the arrows pick one, <kbd>Enter</kbd> makes
  it, <kbd>Esc</kbd> cancels. Drag a ship design onto the stage, or right click it and
choose **Place in Scenario**, to place it in the open scenario. A ship a
scenario made is listed under that scenario. A file the package leaves out wears a dotted mark. **Problems** is the
  live lint of what Play would load; a row selects the node it names. **Ship**
  appears in the Ship editor and holds the skin, the styles and Generate. **Log** keeps
  what the status line said and what the game warned of, with Clear, a level
  filter and a text filter. In the Log and Problems a click marks a row,
  <kbd>Shift</kbd>+click a range, and **Copy** copies the marked rows as text, or
  every shown row when none is marked. Drag the strip's top edge to resize it.

The stage camera has no fly keys: in a scenario the letters are the tools. Hold
right mouse to look about, and orbit, pan and dolly to move. The wheel always
dollies. A right click without a drag opens the stage menu: the Add menu,
Paste, and Frame and Delete on a node.

**Add** holds what the open document takes, in groups: a scenario has Ship (New
Ship and each ship design of the mod), Objects, Lights and Markers; a ship has
Parts; a section has Mesh, Collider and Socket. Every document has Asset, which
starts a new item of any kind. <kbd>Shift</kbd>+<kbd>A</kbd> over the stage
opens it at the pointer, and what it adds lands there. A group opens on hover or
on <kbd>Right</kbd>. <kbd>Up</kbd> and <kbd>Down</kbd> move, <kbd>Left</kbd>
closes a group, <kbd>Enter</kbd> adds, and <kbd>Esc</kbd> closes the menu. Type
to search every group at once.
<kbd>F</kbd> frames whatever is selected, and at the scenario level it
frames the whole document.

Every ship opens in the Ship editor, alone and square on the origin: double
click it in the Hierarchy or on its tile, or select it and press
<kbd>Enter</kbd>. Its position and rotation belong to the scenario, and only show
there. A placed design opens the design. <kbd>Esc</kbd> or **Back to {name}** on
the bar returns to the document you came from, with its camera, selection and
view. The Ship editor has no stage tools: parts mate where they go, so
<kbd>Q</kbd>, <kbd>R</kbd> and <kbd>F</kbd> are part keys there, as in the
game's Ship pane.

Keys are verbs only while nothing else has the keyboard. With the parts gallery
up, the arrow keys move its grid and <kbd>Esc</kbd> closes the gallery rather
than backing out of the ship; with the caret in a text field, every key is a
character; and while a rebind waits for a key, it takes all of them except
<kbd>Esc</kbd>. A block's pop-up in the editor stops the verbs too, and
<kbd>Ctrl</kbd>+<kbd>Z</kbd> closes it before it undoes.
<kbd>Ctrl</kbd>+<kbd>S</kbd> answers under all of that except a field and a
pending rebind.

<table class="controls">
    <tr>
        <td>Look about</td>
        <td>hold right mouse and move the mouse</td>
    </tr>
    <tr>
        <td>Orbit the pivot; pan; dolly</td>
        <td>
            middle drag or <kbd>Alt</kbd>+left drag;
            <kbd>Shift</kbd>+middle drag; wheel or <kbd>Alt</kbd>+right drag
        </td>
    </tr>
    <tr>
        <td>Pick a tool: Select, Move, Turn, Scale (scenario only)</td>
        <td><kbd>Q</kbd> <kbd>W</kbd> <kbd>E</kbd> <kbd>R</kbd></td>
    </tr>
    <tr>
        <td>Swap the handles between world and local axes (scenario only)</td>
        <td><kbd>X</kbd></td>
    </tr>
    <tr>
        <td>Undo; redo</td>
        <td>
            <kbd>Ctrl</kbd>+<kbd>Z</kbd>; <kbd>Ctrl</kbd>+<kbd>Y</kbd> or
            <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>Z</kbd>
        </td>
    </tr>
    <tr>
        <td>Copy; paste; duplicate</td>
        <td><kbd>Ctrl</kbd>+<kbd>C</kbd>; <kbd>Ctrl</kbd>+<kbd>V</kbd>; <kbd>Ctrl</kbd>+<kbd>D</kbd></td>
    </tr>
    <tr>
        <td>Over the Log or Problems: mark every shown row; copy the marked rows as text</td>
        <td><kbd>Ctrl</kbd>+<kbd>A</kbd>; <kbd>Ctrl</kbd>+<kbd>C</kbd></td>
    </tr>
    <tr>
        <td>Select a node; open the ship in the Ship editor</td>
        <td>left click; double click or <kbd>Enter</kbd></td>
    </tr>
    <tr>
        <td>Slide a ship or object on the ground plane (scenario node only)</td>
        <td>drag its body</td>
    </tr>
    <tr>
        <td>Move it along one axis; turn it about one</td>
        <td>drag an arrow; drag a ring</td>
    </tr>
    <tr>
        <td>Put the camera on the selection</td>
        <td><kbd>F</kbd> &nbsp;(or <b>View &gt; Frame Selection</b>)</td>
    </tr>
    <tr>
        <td>Delete the selection</td>
        <td><kbd>Del</kbd> &nbsp;(or <b>Edit &gt; Delete</b>)</td>
    </tr>
    <tr>
        <td>Save the mod</td>
        <td><kbd>Ctrl</kbd>+<kbd>S</kbd> &nbsp;(or <b>File &gt; Save</b>)</td>
    </tr>
    <tr>
        <td>Open or close the parts gallery (Ship editor)</td>
        <td><kbd>Tab</kbd> &nbsp;(or <b>Ship &gt; Parts...</b>)</td>
    </tr>
    <tr>
        <td>Gallery: take the part under the cursor and go build it</td>
        <td><kbd>Q</kbd></td>
    </tr>
    <tr>
        <td>Gallery: select / page / open the focused part</td>
        <td>arrow keys, <kbd>PgUp</kbd> <kbd>PgDn</kbd>, <kbd>Enter</kbd></td>
    </tr>
    <tr>
        <td>Gallery: search (typing needs the caret in the field)</td>
        <td><kbd>/</kbd>, then type; <kbd>Enter</kbd> opens the top hit</td>
    </tr>
    <tr>
        <td>Gallery focus: turn and zoom the part</td>
        <td>drag, wheel</td>
    </tr>
    <tr>
        <td>Place the part you are holding</td>
        <td>left click on the ship</td>
    </tr>
    <tr>
        <td>Roll it about the mating axis; roll it back</td>
        <td><kbd>R</kbd>; <kbd>Shift</kbd>+<kbd>R</kbd></td>
    </tr>
    <tr>
        <td>Cycle which of its sockets mates; step back</td>
        <td><kbd>F</kbd>; <kbd>Shift</kbd>+<kbd>F</kbd></td>
    </tr>
    <tr>
        <td>Pick up the part under the cursor (Ship editor)</td>
        <td><kbd>Q</kbd></td>
    </tr>
    <tr>
        <td>Bind a flight key to the selected part</td>
        <td>
            <b>Ship &gt; Rebind Key...</b> or the inspector's Key row, then
            press the key
        </td>
    </tr>
    <tr>
        <td>Go back to the document you came from</td>
        <td><kbd>Bksp</kbd> &nbsp;(or <b>Back to {name}</b> on the bar)</td>
    </tr>
    <tr>
        <td>Open the marked asset tile in its view; view or play a file</td>
        <td>double click &nbsp;or&nbsp; <kbd>Enter</kbd></td>
    </tr>
    <tr>
        <td>The menu of the stage, the canvas or a tile</td>
        <td>right click without a drag</td>
    </tr>
    <tr>
        <td>Open the Add menu at the pointer, over the stage</td>
        <td><kbd>Shift</kbd>+<kbd>A</kbd></td>
    </tr>
    <tr>
        <td>In a menu: move; open or close a group; add; close</td>
        <td><kbd>Up</kbd> <kbd>Down</kbd>; <kbd>Right</kbd> <kbd>Left</kbd>; <kbd>Enter</kbd>; <kbd>Esc</kbd> &nbsp;(type to search)</td>
    </tr>
    <tr>
        <td>Back out one rung</td>
        <td><kbd>Esc</kbd></td>
    </tr>
</table>

<kbd>Esc</kbd> takes one rung per press, in this order: a block in hand, a row's
menu, a block's pop-up, an open top-bar menu, an open colour picker (its pick is
taken back), the part in hand, Back to the document you came from, and finally
the pause menu. A text field with the caret, the gallery and
a pending rebind hold the keyboard, so <kbd>Esc</kbd> goes to them first.
<kbd>Bksp</kbd> is the shortcut past all of that - it goes Back from wherever you are, unless a text field has the
caret, where it deletes a character instead.

### The build readout

The Hierarchy prints what the hull on the stage would fly like under its tree,
repainted as you place. Five lines, values aligned:

| line | what it is |
| --- | --- |
| `Turn` | the attitude ceiling in `rad/s2`, or `no computer` on a hull that has none |
| `Mass` | the hull's mass, which is the volume of the parts - a section is solid ship at one density, so its box IS its mass |
| `Thrust` | every thruster's magnitude, summed |
| `HP` | every section's health, summed |
| `Parts` | how many sections are on the hull |

A hull with nothing on it reads `no parts yet` rather than `-` down the column,
because nothing has failed - nothing has been built. The numbers are measured
off the parts as they are POSED, through the same arithmetic the flown ship
spawns with, so the readout and the hull cannot disagree; a design that mounts its
engine at half health is counted at half health here too.

Under the block sits one line naming the limit that holds `Turn` down, and the
one thing that raises it: **`torque-limited - fit another computer`** or
**`structure-limited - shorten the hull`**. The two ceilings want opposite
answers - more computers do nothing against the metal, and shorter metal does
nothing against the computers - so the remedy is printed rather than left to be
guessed. See [What sets how hard a ship turns](../sections/controller/).

### The inspector

The right-hand panel, and a block's pop-up in Events, show the selected node's
own fields, read off the thing itself rather than from a list the editor keeps - so a part that grows a setting
grows a row. It opens on the fields that matter for that kind; **View > All
Fields** shows the rest. A field the editor cannot edit is still listed, greyed,
rather than hidden.

**A number's NAME is its grip.** Drag the name left or right and the value
follows, one step per pixel, live - there is nothing to confirm. A vector row
has three of them, one per axis letter. Click into the box instead to type a
number. What a row is measured in comes from the field's own type, and the unit
is printed beside the box: `m`, `m/s` or `m/s2` for a length, a speed or an
acceleration, and `cells` for the build-grid sizes a section's own mesh is drawn
in. Nothing is converted on the way in or out - the file already holds the
number the box shows, so a builder types what the HUD would read.

The two gestures are not the same rule. A drag ARRIVES at a field's floor and
stops there - dragging a radius past zero is asking for the smallest value there
is. A typed number below the floor is REFUSED, because typing `-3` into a radius
is a mistake. A drag that runs out of screen wraps the pointer to the other
side, so a 0.05-per-pixel field is still reachable on a narrow window.

**Ship Skin**, in the strip's **Ship** tab, dresses the build in
the cladding the ship would fly with. Nothing places a plate: the skin is derived
from the structure you have assembled, and it is re-derived as you build -
including around the part you are still holding, so a hull is dragged about UNDER
the skin and the plating closes over it before you click. A placement the editor
refuses stays bare. The toggle carries through to Play, so the ship you built
clad is the ship you fly clad.

**Station**, a checkbox on a ship's inspector, spawns that hull as a station:
parked, docked with, and offering Rebuild, Refit, Trade and Storage. The hull
needs a docking port and a controller, and no thruster. Under the box, the
**Station Check** row reads `ready`, or the first fault, with all of them in
its hint. Play refuses a marked hull that fails and names it and its faults on
the status line. The ship you fly cannot be a station. Driver **AI** spawns a
living station, and **None** spawns a dead one. Save writes the hull as a
station, and Open reads it back as a marked ship.

### Generate a hull

**Generate**, in the same tab, rolls a whole hull into the ship you
are inside. It is a ship verb: a hull is what one ship IS, so what the roll
produces REPLACES the sections that ship is already holding. That is what makes
a seed a dial you turn until you like the answer rather than a way to fill a
scenario with drafts.

Three controls and a list:

- **Hull Seed** is the number the roll is built from. It starts random, so the
  first press is a ship rather than everyone's ship zero, and **Reroll Seed**
  hands you a fresh one. A hull you liked is a number you can write down and
  type back in. Anything that is not a whole positive number is refused in the
  field - `Enter an unsigned integer` - and greys the **Generate** button with
  it, because a button that cannot work should not look like it can.
- **HULL PLAN** reads back what your ticks add up to, stern and bow. The roles
  are derived, not chosen: the biggest ticked drive becomes the ship's main
  engine and the biggest ticked spinal gun its bow gun, and the engine is what
  decides how big the ship comes out.
- **DRAW FROM** lists every section the merged catalog holds, a mod's included.
  Only the ticked rows are drawn, at the weights the standard hull plan already
  prices them at; a section that plan does not name joins the draw at `1`. The
  chip on a ticked row says where on the hull that part may stand, and the zone
  you set on the row beats the plan's own.

The order is collapse, lint, then place. A roll that fails - or a hull the
game's own content lint would reject - writes one line to the status bar and
leaves the document exactly as it was, so the ship on the stage is never left
holding something that will not save. Generate outside the Ship editor, or with
nothing ticked, and you get the same treatment: `open a ship to generate a hull
into it`, `tick at least one section for the collapse to draw`.

The ship's current **Ship Skin** and style go INTO the collapse, so rolling the
seed asks for a different hull under the same ship rather than the cladding back
at its factory setting. What lands is ordinary section nodes - selectable,
movable, deletable - and each one arrives already bound to the flight key its
kind answers to, so a generated ship can be set to the player's and flown as it
is. The status bar names the seed and the part count it built.

Button glyphs from [PromptFont](https://shinmera.com/promptfont/) by Yukari "Shinmera" Hafner (SIL Open Font License).
