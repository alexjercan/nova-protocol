# Keybinds

The full control reference, keyboard and gamepad side by side. Thrust is analog on the gamepad and on/off on the keyboard. The autopilot verbs each fly a whole maneuver and hand control back; any manual input disengages them immediately. New players should start with [Your first flight](../getting-started/), which teaches these one at a time. Every one of these is REBINDABLE in-game under **Settings > Controls**, reachable from both the main menu and the pause menu - one binding group at a time, keyboard and gamepad in their own columns. Pressing a key another action in the same live set already holds is refused by name, and `Reset Defaults` puts the whole table back.

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
            <td>STOP - face retrograde and burn to rest</td>
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
            <td>RCS fine translation (when a scenario grants it)</td>
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
    </tbody>
</table>

RCS is a docking aid, not standard flight: it appears only when a scenario grants it (the RCS chip appears in the keybind dock only when it is available), and the mainline campaign flies without it. See [Flight & autopilot](../flight-autopilot/#rcs-fine-docking-thrusters).

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
                <span
                    class="pf pf-mouse-scroll"
                    role="img"
                    aria-label="Scroll wheel"
                ></span>
                / <kbd>[</kbd> <kbd>]</kbd>
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
            <td>-</td>
        </tr>
        <tr>
            <td>Launch torpedo</td>
            <td>
                <span
                    class="pf pf-mouse-1"
                    role="img"
                    aria-label="Left mouse button"
                ></span>
                (default)
            </td>
            <td>-</td>
        </tr>
    </tbody>
</table>

Weapon sections are rebindable: in the editor, click a section to bind it to any key or mouse button, so a ship's turret and torpedo controls are whatever its build assigns. The shipped ship fires both turrets and torpedoes on the left mouse button; a torpedo only launches while you hold a raised (red) combat lock. A [railgun](../sections/railgun/) you bolt on binds the same way, and its key is a tap that commits the charge rather than a burst you hold.

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
                <a href="../nova-os/">NOVA OS</a> ship-computer terminal
                (commands; pauses, frees the cursor)
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
            <td>
                <a href="../commands/">Command shell</a> (inspect, settings,
                armed cheats; works with no ship, and over the menus)
            </td>
            <td><kbd>:</kbd></td>
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
            <td>Back to editor (Sandbox only)</td>
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
interface. The bottom-left legend shows the gestures that apply to whatever you
are holding right now; **View > Key Legend** hides it.

Keys are verbs only while nothing else has the keyboard. With the parts gallery
up, the arrow keys move its grid and <kbd>Esc</kbd> closes the gallery rather
than backing out of the ship; with the caret in a text field, every key is a
character; and while a rebind waits for a key, it takes all of them except
<kbd>Esc</kbd>. <kbd>Ctrl</kbd>+<kbd>S</kbd> answers under all of that except a
field and a pending rebind.

<table class="controls">
    <tr>
        <td>Fly the stage camera</td>
        <td>
            <kbd>W</kbd> <kbd>A</kbd> <kbd>S</kbd> <kbd>D</kbd>,
            <kbd>Space</kbd> / <kbd>Shift</kbd> for up and down,
            right-drag to look
        </td>
    </tr>
    <tr>
        <td>Select a node; enter the ship you selected</td>
        <td>left click; double click</td>
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
        <td>Save the document</td>
        <td><kbd>Ctrl</kbd>+<kbd>S</kbd> &nbsp;(or <b>File &gt; Save</b>)</td>
    </tr>
    <tr>
        <td>Open or close the parts gallery (inside a ship)</td>
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
        <td>Roll it about the mating axis</td>
        <td>wheel &nbsp;or&nbsp; <kbd>R</kbd></td>
    </tr>
    <tr>
        <td>Cycle which of its sockets mates</td>
        <td><kbd>Ctrl</kbd>+wheel &nbsp;or&nbsp; <kbd>F</kbd></td>
    </tr>
    <tr>
        <td>Pick up the part under the cursor</td>
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
        <td>Leave the ship you are inside</td>
        <td><kbd>Bksp</kbd></td>
    </tr>
    <tr>
        <td>Back out one rung</td>
        <td><kbd>Esc</kbd></td>
    </tr>
</table>

<kbd>Esc</kbd> takes one rung per press, in this order: an open top-bar menu,
the search field, the gallery, a pending rebind, the part in hand, the ship you
are inside, and finally the pause menu. <kbd>Bksp</kbd> is the shortcut past all
of that - it leaves the ship from wherever you are, unless a text field has the
caret, where it deletes a character instead.

### The inspector

The right-hand panel shows the selected node's own fields, read off the thing
itself rather than from a list the editor keeps - so a part that grows a setting
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

**Ship Skin**, in the left rail's **Ship Settings** block, dresses the build in
the cladding the ship would fly with. Nothing places a plate: the skin is derived
from the structure you have assembled, and it is re-derived as you build -
including around the part you are still holding, so a hull is dragged about UNDER
the skin and the plating closes over it before you click. A placement the editor
refuses stays bare. The toggle carries through to Play, so the ship you built
clad is the ship you fly clad.

Button glyphs from [PromptFont](https://shinmera.com/promptfont/) by Yukari "Shinmera" Hafner (SIL Open Font License).
